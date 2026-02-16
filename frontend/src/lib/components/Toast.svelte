<script>
  /**
   * Toast notification that auto-dismisses.
   * Supports stacking multiple toasts.
   */

  let toasts = $state([]);
  let nextId = 0;

  export function show(message, type = 'info', duration = 5000) {
    const id = nextId++;
    toasts = [...toasts, { id, message, type, fading: false }];

    if (duration > 0) {
      setTimeout(() => dismiss(id), duration);
    }
  }

  export function dismiss(id) {
    // Start fade out
    toasts = toasts.map(t => t.id === id ? { ...t, fading: true } : t);
    setTimeout(() => {
      toasts = toasts.filter(t => t.id !== id);
    }, 300);
  }

  const typeStyles = {
    success: 'bg-success/90 text-white',
    error: 'bg-danger/90 text-white',
    warning: 'bg-warning/90 text-black',
    info: 'bg-accent/90 text-white',
  };

  const typeIcons = {
    success: '&#10003;',
    error: '&#10007;',
    warning: '&#9888;',
    info: '&#9432;',
  };
</script>

{#if toasts.length > 0}
  <div class="fixed top-4 right-4 z-[60] flex flex-col gap-2 pointer-events-none">
    {#each toasts as toast (toast.id)}
      <div
        class="pointer-events-auto flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg text-sm max-w-sm transition-all duration-300 {typeStyles[toast.type] || typeStyles.info} {toast.fading ? 'opacity-0 translate-x-4' : 'opacity-100'}"
      >
        <span class="text-base shrink-0">{@html typeIcons[toast.type] || typeIcons.info}</span>
        <span class="flex-1">{toast.message}</span>
        <button
          class="shrink-0 opacity-70 hover:opacity-100 transition-opacity text-lg leading-none"
          onclick={() => dismiss(toast.id)}
        >&#10005;</button>
      </div>
    {/each}
  </div>
{/if}
