<script>
  import { onMount } from 'svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Dashboard from './lib/components/Dashboard.svelte';
  import OsPicker from './lib/components/OsPicker.svelte';
  import ThemeBrowser from './lib/components/ThemeBrowser.svelte';
  import LuksManager from './lib/components/LuksManager.svelte';
  import BackupManager from './lib/components/BackupManager.svelte';
  import LogViewer from './lib/components/LogViewer.svelte';

  let currentPage = $state('dashboard');
  let distroName = $state('');

  onMount(async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const distro = await invoke('detect_distro');
      if (distro?.pretty_name) {
        distroName = distro.pretty_name;
      }
    } catch {
      // Running outside Tauri (dev mode in browser) — OK
      distroName = 'Development Mode';
    }
  });
</script>

<div class="flex h-screen bg-surface-0">
  <Sidebar bind:currentPage {distroName} />

  <main class="flex-1 overflow-y-auto">
    {#if currentPage === 'dashboard'}
      <Dashboard />
    {:else if currentPage === 'os-picker'}
      <OsPicker />
    {:else if currentPage === 'themes'}
      <ThemeBrowser />
    {:else if currentPage === 'luks'}
      <LuksManager />
    {:else if currentPage === 'backups'}
      <BackupManager />
    {:else if currentPage === 'logs'}
      <LogViewer />
    {/if}
  </main>
</div>
