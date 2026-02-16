<script>
  /**
   * ConfirmDialog — Safety gate for destructive operations.
   *
   * Three confirmation levels:
   *   "info"     — Simple OK/Cancel (non-destructive)
   *   "warning"  — Must click Confirm (moderately risky)
   *   "critical" — Must type a confirmation phrase (destructive/irreversible)
   */

  let {
    open = $bindable(false),
    level = 'warning',        // 'info' | 'warning' | 'critical'
    title = 'Confirm Action',
    message = '',
    details = null,           // Optional extra detail block
    confirmPhrase = '',       // Required text for critical level
    confirmLabel = 'Confirm',
    cancelLabel = 'Cancel',
    onconfirm = () => {},
    oncancel = () => {},
  } = $props();

  let typedPhrase = $state('');
  let canConfirm = $derived(
    level === 'critical' ? typedPhrase.trim() === confirmPhrase.trim() : true
  );

  function handleConfirm() {
    if (!canConfirm) return;
    typedPhrase = '';
    open = false;
    onconfirm();
  }

  function handleCancel() {
    typedPhrase = '';
    open = false;
    oncancel();
  }

  function handleKeydown(e) {
    if (e.key === 'Escape') handleCancel();
    if (e.key === 'Enter' && canConfirm) handleConfirm();
  }

  const levelStyles = {
    info: {
      icon: '&#9432;',
      iconBg: 'bg-accent/20 text-accent',
      border: 'border-accent/30',
      confirmBtn: 'bg-accent hover:bg-accent-hover',
    },
    warning: {
      icon: '&#9888;',
      iconBg: 'bg-warning/20 text-warning',
      border: 'border-warning/30',
      confirmBtn: 'bg-warning hover:bg-warning/80',
    },
    critical: {
      icon: '&#9888;',
      iconBg: 'bg-danger/20 text-danger',
      border: 'border-danger/30',
      confirmBtn: 'bg-danger hover:bg-danger/80',
    },
  };

  let style = $derived(levelStyles[level] || levelStyles.warning);
</script>

{#if open}
  <!-- Backdrop -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    onkeydown={handleKeydown}
  >
    <!-- Dialog -->
    <div class="bg-surface-1 border {style.border} rounded-xl shadow-2xl max-w-lg w-full mx-4 overflow-hidden">
      <!-- Header -->
      <div class="flex items-center gap-4 p-5 border-b border-border">
        <div class="w-12 h-12 rounded-full flex items-center justify-center text-2xl {style.iconBg}">
          {@html style.icon}
        </div>
        <div>
          <h3 class="text-lg font-bold text-text-primary">{title}</h3>
          {#if level === 'critical'}
            <p class="text-xs text-danger font-medium mt-0.5">This action cannot be easily undone</p>
          {/if}
        </div>
      </div>

      <!-- Body -->
      <div class="p-5 space-y-4">
        <p class="text-sm text-text-secondary leading-relaxed">{message}</p>

        {#if details}
          <div class="bg-surface-2 rounded-lg p-3 text-xs font-mono text-text-muted overflow-x-auto">
            <pre class="whitespace-pre-wrap">{details}</pre>
          </div>
        {/if}

        <!-- Backup reminder -->
        {#if level === 'warning' || level === 'critical'}
          <div class="flex items-start gap-2 bg-success/10 border border-success/20 rounded-lg p-3">
            <span class="text-success text-sm mt-0.5">&#10003;</span>
            <p class="text-xs text-success">A backup will be created automatically before this operation.</p>
          </div>
        {/if}

        <!-- Critical: type phrase to confirm -->
        {#if level === 'critical'}
          <div class="space-y-2">
            <p class="text-sm text-text-secondary">
              Type <span class="font-mono font-bold text-danger bg-danger/10 px-1.5 py-0.5 rounded">{confirmPhrase}</span> to confirm:
            </p>
            <input
              type="text"
              class="w-full bg-surface-2 border border-border rounded-lg px-3 py-2 text-sm text-text-primary font-mono focus:outline-none focus:border-danger focus:ring-1 focus:ring-danger/50"
              placeholder="Type the phrase above..."
              bind:value={typedPhrase}
              autocomplete="off"
              spellcheck="false"
            />
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-3 p-5 border-t border-border bg-surface-2/50">
        <button
          class="px-4 py-2 bg-surface-2 hover:bg-surface-3 border border-border rounded-lg text-sm text-text-secondary transition-colors"
          onclick={handleCancel}
        >
          {cancelLabel}
        </button>
        <button
          class="px-4 py-2 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-30 disabled:cursor-not-allowed {style.confirmBtn}"
          onclick={handleConfirm}
          disabled={!canConfirm}
        >
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}
