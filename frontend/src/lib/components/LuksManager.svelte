<script>
  import { onMount } from 'svelte';
  import {
    scanLuksVolumes,
    getLuksInfo,
    checkCryptenrollAvailable,
    enrollTpm2,
    enrollFido2,
    enrollRecoveryKey,
    backupLuksHeader,
  } from '../utils/api.js';

  let volumes = $state([]);
  let selectedDevice = $state(null);
  let luksInfo = $state(null);
  let hasCryptenroll = $state(false);
  let loading = $state(true);
  let loadingInfo = $state(false);
  let error = $state(null);
  let actionStatus = $state(null); // { type: 'success'|'error', message: string }
  let actionLoading = $state(null); // 'tpm2' | 'fido2' | 'recovery' | 'backup'

  onMount(async () => {
    try {
      [volumes, hasCryptenroll] = await Promise.all([
        scanLuksVolumes(),
        checkCryptenrollAvailable(),
      ]);
      if (volumes.length > 0) {
        await selectDevice(volumes[0]);
      }
    } catch (e) {
      error = e?.message || String(e);
    } finally {
      loading = false;
    }
  });

  async function selectDevice(device) {
    selectedDevice = device;
    loadingInfo = true;
    luksInfo = null;
    try {
      luksInfo = await getLuksInfo(device);
    } catch (e) {
      error = e?.message || String(e);
    } finally {
      loadingInfo = false;
    }
  }

  async function handleAction(action, label) {
    actionLoading = action;
    actionStatus = null;
    try {
      let result;
      switch (action) {
        case 'tpm2':
          await enrollTpm2(selectedDevice);
          actionStatus = { type: 'success', message: 'TPM2 enrolled successfully. A LUKS header backup was created automatically.' };
          break;
        case 'fido2':
          await enrollFido2(selectedDevice);
          actionStatus = { type: 'success', message: 'FIDO2 enrolled successfully. A LUKS header backup was created automatically.' };
          break;
        case 'recovery':
          result = await enrollRecoveryKey(selectedDevice);
          actionStatus = { type: 'success', message: `Recovery key enrolled. SAVE THIS KEY:\n${result}` };
          break;
        case 'backup':
          await backupLuksHeader(selectedDevice);
          actionStatus = { type: 'success', message: 'LUKS header backup created and verified.' };
          break;
      }
      // Refresh info
      luksInfo = await getLuksInfo(selectedDevice);
    } catch (e) {
      actionStatus = { type: 'error', message: e?.message || String(e) };
    } finally {
      actionLoading = null;
    }
  }
</script>

<div class="p-6 space-y-6">
  <h2 class="text-2xl font-bold text-text-primary">LUKS Manager</h2>

  {#if loading}
    <div class="flex items-center gap-3 text-text-secondary">
      <div class="animate-spin w-5 h-5 border-2 border-accent border-t-transparent rounded-full"></div>
      Scanning LUKS volumes...
    </div>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger">
      <p class="font-semibold">Error</p>
      <p class="text-sm mt-1">{error}</p>
    </div>
  {:else if volumes.length === 0}
    <div class="bg-surface-1 border border-border rounded-lg p-8 text-center">
      <p class="text-text-muted text-lg">No LUKS volumes found</p>
      <p class="text-text-muted text-sm mt-2">This system does not appear to have encrypted volumes.</p>
    </div>
  {:else}
    <!-- Volume selector -->
    <div class="flex gap-2">
      {#each volumes as vol}
        <button
          class="px-4 py-2 rounded-lg text-sm font-mono transition-colors {selectedDevice === vol
            ? 'bg-accent text-white'
            : 'bg-surface-2 text-text-secondary hover:bg-surface-3'}"
          onclick={() => selectDevice(vol)}
        >{vol}</button>
      {/each}
    </div>

    {#if loadingInfo}
      <div class="flex items-center gap-3 text-text-secondary">
        <div class="animate-spin w-5 h-5 border-2 border-accent border-t-transparent rounded-full"></div>
        Loading volume info...
      </div>
    {:else if luksInfo}
      <!-- Volume Info -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <h3 class="font-bold text-text-primary mb-3">Volume Information</h3>
        <div class="grid grid-cols-2 gap-3 text-sm">
          <div><span class="text-text-muted">Device:</span> <span class="text-text-primary font-mono">{luksInfo.device}</span></div>
          <div><span class="text-text-muted">UUID:</span> <span class="text-text-primary font-mono text-xs">{luksInfo.uuid}</span></div>
          <div><span class="text-text-muted">Version:</span> <span class="text-text-primary font-bold">{luksInfo.version === 'Luks2' ? 'LUKS2' : luksInfo.version === 'Luks1' ? 'LUKS1' : 'Unknown'}</span></div>
          <div><span class="text-text-muted">Cipher:</span> <span class="text-text-primary font-mono">{luksInfo.cipher}</span></div>
          <div><span class="text-text-muted">Key Size:</span> <span class="text-text-primary">{luksInfo.key_size_bits} bits</span></div>
          <div><span class="text-text-muted">Hash:</span> <span class="text-text-primary font-mono">{luksInfo.hash}</span></div>
          {#if luksInfo.label}
            <div><span class="text-text-muted">Label:</span> <span class="text-text-primary">{luksInfo.label}</span></div>
          {/if}
        </div>
      </div>

      <!-- Key Slots -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <h3 class="font-bold text-text-primary mb-3">
          Key Slots
          <span class="text-sm font-normal text-text-muted">({luksInfo.active_passphrase_slots} / {luksInfo.total_slots} active)</span>
        </h3>
        <div class="grid grid-cols-4 md:grid-cols-8 gap-2">
          {#each Array(Math.min(luksInfo.total_slots, 32)) as _, i}
            {@const slot = luksInfo.key_slots.find(s => s.slot_number === i)}
            <div
              class="rounded-lg p-2 text-center text-xs border {slot?.enabled
                ? 'bg-success/10 border-success/30 text-success'
                : 'bg-surface-2 border-border text-text-muted'}"
              title={slot?.enabled ? `Slot ${i}: ${slot.key_type}${slot.kdf ? ` (${slot.kdf})` : ''}` : `Slot ${i}: empty`}
            >
              <div class="font-bold">{i}</div>
              <div class="text-[10px] truncate">{slot?.enabled ? slot.key_type : '-'}</div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Tokens (LUKS2 only) -->
      {#if luksInfo.tokens.length > 0}
        <div class="bg-surface-1 border border-border rounded-lg p-5">
          <h3 class="font-bold text-text-primary mb-3">Tokens</h3>
          <div class="space-y-2">
            {#each luksInfo.tokens as token}
              <div class="flex items-center gap-3 p-2 bg-surface-2 rounded-lg text-sm">
                <span class="text-text-muted">#{token.token_id}</span>
                <span class="font-mono text-accent">{token.token_type}</span>
                {#if token.keyslots.length > 0}
                  <span class="text-text-muted text-xs">Keyslots: {token.keyslots.join(', ')}</span>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Actions -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <h3 class="font-bold text-text-primary mb-3">Actions</h3>
        <p class="text-sm text-text-muted mb-4">
          All operations automatically create a LUKS header backup before executing.
          This ensures you can always recover if something goes wrong.
        </p>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
          <button
            class="p-3 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-center transition-colors disabled:opacity-50"
            onclick={() => handleAction('backup', 'Backup Header')}
            disabled={actionLoading !== null}
          >
            <div class="text-2xl mb-1">&#128190;</div>
            <div class="text-sm font-medium text-text-primary">{actionLoading === 'backup' ? 'Backing up...' : 'Backup Header'}</div>
            <div class="text-[10px] text-text-muted mt-0.5">Save LUKS header</div>
          </button>

          {#if hasCryptenroll && luksInfo.version === 'Luks2'}
            <button
              class="p-3 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-center transition-colors disabled:opacity-50"
              onclick={() => handleAction('tpm2', 'Enroll TPM2')}
              disabled={actionLoading !== null}
            >
              <div class="text-2xl mb-1">&#128272;</div>
              <div class="text-sm font-medium text-text-primary">{actionLoading === 'tpm2' ? 'Enrolling...' : 'Enroll TPM2'}</div>
              <div class="text-[10px] text-text-muted mt-0.5">Hardware security</div>
            </button>

            <button
              class="p-3 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-center transition-colors disabled:opacity-50"
              onclick={() => handleAction('fido2', 'Enroll FIDO2')}
              disabled={actionLoading !== null}
            >
              <div class="text-2xl mb-1">&#128273;</div>
              <div class="text-sm font-medium text-text-primary">{actionLoading === 'fido2' ? 'Enrolling...' : 'Enroll FIDO2'}</div>
              <div class="text-[10px] text-text-muted mt-0.5">Security key</div>
            </button>

            <button
              class="p-3 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-center transition-colors disabled:opacity-50"
              onclick={() => handleAction('recovery', 'Recovery Key')}
              disabled={actionLoading !== null}
            >
              <div class="text-2xl mb-1">&#128221;</div>
              <div class="text-sm font-medium text-text-primary">{actionLoading === 'recovery' ? 'Generating...' : 'Recovery Key'}</div>
              <div class="text-[10px] text-text-muted mt-0.5">Emergency access</div>
            </button>
          {:else if !hasCryptenroll}
            <div class="col-span-3 p-3 bg-warning/10 border border-warning/30 rounded-lg text-sm text-warning">
              systemd-cryptenroll not available. Install it for TPM2/FIDO2/Recovery key features.
            </div>
          {:else if luksInfo.version !== 'Luks2'}
            <div class="col-span-3 p-3 bg-warning/10 border border-warning/30 rounded-lg text-sm text-warning">
              LUKS2 required for TPM2/FIDO2 enrollment. This volume uses {luksInfo.version}.
            </div>
          {/if}
        </div>
      </div>

      <!-- Action Status -->
      {#if actionStatus}
        <div class="rounded-lg p-4 {actionStatus.type === 'success'
          ? 'bg-success/10 border border-success/30 text-success'
          : 'bg-danger/10 border border-danger/30 text-danger'}">
          <pre class="text-sm whitespace-pre-wrap">{actionStatus.message}</pre>
        </div>
      {/if}
    {/if}
  {/if}
</div>
