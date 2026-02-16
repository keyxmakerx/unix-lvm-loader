<script>
  import { onMount } from 'svelte';
  import { getRecentLogs, getAuditLogs, exportLogs } from '../utils/api.js';

  let logs = $state([]);
  let loading = $state(true);
  let error = $state(null);
  let logMode = $state('operations'); // 'operations' | 'audit'
  let exporting = $state(false);

  onMount(async () => {
    await loadLogs();
  });

  async function loadLogs() {
    loading = true;
    error = null;
    try {
      logs = logMode === 'audit'
        ? await getAuditLogs(200)
        : await getRecentLogs(200);
      // Reverse so newest first
      logs = [...logs].reverse();
    } catch (e) {
      error = e?.message || String(e);
    } finally {
      loading = false;
    }
  }

  async function handleExport() {
    exporting = true;
    try {
      const data = await exportLogs();
      // Create a download link
      const blob = new Blob([data], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `lvm-loader-logs-${new Date().toISOString().split('T')[0]}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      error = e?.message || String(e);
    } finally {
      exporting = false;
    }
  }

  function formatTimestamp(ts) {
    try {
      return new Date(ts).toLocaleString();
    } catch {
      return ts;
    }
  }

  const levelColors = {
    Debug: 'text-text-muted',
    Info: 'text-accent',
    Warning: 'text-warning',
    Error: 'text-danger',
    Critical: 'text-danger font-bold',
  };

  const levelBg = {
    Debug: 'bg-surface-2',
    Info: 'bg-accent/10',
    Warning: 'bg-warning/10',
    Error: 'bg-danger/10',
    Critical: 'bg-danger/20',
  };
</script>

<div class="p-6 space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-2xl font-bold text-text-primary">Logs</h2>
    <div class="flex gap-2">
      <div class="flex bg-surface-2 rounded-lg p-1">
        <button
          class="px-3 py-1.5 text-sm rounded-md transition-colors {logMode === 'operations' ? 'bg-accent text-white' : 'text-text-secondary hover:text-text-primary'}"
          onclick={() => { logMode = 'operations'; loadLogs(); }}
        >Operations</button>
        <button
          class="px-3 py-1.5 text-sm rounded-md transition-colors {logMode === 'audit' ? 'bg-accent text-white' : 'text-text-secondary hover:text-text-primary'}"
          onclick={() => { logMode = 'audit'; loadLogs(); }}
        >Audit</button>
      </div>
      <button
        class="px-3 py-1.5 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-sm text-text-secondary transition-colors disabled:opacity-50"
        onclick={handleExport}
        disabled={exporting}
      >
        {exporting ? 'Exporting...' : 'Export JSON'}
      </button>
      <button
        class="px-3 py-1.5 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-sm text-text-secondary transition-colors"
        onclick={loadLogs}
      >Refresh</button>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center gap-3 text-text-secondary">
      <div class="animate-spin w-5 h-5 border-2 border-accent border-t-transparent rounded-full"></div>
      Loading logs...
    </div>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger">
      <p class="text-sm">{error}</p>
    </div>
  {:else if logs.length === 0}
    <div class="bg-surface-1 border border-border rounded-lg p-8 text-center">
      <p class="text-text-muted">No log entries yet.</p>
    </div>
  {:else}
    <div class="space-y-1">
      {#each logs as log}
        <div class="rounded-lg px-4 py-2.5 text-sm flex items-start gap-3 {levelBg[log.level] || 'bg-surface-1'}">
          <!-- Timestamp -->
          <span class="text-text-muted text-xs font-mono shrink-0 w-40">{formatTimestamp(log.timestamp)}</span>

          <!-- Level badge -->
          <span class="text-xs font-bold uppercase shrink-0 w-16 {levelColors[log.level] || 'text-text-muted'}">{log.level}</span>

          <!-- Category -->
          <span class="text-xs text-text-muted shrink-0 w-28 truncate" title={log.category}>{log.category}</span>

          <!-- Message -->
          <span class="flex-1 text-text-primary">{log.message}</span>

          <!-- Rollback hint -->
          {#if log.rollback_hint}
            <span class="text-[10px] text-warning shrink-0" title="Rollback: {log.rollback_hint}">
              [undo]
            </span>
          {/if}
        </div>
      {/each}
    </div>

    <p class="text-xs text-text-muted">{logs.length} entries shown (newest first)</p>
  {/if}
</div>
