<script>
  import { onMount } from 'svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import {
    scanLuksVolumes,
    getLuksInfo,
    checkCryptenrollAvailable,
    enrollTpm2,
    enrollFido2,
    enrollRecoveryKey,
    backupLuksHeader,
    restoreLuksHeader,
    getInitramfsInfo,
    rebuildInitramfs,
  } from '../utils/api.js';
  import { formatError } from '../utils/errors.js';

  let volumes = $state([]);
  let selectedDevice = $state(null);
  let luksInfo = $state(null);
  let hasCryptenroll = $state(false);
  let loading = $state(true);
  let loadingInfo = $state(false);
  let error = $state(null);
  let actionStatus = $state(null);
  let actionLoading = $state(null);

  // Initramfs
  let showRebuildPrompt = $state(false);
  let rebuildingInitramfs = $state(false);
  let rebuildResult = $state(null);

  // Confirmation dialog state
  let confirmOpen = $state(false);
  let confirmLevel = $state('warning');
  let confirmTitle = $state('');
  let confirmMessage = $state('');
  let confirmDetails = $state(null);
  let confirmPhrase = $state('');
  let confirmLabel = $state('Confirm');
  let pendingAction = $state(null);

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
      const err = formatError(e);
      error = err;
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
      const err = formatError(e);
      error = err;
    } finally {
      loadingInfo = false;
    }
  }

  // Show confirmation dialog before action
  function requestAction(action) {
    pendingAction = action;

    switch (action) {
      case 'backup':
        // Backup is safe — just info level
        confirmLevel = 'info';
        confirmTitle = 'Backup LUKS Header';
        confirmMessage = `Create a backup of the LUKS header for ${selectedDevice}. This is a non-destructive read-only operation.`;
        confirmDetails = `Device: ${selectedDevice}\nUUID: ${luksInfo?.uuid || 'unknown'}`;
        confirmPhrase = '';
        confirmLabel = 'Create Backup';
        break;

      case 'tpm2':
        confirmLevel = 'warning';
        confirmTitle = 'Enroll TPM2 Device';
        confirmMessage = `This will enroll your TPM2 hardware security chip into the LUKS2 volume on ${selectedDevice}. A header backup will be created automatically before enrollment.`;
        confirmDetails = `Device: ${selectedDevice}\nUUID: ${luksInfo?.uuid || 'unknown'}\n\nAfter enrollment:\n- The system can auto-unlock at boot using the TPM2 chip\n- Your existing passphrase will still work as a fallback\n- You must rebuild initramfs for auto-unlock to take effect`;
        confirmPhrase = '';
        confirmLabel = 'Enroll TPM2';
        break;

      case 'fido2':
        confirmLevel = 'warning';
        confirmTitle = 'Enroll FIDO2 Security Key';
        confirmMessage = `This will enroll a FIDO2 security key (e.g., YubiKey) into the LUKS2 volume on ${selectedDevice}. A header backup will be created automatically. Have your security key ready — you may need to touch it during enrollment.`;
        confirmDetails = `Device: ${selectedDevice}\nUUID: ${luksInfo?.uuid || 'unknown'}`;
        confirmPhrase = '';
        confirmLabel = 'Enroll FIDO2';
        break;

      case 'recovery':
        confirmLevel = 'warning';
        confirmTitle = 'Generate Recovery Key';
        confirmMessage = `This will generate a recovery key and enroll it into the LUKS2 volume on ${selectedDevice}. IMPORTANT: You must save the recovery key securely — it is your emergency access if all other keys fail. A header backup will be created automatically.`;
        confirmDetails = `Device: ${selectedDevice}\nUUID: ${luksInfo?.uuid || 'unknown'}`;
        confirmPhrase = '';
        confirmLabel = 'Generate Recovery Key';
        break;

      default:
        return;
    }

    confirmOpen = true;
  }

  async function executeAction() {
    const action = pendingAction;
    if (!action) return;
    pendingAction = null;

    actionLoading = action;
    actionStatus = null;
    try {
      let result;
      switch (action) {
        case 'tpm2':
          await enrollTpm2(selectedDevice);
          actionStatus = { type: 'success', message: 'TPM2 enrolled successfully. A LUKS header backup was created automatically.' };
          showRebuildPrompt = true;
          break;
        case 'fido2':
          await enrollFido2(selectedDevice);
          actionStatus = { type: 'success', message: 'FIDO2 enrolled successfully. A LUKS header backup was created automatically.' };
          showRebuildPrompt = true;
          break;
        case 'recovery':
          result = await enrollRecoveryKey(selectedDevice);
          actionStatus = { type: 'success', message: `Recovery key enrolled. A LUKS header backup was created automatically.\n\nSAVE THIS KEY SECURELY — print it or store it offline:\n${result}` };
          break;
        case 'backup':
          await backupLuksHeader(selectedDevice);
          actionStatus = { type: 'success', message: 'LUKS header backup created and verified with SHA-256 checksum.' };
          break;
      }
      // Refresh info
      luksInfo = await getLuksInfo(selectedDevice);
    } catch (e) {
      const err = formatError(e);
      actionStatus = { type: 'error', title: err.title, message: err.message, hint: err.hint };
    } finally {
      actionLoading = null;
    }
  }
</script>

<!-- Confirmation Dialog -->
<ConfirmDialog
  bind:open={confirmOpen}
  level={confirmLevel}
  title={confirmTitle}
  message={confirmMessage}
  details={confirmDetails}
  confirmPhrase={confirmPhrase}
  confirmLabel={confirmLabel}
  onconfirm={executeAction}
/>

<div class="p-6 space-y-6">
  <h2 class="text-2xl font-bold text-text-primary">LUKS Manager</h2>

  {#if loading}
    <div class="flex items-center gap-3 text-text-secondary">
      <div class="animate-spin w-5 h-5 border-2 border-accent border-t-transparent rounded-full"></div>
      Scanning LUKS volumes...
    </div>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger">
      <p class="font-semibold">{error.title || 'Error'}</p>
      <p class="text-sm mt-1">{error.message}</p>
      {#if error.hint}
        <p class="text-xs mt-2 opacity-80">{error.hint}</p>
      {/if}
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
          You will be asked to confirm before any changes are made.
        </p>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
          <button
            class="p-3 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-center transition-colors disabled:opacity-50"
            onclick={() => requestAction('backup')}
            disabled={actionLoading !== null}
          >
            <div class="text-2xl mb-1">&#128190;</div>
            <div class="text-sm font-medium text-text-primary">{actionLoading === 'backup' ? 'Backing up...' : 'Backup Header'}</div>
            <div class="text-[10px] text-text-muted mt-0.5">Save LUKS header</div>
          </button>

          {#if hasCryptenroll && luksInfo.version === 'Luks2'}
            <button
              class="p-3 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-center transition-colors disabled:opacity-50"
              onclick={() => requestAction('tpm2')}
              disabled={actionLoading !== null}
            >
              <div class="text-2xl mb-1">&#128272;</div>
              <div class="text-sm font-medium text-text-primary">{actionLoading === 'tpm2' ? 'Enrolling...' : 'Enroll TPM2'}</div>
              <div class="text-[10px] text-text-muted mt-0.5">Hardware security</div>
            </button>

            <button
              class="p-3 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-center transition-colors disabled:opacity-50"
              onclick={() => requestAction('fido2')}
              disabled={actionLoading !== null}
            >
              <div class="text-2xl mb-1">&#128273;</div>
              <div class="text-sm font-medium text-text-primary">{actionLoading === 'fido2' ? 'Enrolling...' : 'Enroll FIDO2'}</div>
              <div class="text-[10px] text-text-muted mt-0.5">Security key</div>
            </button>

            <button
              class="p-3 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-center transition-colors disabled:opacity-50"
              onclick={() => requestAction('recovery')}
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
          {#if actionStatus.title && actionStatus.type === 'error'}
            <p class="font-semibold">{actionStatus.title}</p>
          {/if}
          <pre class="text-sm whitespace-pre-wrap mt-1">{actionStatus.message}</pre>
          {#if actionStatus.hint}
            <p class="text-xs mt-2 opacity-80">{actionStatus.hint}</p>
          {/if}
        </div>
      {/if}

      <!-- Initramfs Rebuild Prompt (shown after TPM2/FIDO2 enrollment) -->
      {#if showRebuildPrompt}
        <div class="bg-warning/10 border border-warning/30 rounded-lg p-5">
          <div class="flex items-center gap-3 mb-3">
            <span class="text-warning text-xl">&#9888;</span>
            <h3 class="font-bold text-warning">Initramfs Rebuild Required</h3>
          </div>
          <p class="text-sm text-text-secondary mb-4">
            For the newly enrolled key to work at boot, the initramfs (initial RAM filesystem)
            must be rebuilt to include the necessary drivers and configuration. The current
            initramfs will be backed up automatically before rebuilding.
          </p>

          {#if rebuildResult}
            <div class="mb-4 rounded-lg p-3 text-sm {rebuildResult.success
              ? 'bg-success/10 border border-success/30 text-success'
              : 'bg-danger/10 border border-danger/30 text-danger'}">
              <p class="font-medium">{rebuildResult.success ? 'Rebuild successful' : 'Rebuild failed'}</p>
              <p class="text-xs mt-1 font-mono">Command: {rebuildResult.command_run}</p>
              {#if !rebuildResult.success && rebuildResult.stderr}
                <pre class="text-xs mt-2 whitespace-pre-wrap max-h-32 overflow-y-auto">{rebuildResult.stderr}</pre>
              {/if}
              {#if rebuildResult.backup_id}
                <p class="text-xs mt-1">Previous initramfs backed up (ID: {rebuildResult.backup_id.substring(0, 8)}...)</p>
              {/if}
            </div>
          {/if}

          <div class="flex gap-3">
            <button
              class="px-4 py-2 bg-warning hover:bg-warning/80 text-black rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
              onclick={async () => {
                rebuildingInitramfs = true;
                try {
                  rebuildResult = await rebuildInitramfs();
                  if (rebuildResult.success) {
                    showRebuildPrompt = false;
                  }
                } catch (e) {
                  rebuildResult = { success: false, stderr: e?.message || String(e), command_run: 'unknown', stdout: '', initramfs_system: 'unknown', backup_id: null };
                } finally {
                  rebuildingInitramfs = false;
                }
              }}
              disabled={rebuildingInitramfs}
            >
              {rebuildingInitramfs ? 'Rebuilding...' : 'Rebuild Initramfs Now'}
            </button>
            <button
              class="px-4 py-2 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-sm text-text-secondary transition-colors"
              onclick={() => { showRebuildPrompt = false; }}
            >
              Skip for Now
            </button>
          </div>
        </div>
      {/if}
    {/if}
  {/if}
</div>
