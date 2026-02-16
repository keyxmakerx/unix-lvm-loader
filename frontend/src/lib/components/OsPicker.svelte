<script>
  import { onMount } from 'svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import { scanBootEntries, setDefaultBoot } from '../utils/api.js';
  import { formatError } from '../utils/errors.js';
  import { getDistroIcon } from '../utils/distro-icons.js';

  let bootState = $state(null);
  let loading = $state(true);
  let error = $state(null);
  let settingDefault = $state(null);
  let viewMode = $state('graphical'); // 'graphical' (BURG-style) or 'text'
  let selectedEntry = $state(null);

  // Confirmation
  let confirmOpen = $state(false);
  let pendingDefaultId = $state(null);

  onMount(async () => {
    await loadEntries();
  });

  async function loadEntries() {
    loading = true;
    error = null;
    try {
      bootState = await scanBootEntries();
      selectedEntry = bootState.entries.find(e => e.is_default) || bootState.entries[0] || null;
    } catch (e) {
      const err = formatError(e);
      error = err;
    } finally {
      loading = false;
    }
  }

  function requestSetDefault(entryId) {
    pendingDefaultId = entryId;
    confirmOpen = true;
  }

  async function executeSetDefault() {
    const entryId = pendingDefaultId;
    if (!entryId) return;
    pendingDefaultId = null;
    settingDefault = entryId;
    try {
      await setDefaultBoot(entryId);
      await loadEntries();
    } catch (e) {
      const err = formatError(e);
      error = err;
    } finally {
      settingDefault = null;
    }
  }

  const distroColors = {
    ubuntu: '#E95420',
    debian: '#A80030',
    fedora: '#3C6EB4',
    nobara: '#7B2D8B',
    bazzite: '#1A5CFF',
    cachyos: '#0DB7ED',
    arch: '#1793D1',
    manjaro: '#35BF5C',
    endeavouros: '#7B3FA0',
    'pop-os': '#48B9C7',
    linuxmint: '#87CF3E',
    opensuse: '#73BA25',
    windows: '#0078D4',
    linux: '#FCC624',
  };

  function getColor(entry) {
    return distroColors[entry.distro_icon] || '#6366f1';
  }
</script>

<!-- Set Default Boot Confirmation -->
<ConfirmDialog
  bind:open={confirmOpen}
  level="warning"
  title="Change Default Boot Entry"
  message={`This will change the default boot entry to "${bootState?.entries?.find(e => e.id === pendingDefaultId)?.title || pendingDefaultId}". The boot loader configuration will be updated.`}
  details={`Boot loader: ${bootState?.boot_loader || 'unknown'}\nEntry ID: ${pendingDefaultId || ''}\n\nA backup of your boot configuration will be created before making changes.`}
  confirmLabel="Set as Default"
  onconfirm={executeSetDefault}
/>

<div class="p-6 space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-2xl font-bold text-text-primary">OS Picker</h2>

    <!-- View mode toggle -->
    <div class="flex bg-surface-2 rounded-lg p-1">
      <button
        class="px-3 py-1.5 text-sm rounded-md transition-colors {viewMode === 'graphical' ? 'bg-accent text-white' : 'text-text-secondary hover:text-text-primary'}"
        onclick={() => (viewMode = 'graphical')}
      >Graphical</button>
      <button
        class="px-3 py-1.5 text-sm rounded-md transition-colors {viewMode === 'text' ? 'bg-accent text-white' : 'text-text-secondary hover:text-text-primary'}"
        onclick={() => (viewMode = 'text')}
      >Text</button>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center gap-3 text-text-secondary">
      <div class="animate-spin w-5 h-5 border-2 border-accent border-t-transparent rounded-full"></div>
      Scanning boot entries...
    </div>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger">
      <p class="font-semibold">{error.title}</p>
      <p class="text-sm mt-1">{error.message}</p>
      {#if error.hint}
        <p class="text-xs text-text-muted mt-2">{error.hint}</p>
      {/if}
    </div>
  {:else if bootState}

    {#if bootState.entries.length === 0}
      <div class="bg-surface-1 border border-border rounded-lg p-8 text-center">
        <p class="text-text-muted text-lg">No boot entries found</p>
        <p class="text-text-muted text-sm mt-2">Boot loader: {bootState.boot_loader}</p>
      </div>
    {:else}

      <!-- ═══ GRAPHICAL MODE (BURG-style) ═══ -->
      {#if viewMode === 'graphical'}
        <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
          {#each bootState.entries as entry}
            {@const color = getColor(entry)}
            {@const icon = getDistroIcon(entry.distro_icon)}
            <button
              class="group relative bg-surface-1 border-2 rounded-xl p-6 text-center transition-all hover:scale-105 hover:shadow-lg {selectedEntry?.id === entry.id ? 'border-accent shadow-accent/20 shadow-lg' : 'border-border hover:border-accent/50'}"
              onclick={() => (selectedEntry = entry)}
              ondblclick={() => requestSetDefault(entry.id)}
            >
              {#if entry.is_default}
                <div class="absolute top-2 right-2 bg-success text-white text-[10px] px-1.5 py-0.5 rounded-full font-bold">DEFAULT</div>
              {/if}

              <!-- OS Icon -->
              {#if icon}
                <div class="w-20 h-20 mx-auto mb-4 transition-transform group-hover:scale-110">
                  {@html icon}
                </div>
              {:else}
                <div
                  class="w-20 h-20 mx-auto rounded-full flex items-center justify-center text-3xl font-bold text-white mb-4 transition-transform group-hover:scale-110"
                  style="background-color: {color};"
                >
                  {entry.title.charAt(0).toUpperCase()}
                </div>
              {/if}

              <h3 class="font-semibold text-text-primary text-sm leading-tight">{entry.title}</h3>

              {#if entry.linux_kernel}
                <p class="text-[11px] text-text-muted mt-1 font-mono truncate">{entry.linux_kernel}</p>
              {/if}

              <p class="text-[10px] text-text-muted mt-1 uppercase">{entry.source}</p>
            </button>
          {/each}
        </div>

        <!-- Selected entry details -->
        {#if selectedEntry}
          <div class="bg-surface-1 border border-border rounded-lg p-5 mt-4">
            <div class="flex items-center justify-between mb-3">
              <h3 class="font-bold text-text-primary text-lg">{selectedEntry.title}</h3>
              {#if !selectedEntry.is_default}
                <button
                  class="px-4 py-2 bg-accent hover:bg-accent-hover text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
                  onclick={() => requestSetDefault(selectedEntry.id)}
                  disabled={settingDefault !== null}
                >
                  {settingDefault === selectedEntry.id ? 'Setting...' : 'Set as Default'}
                </button>
              {/if}
            </div>
            <div class="grid grid-cols-2 gap-3 text-sm">
              <div><span class="text-text-muted">ID:</span> <span class="text-text-secondary font-mono">{selectedEntry.id}</span></div>
              <div><span class="text-text-muted">Source:</span> <span class="text-text-secondary">{selectedEntry.source}</span></div>
              {#if selectedEntry.linux_kernel}
                <div class="col-span-2"><span class="text-text-muted">Kernel:</span> <span class="text-text-secondary font-mono">{selectedEntry.linux_kernel}</span></div>
              {/if}
              {#if selectedEntry.initrd}
                <div class="col-span-2"><span class="text-text-muted">Initrd:</span> <span class="text-text-secondary font-mono">{selectedEntry.initrd}</span></div>
              {/if}
              {#if selectedEntry.options}
                <div class="col-span-2"><span class="text-text-muted">Options:</span> <span class="text-text-secondary font-mono text-xs break-all">{selectedEntry.options}</span></div>
              {/if}
            </div>
          </div>
        {/if}

      <!-- ═══ TEXT MODE (classic list) ═══ -->
      {:else}
        <div class="bg-surface-0 border border-border rounded-lg font-mono text-sm overflow-hidden">
          <!-- Header -->
          <div class="bg-surface-2 px-4 py-2 border-b border-border flex items-center gap-4">
            <span class="text-text-muted w-8">#</span>
            <span class="text-text-muted flex-1">Title</span>
            <span class="text-text-muted w-32">Source</span>
            <span class="text-text-muted w-20 text-right">Status</span>
          </div>

          {#each bootState.entries as entry, i}
            {@const textIcon = getDistroIcon(entry.distro_icon)}
            <button
              class="w-full text-left px-4 py-2.5 flex items-center gap-4 border-b border-border/50 transition-colors {selectedEntry?.id === entry.id
                ? 'bg-accent/10 text-accent'
                : 'text-text-secondary hover:bg-surface-1'}"
              onclick={() => (selectedEntry = entry)}
            >
              <span class="w-8 text-text-muted">{i}</span>
              {#if textIcon}
                <span class="w-5 h-5 shrink-0">{@html textIcon}</span>
              {/if}
              <span class="flex-1 text-text-primary truncate">
                {#if entry.is_default}<span class="text-success">*</span>{/if}
                {entry.title}
              </span>
              <span class="w-32 text-text-muted">{entry.source}</span>
              <span class="w-20 text-right">
                {#if entry.is_default}
                  <span class="text-success text-xs">DEFAULT</span>
                {/if}
              </span>
            </button>
          {/each}
        </div>

        <!-- Actions for text mode -->
        {#if selectedEntry && !selectedEntry.is_default}
          <div class="flex gap-3">
            <button
              class="px-4 py-2 bg-accent hover:bg-accent-hover text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
              onclick={() => requestSetDefault(selectedEntry.id)}
              disabled={settingDefault !== null}
            >
              {settingDefault === selectedEntry.id ? 'Setting...' : 'Set as Default Boot Entry'}
            </button>
          </div>
        {/if}
      {/if}

    {/if}

    <!-- Boot info footer -->
    <div class="text-xs text-text-muted flex items-center gap-4">
      <span>Boot Loader: {bootState.boot_loader}</span>
      {#if bootState.timeout !== null && bootState.timeout !== undefined}
        <span>Timeout: {bootState.timeout}s</span>
      {/if}
      {#if bootState.default_entry}
        <span>Default: {bootState.default_entry}</span>
      {/if}
    </div>
  {/if}
</div>
