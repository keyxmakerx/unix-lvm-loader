<script>
  import { onMount } from 'svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import {
    listBackups,
    verifyBackup,
    verifyAllBackups,
    backupCrypttab,
    restoreLuksHeader,
  } from '../utils/api.js';
  import { formatError } from '../utils/errors.js';

  let backups = $state([]);
  let loading = $state(true);
  let error = $state(null);
  let verifying = $state(null);
  let verifyingAll = $state(false);
  let verificationResults = $state(null);

  // Restore confirmation (critical level)
  let restoreConfirmOpen = $state(false);
  let pendingRestore = $state(null);
  let restoring = $state(false);
  let actionStatus = $state(null);
  let filterType = $state('all');

  onMount(async () => {
    await loadBackups();
  });

  async function loadBackups() {
    loading = true;
    try {
      backups = await listBackups(filterType === 'all' ? null : filterType);
    } catch (e) {
      const err = formatError(e);
      error = `${err.title}: ${err.message}`;
    } finally {
      loading = false;
    }
  }

  async function handleVerify(backupId) {
    verifying = backupId;
    try {
      const valid = await verifyBackup(backupId);
      actionStatus = {
        type: valid ? 'success' : 'error',
        message: valid ? 'Backup integrity verified (SHA-256 match)' : 'BACKUP CORRUPTED! SHA-256 mismatch detected.',
      };
    } catch (e) {
      const err = formatError(e);
      actionStatus = { type: 'error', message: `${err.title}: ${err.message}` };
    } finally {
      verifying = null;
    }
  }

  async function handleVerifyAll() {
    verifyingAll = true;
    verificationResults = null;
    try {
      const results = await verifyAllBackups();
      verificationResults = results;
      const failed = results.filter(([_, valid]) => !valid);
      if (failed.length === 0) {
        actionStatus = { type: 'success', message: `All ${results.length} backups verified successfully.` };
      } else {
        actionStatus = { type: 'error', message: `${failed.length} of ${results.length} backups FAILED verification!` };
      }
    } catch (e) {
      const err = formatError(e);
      actionStatus = { type: 'error', message: `${err.title}: ${err.message}` };
    } finally {
      verifyingAll = false;
    }
  }

  async function handleBackupCrypttab() {
    try {
      await backupCrypttab();
      actionStatus = { type: 'success', message: 'crypttab backed up successfully.' };
      await loadBackups();
    } catch (e) {
      const err = formatError(e);
      actionStatus = { type: 'error', message: `${err.title}: ${err.message}` };
    }
  }

  function requestRestore(backup) {
    if (backup.backup_type !== 'LuksHeader') return;
    pendingRestore = backup;
    restoreConfirmOpen = true;
  }

  async function executeRestore() {
    if (!pendingRestore) return;
    const backup = pendingRestore;
    pendingRestore = null;
    restoring = true;
    try {
      await restoreLuksHeader(backup.source_path, backup.id);
      actionStatus = { type: 'success', message: `LUKS header restored from backup (${formatDate(backup.created_at)}). A safety backup of the previous header was created automatically.` };
    } catch (e) {
      const err = formatError(e);
      actionStatus = { type: 'error', message: `${err.title}: ${err.message}` };
    } finally {
      restoring = false;
    }
  }

  function formatSize(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatDate(dateStr) {
    try {
      return new Date(dateStr).toLocaleString();
    } catch {
      return dateStr;
    }
  }

  const typeLabels = {
    LuksHeader: 'LUKS Header',
    Crypttab: 'crypttab',
    Initramfs: 'Initramfs',
    BootConfig: 'Boot Config',
    KeySlotSnapshot: 'Key Slots',
  };

  const typeColors = {
    LuksHeader: 'text-danger',
    Crypttab: 'text-warning',
    Initramfs: 'text-accent',
    BootConfig: 'text-success',
    KeySlotSnapshot: 'text-text-secondary',
  };
</script>

<!-- Restore Confirmation (CRITICAL — requires typing) -->
<ConfirmDialog
  bind:open={restoreConfirmOpen}
  level="critical"
  title="Restore LUKS Header"
  message={`This will REPLACE the current LUKS header on ${pendingRestore?.source_path || 'the device'} with the backup from ${pendingRestore ? formatDate(pendingRestore.created_at) : ''}. A safety backup of the current header will be created first, but this is still a critical operation.`}
  details={pendingRestore ? `Device: ${pendingRestore.source_path}\nBackup: ${pendingRestore.backup_path}\nCreated: ${formatDate(pendingRestore.created_at)}\nSHA-256: ${pendingRestore.sha256}` : ''}
  confirmPhrase="RESTORE HEADER"
  confirmLabel="Restore Header"
  onconfirm={executeRestore}
/>

<div class="p-6 space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-2xl font-bold text-text-primary">Backup Manager</h2>
    <div class="flex gap-2">
      <button
        class="px-3 py-1.5 bg-accent hover:bg-accent-hover text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
        onclick={handleVerifyAll}
        disabled={verifyingAll}
      >
        {verifyingAll ? 'Verifying...' : 'Verify All'}
      </button>
      <button
        class="px-3 py-1.5 bg-surface-2 hover:bg-surface-3 text-text-primary rounded-lg text-sm font-medium transition-colors border border-border"
        onclick={handleBackupCrypttab}
      >
        Backup crypttab
      </button>
    </div>
  </div>

  <!-- Filter tabs -->
  <div class="flex gap-1 bg-surface-1 rounded-lg p-1">
    {#each [['all', 'All'], ['luks_header', 'LUKS Headers'], ['crypttab', 'crypttab'], ['initramfs', 'Initramfs'], ['boot_config', 'Boot Config']] as [value, label]}
      <button
        class="px-3 py-1.5 text-sm rounded-md transition-colors {filterType === value ? 'bg-accent text-white' : 'text-text-secondary hover:text-text-primary'}"
        onclick={() => { filterType = value; loadBackups(); }}
      >{label}</button>
    {/each}
  </div>

  <!-- Status messages -->
  {#if actionStatus}
    <div class="rounded-lg p-3 text-sm {actionStatus.type === 'success'
      ? 'bg-success/10 border border-success/30 text-success'
      : 'bg-danger/10 border border-danger/30 text-danger'}">
      {actionStatus.message}
    </div>
  {/if}

  {#if loading}
    <div class="flex items-center gap-3 text-text-secondary">
      <div class="animate-spin w-5 h-5 border-2 border-accent border-t-transparent rounded-full"></div>
      Loading backups...
    </div>
  {:else if backups.length === 0}
    <div class="bg-surface-1 border border-border rounded-lg p-8 text-center">
      <p class="text-text-muted text-lg">No backups yet</p>
      <p class="text-text-muted text-sm mt-2">Backups are created automatically before any destructive operation.</p>
      <p class="text-text-muted text-sm">You can also create manual backups from the LUKS Manager.</p>
    </div>
  {:else}
    <!-- Backup list -->
    <div class="space-y-3">
      {#each backups as backup}
        {@const isVerified = verificationResults?.find(([b, _]) => b.id === backup.id)}
        <div class="bg-surface-1 border border-border rounded-lg p-4 flex items-center gap-4">
          <!-- Type badge -->
          <div class="shrink-0">
            <span class="text-xs font-bold uppercase px-2 py-1 rounded bg-surface-2 {typeColors[backup.backup_type] || 'text-text-muted'}">
              {typeLabels[backup.backup_type] || backup.backup_type}
            </span>
          </div>

          <!-- Info -->
          <div class="flex-1 min-w-0">
            <p class="text-sm text-text-primary font-medium truncate">{backup.description}</p>
            <div class="flex items-center gap-3 mt-1 text-xs text-text-muted">
              <span>{formatDate(backup.created_at)}</span>
              <span>{formatSize(backup.size_bytes)}</span>
              <span class="font-mono truncate max-w-[200px]" title={backup.sha256}>SHA: {backup.sha256.substring(0, 12)}...</span>
            </div>
          </div>

          <!-- Verification status -->
          {#if isVerified}
            <span class="text-sm {isVerified[1] ? 'text-success' : 'text-danger'} font-medium">
              {isVerified[1] ? 'Valid' : 'CORRUPT'}
            </span>
          {/if}

          <!-- Verify button -->
          <button
            class="shrink-0 px-3 py-1.5 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-xs text-text-secondary transition-colors disabled:opacity-50"
            onclick={() => handleVerify(backup.id)}
            disabled={verifying === backup.id}
          >
            {verifying === backup.id ? 'Checking...' : 'Verify'}
          </button>

          <!-- Restore button (LUKS headers only) -->
          {#if backup.backup_type === 'LuksHeader'}
            <button
              class="shrink-0 px-3 py-1.5 bg-danger/10 hover:bg-danger/20 border border-danger/30 rounded-lg text-xs text-danger font-medium transition-colors disabled:opacity-50"
              onclick={() => requestRestore(backup)}
              disabled={restoring}
            >
              {restoring ? 'Restoring...' : 'Restore'}
            </button>
          {/if}
        </div>
      {/each}
    </div>

    <p class="text-xs text-text-muted">
      {backups.length} backup(s) stored. All backups include SHA-256 checksums for integrity verification.
    </p>
  {/if}
</div>
