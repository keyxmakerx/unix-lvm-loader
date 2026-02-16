<script>
  import { onMount } from 'svelte';
  import {
    getClevisStatus,
    verifyTangServer,
    listClevisBindings,
    bindTang,
    unbindClevis,
    getClevisInstallInstructions,
    scanLuksVolumes,
  } from '../utils/api.js';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import Toast from './Toast.svelte';
  import { formatError } from '../utils/errors.js';

  let status = $state(null);
  let volumes = $state([]);
  let selectedDevice = $state('');
  let bindings = $state([]);
  let loading = $state(false);
  let installInstructions = $state('');

  // Tang wizard state
  let showTangWizard = $state(false);
  let tangStep = $state(1);
  let tangUrl = $state('');
  let tangVerification = $state(null);
  let tangVerifying = $state(false);
  let tangBinding = $state(false);

  let confirmDialog;
  let toast;

  onMount(async () => {
    await loadStatus();
    await loadVolumes();
  });

  async function loadStatus() {
    try {
      status = await getClevisStatus();
      if (!status.clevis_installed) {
        installInstructions = await getClevisInstallInstructions();
      }
    } catch (e) {
      console.error('Failed to load Clevis status:', e);
    }
  }

  async function loadVolumes() {
    try {
      volumes = await scanLuksVolumes();
      if (volumes.length > 0 && !selectedDevice) {
        selectedDevice = volumes[0];
        await loadBindings();
      }
    } catch (e) {
      console.error('Failed to load volumes:', e);
    }
  }

  async function loadBindings() {
    if (!selectedDevice || !status?.clevis_installed) return;
    loading = true;
    try {
      bindings = await listClevisBindings(selectedDevice);
    } catch (e) {
      bindings = [];
    }
    loading = false;
  }

  async function handleDeviceChange() {
    bindings = [];
    await loadBindings();
  }

  // ── Tang Wizard ──

  function startTangWizard() {
    tangStep = 1;
    tangUrl = '';
    tangVerification = null;
    showTangWizard = true;
  }

  async function verifyTang() {
    if (!tangUrl.trim()) return;
    tangVerifying = true;
    try {
      tangVerification = await verifyTangServer(tangUrl.trim());
    } catch (e) {
      tangVerification = { reachable: false, url: tangUrl };
      const err = formatError(e);
      toast?.show(`${err.title}: ${err.message}`, 'error');
    }
    tangVerifying = false;
  }

  async function executeTangBind() {
    confirmDialog?.open({
      title: 'Bind to Tang Server',
      message: `This will bind ${selectedDevice} to the Tang server at ${tangUrl}. The device will automatically unlock when connected to the network where this server is reachable. A LUKS header backup will be created first.`,
      level: 'warning',
      confirmLabel: 'Bind Device',
      onConfirm: async () => {
        tangBinding = true;
        try {
          await bindTang(
            selectedDevice,
            tangUrl.trim(),
            tangVerification?.thumbprint || null
          );
          toast?.show('Tang binding successful!', 'success');
          showTangWizard = false;
          await loadBindings();
        } catch (e) {
          const err = formatError(e);
          toast?.show(`${err.title}: ${err.message}`, 'error');
        }
        tangBinding = false;
      },
    });
  }

  async function handleUnbind(slot) {
    const binding = bindings.find((b) => b.slot === slot);
    confirmDialog?.open({
      title: 'Remove Clevis Binding',
      message: `Remove the ${binding?.pin || 'Clevis'} binding from slot ${slot} on ${selectedDevice}? A LUKS header backup will be created before removal.`,
      level: 'warning',
      confirmLabel: 'Remove Binding',
      onConfirm: async () => {
        try {
          await unbindClevis(selectedDevice, slot);
          toast?.show(`Binding removed from slot ${slot}`, 'success');
          await loadBindings();
        } catch (e) {
          const err = formatError(e);
          toast?.show(`${err.title}: ${err.message}`, 'error');
        }
      },
    });
  }
</script>

<ConfirmDialog bind:this={confirmDialog} />
<Toast bind:this={toast} />

<div class="p-6 max-w-4xl">
  <h2 class="text-2xl font-bold text-text-primary mb-1">Network Unlock</h2>
  <p class="text-sm text-text-muted mb-6">
    Clevis/Tang network-bound disk encryption — auto-unlock when on a trusted network
  </p>

  <!-- Status Banner -->
  {#if status && !status.clevis_installed}
    <div class="bg-warning/10 border border-warning/30 rounded-lg p-4 mb-6">
      <h3 class="text-warning font-semibold mb-1">Clevis Not Installed</h3>
      <p class="text-sm text-text-secondary mb-2">
        Clevis is required for network-bound disk encryption. Install it with:
      </p>
      <code class="block bg-surface-0 rounded px-3 py-2 text-sm text-accent font-mono">
        {installInstructions}
      </code>
    </div>
  {:else if status}
    <div class="bg-success/10 border border-success/30 rounded-lg p-4 mb-6 flex items-center gap-3">
      <span class="text-success text-xl">&#10003;</span>
      <div>
        <span class="text-text-primary font-medium">Clevis is installed</span>
        {#if status.clevis_version}
          <span class="text-text-muted text-sm ml-2">({status.clevis_version})</span>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Device Selector -->
  {#if status?.clevis_installed && volumes.length > 0}
    <div class="mb-6">
      <label for="clevis-device-select" class="block text-sm text-text-secondary mb-1.5">LUKS Device</label>
      <select
        id="clevis-device-select"
        class="w-full bg-surface-1 border border-border rounded-lg px-3 py-2 text-text-primary"
        bind:value={selectedDevice}
        onchange={handleDeviceChange}
      >
        {#each volumes as vol}
          <option value={vol}>{vol}</option>
        {/each}
      </select>
    </div>

    <!-- Current Bindings -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-lg font-semibold text-text-primary">Current Bindings</h3>
        <button
          class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/80 transition-colors text-sm"
          onclick={startTangWizard}
        >
          + Bind Tang Server
        </button>
      </div>

      {#if loading}
        <div class="bg-surface-1 rounded-lg p-6 text-center text-text-muted">
          Scanning bindings...
        </div>
      {:else if bindings.length === 0}
        <div class="bg-surface-1 rounded-lg p-6 text-center">
          <p class="text-text-muted mb-2">No Clevis bindings on this device</p>
          <p class="text-xs text-text-muted">
            Bind to a Tang server to enable automatic network-based unlock at boot
          </p>
        </div>
      {:else}
        <div class="space-y-2">
          {#each bindings as binding}
            <div class="bg-surface-1 rounded-lg p-4 flex items-center justify-between">
              <div class="flex items-center gap-3">
                <span class="w-8 h-8 flex items-center justify-center rounded-md text-sm font-mono
                  {binding.pin === 'tang' ? 'bg-accent/20 text-accent' : 'bg-surface-3 text-text-secondary'}">
                  {binding.slot}
                </span>
                <div>
                  <div class="flex items-center gap-2">
                    <span class="text-text-primary font-medium capitalize">{binding.pin}</span>
                    <span class="px-1.5 py-0.5 rounded text-xs bg-surface-2 text-text-muted">
                      Slot {binding.slot}
                    </span>
                  </div>
                  {#if binding.server_url}
                    <p class="text-xs text-text-muted mt-0.5">{binding.server_url}</p>
                  {:else}
                    <p class="text-xs text-text-muted mt-0.5 font-mono truncate max-w-md">
                      {binding.config}
                    </p>
                  {/if}
                </div>
              </div>
              <button
                class="px-3 py-1.5 text-sm text-danger/80 hover:text-danger hover:bg-danger/10 rounded transition-colors"
                onclick={() => handleUnbind(binding.slot)}
              >
                Remove
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else if status?.clevis_installed}
    <div class="bg-surface-1 rounded-lg p-6 text-center text-text-muted">
      No LUKS volumes detected
    </div>
  {/if}

  <!-- Tang Wizard Modal -->
  {#if showTangWizard}
    <div class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center">
      <div class="bg-surface-1 rounded-xl shadow-xl w-full max-w-lg mx-4">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-border">
          <h3 class="text-lg font-semibold text-text-primary">Bind Tang Server</h3>
          <button
            class="text-text-muted hover:text-text-primary transition-colors"
            onclick={() => (showTangWizard = false)}
          >&#10005;</button>
        </div>

        <!-- Step indicator -->
        <div class="flex items-center gap-2 px-6 pt-4">
          {#each [1, 2, 3] as step}
            <div class="flex items-center gap-2 {step <= tangStep ? 'text-accent' : 'text-text-muted'}">
              <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium
                {step === tangStep ? 'bg-accent text-white' : step < tangStep ? 'bg-accent/20 text-accent' : 'bg-surface-2 text-text-muted'}">
                {step < tangStep ? '&#10003;' : step}
              </div>
              <span class="text-xs hidden sm:inline">
                {step === 1 ? 'Server URL' : step === 2 ? 'Verify' : 'Confirm'}
              </span>
            </div>
            {#if step < 3}
              <div class="flex-1 h-px {step < tangStep ? 'bg-accent' : 'bg-border'}"></div>
            {/if}
          {/each}
        </div>

        <!-- Step content -->
        <div class="px-6 py-4 min-h-[200px]">
          {#if tangStep === 1}
            <p class="text-sm text-text-secondary mb-4">
              Enter the URL of your Tang server. The device will auto-unlock when this server is reachable on the network.
            </p>
            <label for="tang-url-input" class="block text-sm text-text-secondary mb-1.5">Tang Server URL</label>
            <input
              id="tang-url-input"
              type="url"
              class="w-full bg-surface-0 border border-border rounded-lg px-3 py-2 text-text-primary placeholder-text-muted"
              placeholder="http://tang.example.com:7500"
              bind:value={tangUrl}
            />
            <p class="text-xs text-text-muted mt-2">
              Common default port is 7500. The server must be running the Tang daemon.
            </p>
          {:else if tangStep === 2}
            {#if tangVerifying}
              <div class="flex items-center gap-3 py-8 justify-center">
                <div class="w-5 h-5 border-2 border-accent border-t-transparent rounded-full animate-spin"></div>
                <span class="text-text-secondary">Verifying server...</span>
              </div>
            {:else if tangVerification}
              {#if tangVerification.reachable}
                <div class="bg-success/10 border border-success/30 rounded-lg p-4 mb-4">
                  <div class="flex items-center gap-2 mb-2">
                    <span class="text-success">&#10003;</span>
                    <span class="text-success font-medium">Server is reachable</span>
                  </div>
                  {#if tangVerification.thumbprint}
                    <div class="mt-2">
                      <p class="text-xs text-text-secondary mb-1">Server thumbprint:</p>
                      <code class="block bg-surface-0 rounded px-3 py-2 text-sm font-mono text-accent break-all">
                        {tangVerification.thumbprint}
                      </code>
                    </div>
                  {/if}
                </div>
                <p class="text-sm text-text-secondary">
                  Verify this thumbprint matches what your Tang server administrator provides.
                  This ensures you are connecting to the correct server.
                </p>
              {:else}
                <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 mb-4">
                  <div class="flex items-center gap-2">
                    <span class="text-danger">&#10007;</span>
                    <span class="text-danger font-medium">Server unreachable</span>
                  </div>
                  <p class="text-sm text-text-muted mt-2">
                    Could not connect to {tangVerification.url}. Check the URL and ensure the Tang server is running.
                  </p>
                </div>
              {/if}
            {:else}
              <div class="text-center py-8">
                <p class="text-text-secondary mb-4">
                  We'll verify the Tang server at <strong class="text-text-primary">{tangUrl}</strong> is reachable
                  and show you its thumbprint for verification.
                </p>
                <button
                  class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/80 transition-colors"
                  onclick={verifyTang}
                >
                  Verify Server
                </button>
              </div>
            {/if}
          {:else if tangStep === 3}
            <div class="space-y-4">
              <div class="bg-surface-0 rounded-lg p-4">
                <h4 class="text-sm font-medium text-text-secondary mb-2">Summary</h4>
                <div class="space-y-2 text-sm">
                  <div class="flex justify-between">
                    <span class="text-text-muted">Device:</span>
                    <span class="text-text-primary font-mono">{selectedDevice}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-text-muted">Tang Server:</span>
                    <span class="text-text-primary">{tangUrl}</span>
                  </div>
                  {#if tangVerification?.thumbprint}
                    <div class="flex justify-between">
                      <span class="text-text-muted">Thumbprint:</span>
                      <span class="text-accent font-mono text-xs">{tangVerification.thumbprint}</span>
                    </div>
                  {/if}
                </div>
              </div>
              <div class="bg-warning/10 border border-warning/30 rounded-lg p-3">
                <p class="text-sm text-text-secondary">
                  <strong class="text-warning">Safety:</strong> A LUKS header backup will be created automatically before binding.
                </p>
              </div>
            </div>
          {/if}
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-between px-6 py-4 border-t border-border">
          <button
            class="px-4 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
            onclick={() => {
              if (tangStep === 1) showTangWizard = false;
              else tangStep--;
            }}
          >
            {tangStep === 1 ? 'Cancel' : 'Back'}
          </button>

          {#if tangStep === 1}
            <button
              class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/80 transition-colors text-sm disabled:opacity-50"
              disabled={!tangUrl.trim()}
              onclick={() => { tangStep = 2; verifyTang(); }}
            >
              Next
            </button>
          {:else if tangStep === 2}
            <button
              class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/80 transition-colors text-sm disabled:opacity-50"
              disabled={!tangVerification?.reachable}
              onclick={() => (tangStep = 3)}
            >
              Next
            </button>
          {:else}
            <button
              class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/80 transition-colors text-sm disabled:opacity-50"
              disabled={tangBinding}
              onclick={executeTangBind}
            >
              {tangBinding ? 'Binding...' : 'Bind Device'}
            </button>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
