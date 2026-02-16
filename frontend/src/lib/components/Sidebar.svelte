<script>
  let { currentPage = $bindable(), distroName = '' } = $props();

  const navItems = [
    { id: 'dashboard', label: 'Dashboard', icon: '&#9632;' },
    { id: 'os-picker', label: 'OS Picker', icon: '&#9654;' },
    { id: 'themes', label: 'Themes', icon: '&#9733;' },
    { id: 'luks', label: 'LUKS Manager', icon: '&#128274;' },
    { id: 'clevis', label: 'Network Unlock', icon: '&#128279;' },
    { id: 'backups', label: 'Backups', icon: '&#128190;' },
    { id: 'recovery', label: 'Recovery', icon: '&#128657;' },
    { id: 'logs', label: 'Logs', icon: '&#128196;' },
  ];
</script>

<aside class="w-56 bg-surface-1 border-r border-border flex flex-col h-screen shrink-0">
  <!-- Logo -->
  <div class="p-4 border-b border-border">
    <h1 class="text-lg font-bold text-text-primary tracking-tight">LVM Loader</h1>
    {#if distroName}
      <p class="text-xs text-text-muted mt-1">{distroName}</p>
    {/if}
  </div>

  <!-- Navigation -->
  <nav class="flex-1 py-2">
    {#each navItems as item}
      <button
        class="w-full text-left px-4 py-2.5 flex items-center gap-3 transition-colors {currentPage === item.id
          ? 'bg-surface-3 text-accent border-r-2 border-accent'
          : 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'}"
        onclick={() => (currentPage = item.id)}
      >
        <span class="text-base w-5 text-center">{@html item.icon}</span>
        <span class="text-sm">{item.label}</span>
      </button>
    {/each}
  </nav>

  <!-- Footer -->
  <div class="p-4 border-t border-border">
    <p class="text-xs text-text-muted">v0.1.0</p>
  </div>
</aside>
