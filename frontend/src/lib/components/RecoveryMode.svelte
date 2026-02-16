<script>
  import { onMount } from 'svelte';
  import { runDiagnostics, getRecoveryBackups, rebuildInitramfs } from '../utils/api.js';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import Toast from './Toast.svelte';
  import { formatError } from '../utils/errors.js';

  let diagnostics = $state(null);
  let backups = $state([]);
  let scanning = $state(false);
  let executingAction = $state(null);

  let confirmDialog;
  let toast;

  onMount(async () => {
    await runScan();
  });

  async function runScan() {
    scanning = true;
    try {
      diagnostics = await runDiagnostics();
      backups = await getRecoveryBackups();
    } catch (e) {
      const err = formatError(e);
      toast?.show(`${err.title}: ${err.message}`, 'error');
    }
    scanning = false;
  }

  function healthColor(status) {
    switch (status) {
      case 'Healthy': return 'text-success';
      case 'Warning': return 'text-warning';
      case 'Critical': return 'text-danger';
      default: return 'text-text-muted';
    }
  }

  function healthBg(status) {
    switch (status) {
      case 'Healthy': return 'bg-success/10 border-success/30';
      case 'Warning': return 'bg-warning/10 border-warning/30';
      case 'Critical': return 'bg-danger/10 border-danger/30';
      default: return 'bg-surface-1 border-border';
    }
  }

  function checkIcon(status) {
    switch (status) {
      case 'Pass': return { icon: '&#10003;', class: 'text-success' };
      case 'Warning': return { icon: '&#9888;', class: 'text-warning' };
      case 'Fail': return { icon: '&#10007;', class: 'text-danger' };
      case 'Skipped': return { icon: '&#8212;', class: 'text-text-muted' };
      default: return { icon: '?', class: 'text-text-muted' };
    }
  }

  function severityBadge(severity) {
    switch (severity) {
      case 'Critical': return 'bg-danger/20 text-danger';
      case 'High': return 'bg-danger/10 text-danger/80';
      case 'Medium': return 'bg-warning/20 text-warning';
      case 'Low': return 'bg-surface-2 text-text-muted';
      default: return 'bg-surface-2 text-text-muted';
    }
  }

  async function executeAction(issue, action) {
    const level = action.destructive ? 'critical' : 'warning';
    const confirmPhrase = action.destructive ? 'EXECUTE RECOVERY' : undefined;

    confirmDialog?.show({
      title: action.label,
      message: `${action.description}${action.command_preview ? '\n\nCommand: ' + action.command_preview : ''}`,
      level,
      confirmPhrase,
      confirmLabel: 'Execute',
      onConfirm: async () => {
        executingAction = action.id;
        try {
          // Route to the appropriate action handler
          switch (action.id) {
            case 'rebuild-initramfs':
              const result = await rebuildInitramfs();
              if (result.success) {
                toast?.show('Initramfs rebuilt successfully', 'success');
              } else {
                toast?.show('Initramfs rebuild failed: ' + result.stderr, 'error');
              }
              break;
            case 'create-backups':
              toast?.show('Creating fresh backups...', 'info');
              // The backup commands auto-backup; we trigger a rescan
              break;
            default:
              toast?.show(`Action "${action.label}" requires manual intervention. See logs for details.`, 'warning');
          }
          // Rescan after any action
          await runScan();
        } catch (e) {
          const err = formatError(e);
          toast?.show(`${err.title}: ${err.message}`, 'error');
        }
        executingAction = null;
      },
    });
  }
</script>

<ConfirmDialog bind:this={confirmDialog} />
<Toast bind:this={toast} />

<div class="p-6 max-w-4xl">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-2xl font-bold text-text-primary mb-1">Recovery Mode</h2>
      <p class="text-sm text-text-muted">
        System diagnostics and guided recovery for encryption-related issues
      </p>
    </div>
    <button
      class="px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent/80 transition-colors text-sm disabled:opacity-50"
      disabled={scanning}
      onclick={runScan}
    >
      {scanning ? 'Scanning...' : 'Re-scan'}
    </button>
  </div>

  {#if scanning && !diagnostics}
    <div class="flex items-center gap-3 py-12 justify-center">
      <div class="w-6 h-6 border-2 border-accent border-t-transparent rounded-full animate-spin"></div>
      <span class="text-text-secondary">Running diagnostics...</span>
    </div>
  {:else if diagnostics}
    <!-- Overall Health Banner -->
    <div class="border rounded-lg p-4 mb-6 {healthBg(diagnostics.overall_health)}">
      <div class="flex items-center gap-3">
        <span class="text-2xl {healthColor(diagnostics.overall_health)}">
          {#if diagnostics.overall_health === 'Healthy'}
            &#10003;
          {:else if diagnostics.overall_health === 'Warning'}
            &#9888;
          {:else}
            &#10007;
          {/if}
        </span>
        <div>
          <h3 class="text-lg font-semibold {healthColor(diagnostics.overall_health)}">
            System {diagnostics.overall_health}
          </h3>
          <p class="text-sm text-text-muted">
            {diagnostics.checks.length} checks performed, {diagnostics.issues.length} issue(s) found
          </p>
        </div>
      </div>
    </div>

    <!-- Issues (if any) -->
    {#if diagnostics.issues.length > 0}
      <div class="mb-6">
        <h3 class="text-lg font-semibold text-text-primary mb-3">Issues Detected</h3>
        <div class="space-y-3">
          {#each diagnostics.issues as issue}
            <div class="bg-surface-1 rounded-lg border border-border overflow-hidden">
              <div class="p-4">
                <div class="flex items-center gap-2 mb-2">
                  <span class="px-2 py-0.5 rounded text-xs font-medium {severityBadge(issue.severity)}">
                    {issue.severity}
                  </span>
                  <h4 class="text-text-primary font-medium">{issue.title}</h4>
                </div>
                <p class="text-sm text-text-secondary mb-3">{issue.description}</p>

                {#if issue.recovery_actions.length > 0}
                  <div class="flex flex-wrap gap-2">
                    {#each issue.recovery_actions as action}
                      <button
                        class="px-3 py-1.5 text-sm rounded-lg transition-colors disabled:opacity-50
                          {action.destructive
                            ? 'bg-danger/10 text-danger hover:bg-danger/20 border border-danger/30'
                            : 'bg-accent/10 text-accent hover:bg-accent/20 border border-accent/30'}"
                        disabled={executingAction === action.id}
                        onclick={() => executeAction(issue, action)}
                      >
                        {executingAction === action.id ? 'Running...' : action.label}
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Health Checks Detail -->
    <div class="mb-6">
      <h3 class="text-lg font-semibold text-text-primary mb-3">Health Checks</h3>
      <div class="bg-surface-1 rounded-lg border border-border overflow-hidden divide-y divide-border">
        {#each diagnostics.checks as check}
          {@const icon = checkIcon(check.status)}
          <div class="p-3 flex items-center gap-3">
            <span class="w-6 text-center shrink-0 {icon.class}">{@html icon.icon}</span>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium text-text-primary">{check.name}</span>
                <span class="text-xs text-text-muted hidden sm:inline">{check.description}</span>
              </div>
              {#if check.detail}
                <p class="text-xs text-text-muted mt-0.5 truncate">{check.detail}</p>
              {/if}
            </div>
            <span class="px-2 py-0.5 rounded text-xs shrink-0
              {check.status === 'Pass' ? 'bg-success/10 text-success' :
               check.status === 'Warning' ? 'bg-warning/10 text-warning' :
               check.status === 'Fail' ? 'bg-danger/10 text-danger' :
               'bg-surface-2 text-text-muted'}">
              {check.status}
            </span>
          </div>
        {/each}
      </div>
    </div>

    <!-- Available Backups for Recovery -->
    {#if backups.length > 0}
      <div>
        <h3 class="text-lg font-semibold text-text-primary mb-3">Available Backups</h3>
        <p class="text-sm text-text-muted mb-3">
          These backups can be used to restore previous configurations if needed.
          Go to the Backups page for full management.
        </p>
        <div class="bg-surface-1 rounded-lg border border-border overflow-hidden">
          <div class="max-h-64 overflow-y-auto divide-y divide-border">
            {#each backups.slice(0, 10) as backup}
              <div class="px-4 py-2.5 flex items-center justify-between">
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="px-1.5 py-0.5 rounded text-xs bg-surface-2 text-text-muted">
                      {backup.backup_type === 'LuksHeader' ? 'LUKS' :
                       backup.backup_type === 'Crypttab' ? 'crypttab' :
                       backup.backup_type === 'Initramfs' ? 'initramfs' :
                       backup.backup_type === 'BootConfig' ? 'boot' :
                       backup.backup_type}
                    </span>
                    <span class="text-sm text-text-primary truncate">{backup.description}</span>
                  </div>
                  <p class="text-xs text-text-muted mt-0.5">
                    {new Date(backup.created_at).toLocaleString()}
                    &middot; {(backup.size_bytes / 1024).toFixed(1)} KB
                  </p>
                </div>
              </div>
            {/each}
          </div>
          {#if backups.length > 10}
            <div class="px-4 py-2 text-center text-xs text-text-muted border-t border-border">
              ...and {backups.length - 10} more (see Backups page)
            </div>
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</div>
