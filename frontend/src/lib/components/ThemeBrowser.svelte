<script>
  import { onMount } from 'svelte';
  import { listThemes, setActiveTheme, getThemeScreenshot } from '../utils/api.js';
  import { formatError } from '../utils/errors.js';

  let themes = $state([]);
  let loading = $state(true);
  let error = $state(null);
  let hoveredTheme = $state(null);
  let previewScreenshot = $state(null);
  let loadingScreenshot = $state(false);
  let settingTheme = $state(null);

  onMount(async () => {
    await loadThemes();
  });

  async function loadThemes() {
    loading = true;
    try {
      themes = await listThemes();
    } catch (e) {
      error = formatError(e);
    } finally {
      loading = false;
    }
  }

  async function handleHover(theme) {
    hoveredTheme = theme.id;
    if (theme.thumbnail_data) {
      previewScreenshot = theme.thumbnail_data;
    } else {
      loadingScreenshot = true;
      try {
        const screenshot = await getThemeScreenshot(theme.id);
        if (hoveredTheme === theme.id) {
          previewScreenshot = screenshot;
        }
      } catch {
        // Silently fail for preview
      } finally {
        loadingScreenshot = false;
      }
    }
  }

  function handleHoverEnd() {
    hoveredTheme = null;
    previewScreenshot = null;
  }

  async function handleSetActive(themeId) {
    settingTheme = themeId;
    try {
      await setActiveTheme(themeId);
      await loadThemes();
    } catch (e) {
      error = formatError(e);
    } finally {
      settingTheme = null;
    }
  }

  const styleIcons = {
    Text: '&#9776;',
    Graphical: '&#127912;',
  };
</script>

<div class="p-6 space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-2xl font-bold text-text-primary">Theme Browser</h2>
      <p class="text-sm text-text-muted mt-1">Hover over themes to preview. Click to activate.</p>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center gap-3 text-text-secondary">
      <div class="animate-spin w-5 h-5 border-2 border-accent border-t-transparent rounded-full"></div>
      Loading themes...
    </div>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger">
      <p class="font-semibold">{error.title}</p>
      <p class="text-sm mt-1">{error.message}</p>
      {#if error.hint}
        <p class="text-xs text-text-muted mt-2">{error.hint}</p>
      {/if}
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
      {#each themes as theme}
        <div
          class="group relative bg-surface-1 border-2 rounded-xl overflow-hidden transition-all hover:shadow-lg {theme.active ? 'border-accent shadow-accent/20' : 'border-border hover:border-accent/50'}"
          role="button"
          tabindex="0"
          onmouseenter={() => handleHover(theme)}
          onmouseleave={handleHoverEnd}
        >
          <!-- Thumbnail / Preview Area -->
          <div class="relative h-40 bg-surface-2 flex items-center justify-center overflow-hidden">
            {#if hoveredTheme === theme.id && previewScreenshot}
              <img src={previewScreenshot} alt="{theme.name} preview" class="w-full h-full object-cover transition-opacity" />
            {:else if theme.thumbnail_data}
              <img src={theme.thumbnail_data} alt="{theme.name} thumbnail" class="w-full h-full object-cover" />
            {:else}
              <!-- Placeholder with style icon -->
              <div class="text-center">
                <div class="text-4xl text-text-muted mb-2">{@html styleIcons[theme.style] || '&#9733;'}</div>
                <p class="text-sm text-text-muted">{theme.style === 'Graphical' ? 'Graphical (BURG-style)' : 'Text-based'}</p>
              </div>
            {/if}

            {#if hoveredTheme === theme.id && loadingScreenshot}
              <div class="absolute inset-0 bg-surface-0/50 flex items-center justify-center">
                <div class="animate-spin w-6 h-6 border-2 border-accent border-t-transparent rounded-full"></div>
              </div>
            {/if}

            <!-- Hover overlay with "Preview" tooltip -->
            <div class="absolute inset-0 bg-surface-0/0 group-hover:bg-surface-0/30 transition-colors flex items-center justify-center">
              <span class="opacity-0 group-hover:opacity-100 transition-opacity bg-surface-0/80 text-text-primary px-3 py-1 rounded-full text-sm">
                Preview
              </span>
            </div>

            {#if theme.active}
              <div class="absolute top-2 right-2 bg-accent text-white text-[10px] px-2 py-0.5 rounded-full font-bold">
                ACTIVE
              </div>
            {/if}
          </div>

          <!-- Info -->
          <div class="p-4">
            <div class="flex items-center justify-between mb-2">
              <h3 class="font-bold text-text-primary">{theme.name}</h3>
              <span class="text-[10px] text-text-muted uppercase bg-surface-2 px-2 py-0.5 rounded">
                {theme.style}
              </span>
            </div>
            <p class="text-sm text-text-secondary mb-3">{theme.description}</p>
            <div class="flex items-center justify-between">
              <span class="text-xs text-text-muted">by {theme.author} &middot; v{theme.version}</span>
              {#if !theme.active}
                <button
                  class="px-3 py-1.5 bg-accent hover:bg-accent-hover text-white rounded-lg text-xs font-medium transition-colors disabled:opacity-50"
                  onclick={() => handleSetActive(theme.id)}
                  disabled={settingTheme !== null}
                >
                  {settingTheme === theme.id ? 'Activating...' : 'Activate'}
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>

    {#if themes.length === 0}
      <div class="bg-surface-1 border border-border rounded-lg p-8 text-center">
        <p class="text-text-muted text-lg">No themes installed</p>
        <p class="text-text-muted text-sm mt-2">Themes will appear here once downloaded from the theme repository.</p>
      </div>
    {/if}
  {/if}
</div>
