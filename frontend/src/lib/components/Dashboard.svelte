<script>
  import { onMount } from 'svelte';
  import { getSystemOverview, getClevisStatus, runDiagnostics, getDemoMode, getLuksInfo, listClevisBindings } from '../utils/api.js';

  let overview = $state(null);
  let clevisStatus = $state(null);
  let diagnostics = $state(null);
  let loading = $state(true);
  let error = $state(null);
  let demoMode = $state(false);
  let setupStatus = $state(null);

  onMount(async () => {
    try {
      const [ov, cs, diag, dm] = await Promise.allSettled([
        getSystemOverview(),
        getClevisStatus(),
        runDiagnostics(),
        getDemoMode(),
      ]);
      overview = ov.status === 'fulfilled' ? ov.value : null;
      clevisStatus = cs.status === 'fulfilled' ? cs.value : null;
      diagnostics = diag.status === 'fulfilled' ? diag.value : null;
      demoMode = dm.status === 'fulfilled' ? dm.value : false;
      if (!overview) {
        error = ov.reason?.message || 'Failed to load system overview';
      } else {
        // Detect existing setup status
        await detectSetupStatus();
      }
    } catch (e) {
      error = e?.message || String(e);
    } finally {
      loading = false;
    }
  });

  async function detectSetupStatus() {
    if (!overview) return;
    const status = {
      hasLuks: overview.luks_volumes.length > 0,
      hasLvm: overview.lvm.available && overview.lvm.volume_groups.length > 0,
      hasBoot: overview.boot && overview.boot.entries.length > 0,
      hasCryptenroll: overview.has_cryptenroll,
      hasClevis: clevisStatus?.clevis_installed || false,
      hasBackups: overview.backup_count > 0,
      tpm2Enrolled: false,
      fido2Enrolled: false,
      recoveryKey: false,
      clevisBindings: 0,
      configuredDevices: [],
    };

    // Check each LUKS volume for existing enrollment
    for (const device of overview.luks_volumes) {
      try {
        const info = await getLuksInfo(device);
        const deviceStatus = {
          device,
          hasTpm2: info.tokens?.some(t => t.token_type?.includes('tpm2')) || false,
          hasFido2: info.tokens?.some(t => t.token_type?.includes('fido2')) || false,
          hasRecovery: info.key_slots?.length > 2,
          slotCount: info.active_passphrase_slots || 0,
        };
        status.configuredDevices.push(deviceStatus);
        if (deviceStatus.hasTpm2) status.tpm2Enrolled = true;
        if (deviceStatus.hasFido2) status.fido2Enrolled = true;
      } catch {
        // Skip devices we can't read
      }
    }

    // Check for existing Clevis bindings
    if (status.hasClevis && overview.luks_volumes.length > 0) {
      try {
        const bindings = await listClevisBindings(overview.luks_volumes[0]);
        status.clevisBindings = bindings.length;
      } catch {
        // OK if no bindings
      }
    }

    setupStatus = status;
  }

  function familyLabel(family) {
    const labels = {
      Debian: 'Debian / Ubuntu',
      Fedora: 'Fedora / Nobara',
      Atomic: 'Fedora Atomic (Immutable)',
      Arch: 'Arch / CachyOS',
      Suse: 'openSUSE',
      Unknown: 'Unknown',
    };
    return labels[family] || family;
  }
</script>

<div class="p-6 space-y-6">
  <h2 class="text-2xl font-bold text-text-primary">System Dashboard</h2>

  {#if demoMode}
    <div class="bg-warning/10 border border-warning/30 rounded-lg px-4 py-3 flex items-center gap-3">
      <span class="text-warning text-lg">&#9881;</span>
      <div>
        <p class="text-sm font-medium text-warning">Demo Mode Active</p>
        <p class="text-xs text-text-muted">Showing simulated data from a Fedora 41 workstation with LUKS2-on-LVM. Toggle off in the sidebar to see real system data.</p>
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="flex items-center gap-3 text-text-secondary">
      <div class="animate-spin w-5 h-5 border-2 border-accent border-t-transparent rounded-full"></div>
      Scanning system...
    </div>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger">
      <p class="font-semibold">Failed to scan system</p>
      <p class="text-sm mt-1">{error}</p>
    </div>
  {:else if overview}
    <!-- Info Cards Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">

      <!-- Distro Card -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-accent/20 rounded-lg flex items-center justify-center text-accent text-lg">
            &#128421;
          </div>
          <div>
            <h3 class="font-semibold text-text-primary">Distribution</h3>
            <p class="text-xs text-text-muted">Detected OS</p>
          </div>
        </div>
        {#if overview.distro}
          <p class="text-lg font-bold text-text-primary">{overview.distro.pretty_name}</p>
          <div class="mt-2 space-y-1 text-sm text-text-secondary">
            <p>Family: <span class="text-text-primary">{familyLabel(overview.distro.family)}</span></p>
            <p>Initramfs: <span class="text-text-primary">{overview.distro.initramfs}</span></p>
            <p>Package Manager: <span class="text-text-primary">{overview.distro.package_manager}</span></p>
            {#if overview.distro.is_immutable}
              <p class="text-warning font-medium">Immutable System</p>
            {/if}
          </div>
        {:else}
          <p class="text-text-muted">Could not detect distribution</p>
        {/if}
      </div>

      <!-- LUKS Card -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-success/20 rounded-lg flex items-center justify-center text-success text-lg">
            &#128274;
          </div>
          <div>
            <h3 class="font-semibold text-text-primary">LUKS Volumes</h3>
            <p class="text-xs text-text-muted">Encrypted volumes</p>
          </div>
        </div>
        <p class="text-3xl font-bold text-text-primary">{overview.luks_volumes.length}</p>
        {#if overview.luks_volumes.length > 0}
          <div class="mt-2 space-y-1">
            {#each overview.luks_volumes as vol}
              <p class="text-sm text-text-secondary font-mono">{vol}</p>
            {/each}
          </div>
        {:else}
          <p class="text-sm text-text-muted mt-2">No encrypted volumes found</p>
        {/if}
      </div>

      <!-- LVM Card -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-warning/20 rounded-lg flex items-center justify-center text-warning text-lg">
            &#9638;
          </div>
          <div>
            <h3 class="font-semibold text-text-primary">LVM</h3>
            <p class="text-xs text-text-muted">Logical Volume Manager</p>
          </div>
        </div>
        {#if overview.lvm.available}
          <div class="space-y-1 text-sm">
            <p class="text-text-secondary">PVs: <span class="text-text-primary font-bold">{overview.lvm.physical_volumes.length}</span></p>
            <p class="text-text-secondary">VGs: <span class="text-text-primary font-bold">{overview.lvm.volume_groups.length}</span></p>
            <p class="text-text-secondary">LVs: <span class="text-text-primary font-bold">{overview.lvm.logical_volumes.length}</span></p>
          </div>
        {:else}
          <p class="text-text-muted text-sm">LVM not available</p>
        {/if}
      </div>

      <!-- Boot Entries Card -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-accent/20 rounded-lg flex items-center justify-center text-accent text-lg">
            &#9654;
          </div>
          <div>
            <h3 class="font-semibold text-text-primary">Boot Entries</h3>
            <p class="text-xs text-text-muted">Installed operating systems</p>
          </div>
        </div>
        {#if overview.boot}
          <p class="text-3xl font-bold text-text-primary">{overview.boot.entries.length}</p>
          <p class="text-sm text-text-secondary mt-1">Boot Loader: <span class="text-text-primary">{overview.boot.boot_loader}</span></p>
        {:else}
          <p class="text-text-muted text-sm">Could not scan boot entries</p>
        {/if}
      </div>

      <!-- Backups Card -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-success/20 rounded-lg flex items-center justify-center text-success text-lg">
            &#128190;
          </div>
          <div>
            <h3 class="font-semibold text-text-primary">Backups</h3>
            <p class="text-xs text-text-muted">Safety backups stored</p>
          </div>
        </div>
        <p class="text-3xl font-bold text-text-primary">{overview.backup_count}</p>
        <p class="text-sm text-text-muted mt-1">Headers, crypttab, initramfs, boot config</p>
      </div>

      <!-- cryptenroll Card -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 rounded-lg flex items-center justify-center text-lg {overview.has_cryptenroll ? 'bg-success/20 text-success' : 'bg-surface-3 text-text-muted'}">
            &#128272;
          </div>
          <div>
            <h3 class="font-semibold text-text-primary">systemd-cryptenroll</h3>
            <p class="text-xs text-text-muted">TPM2 / FIDO2 support</p>
          </div>
        </div>
        {#if overview.has_cryptenroll}
          <p class="text-success font-semibold">Available</p>
          <p class="text-sm text-text-secondary mt-1">TPM2 and FIDO2 enrollment ready</p>
        {:else}
          <p class="text-warning font-semibold">Not Available</p>
          <p class="text-sm text-text-secondary mt-1">Install systemd-cryptenroll for hardware security</p>
        {/if}
      </div>

      <!-- Clevis/Tang Card -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 rounded-lg flex items-center justify-center text-lg {clevisStatus?.clevis_installed ? 'bg-success/20 text-success' : 'bg-surface-3 text-text-muted'}">
            &#128279;
          </div>
          <div>
            <h3 class="font-semibold text-text-primary">Clevis / Tang</h3>
            <p class="text-xs text-text-muted">Network-bound unlock</p>
          </div>
        </div>
        {#if clevisStatus?.clevis_installed}
          <p class="text-success font-semibold">Available</p>
          <p class="text-sm text-text-secondary mt-1">
            Network-based auto-unlock ready
            {#if clevisStatus.clevis_version}
              <span class="text-text-muted">({clevisStatus.clevis_version})</span>
            {/if}
          </p>
        {:else}
          <p class="text-text-muted font-semibold">Not Installed</p>
          <p class="text-sm text-text-secondary mt-1">Install clevis for network-bound disk encryption</p>
        {/if}
      </div>

      <!-- System Health Card -->
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 rounded-lg flex items-center justify-center text-lg
            {diagnostics?.overall_health === 'Healthy' ? 'bg-success/20 text-success' :
             diagnostics?.overall_health === 'Warning' ? 'bg-warning/20 text-warning' :
             diagnostics?.overall_health === 'Critical' ? 'bg-danger/20 text-danger' :
             'bg-surface-3 text-text-muted'}">
            {#if diagnostics?.overall_health === 'Healthy'}
              &#10003;
            {:else if diagnostics?.overall_health === 'Warning'}
              &#9888;
            {:else if diagnostics?.overall_health === 'Critical'}
              &#10007;
            {:else}
              &#128657;
            {/if}
          </div>
          <div>
            <h3 class="font-semibold text-text-primary">System Health</h3>
            <p class="text-xs text-text-muted">Recovery diagnostics</p>
          </div>
        </div>
        {#if diagnostics}
          <p class="font-semibold
            {diagnostics.overall_health === 'Healthy' ? 'text-success' :
             diagnostics.overall_health === 'Warning' ? 'text-warning' : 'text-danger'}">
            {diagnostics.overall_health}
          </p>
          <p class="text-sm text-text-secondary mt-1">
            {diagnostics.checks.length} checks, {diagnostics.issues.length} issue(s)
          </p>
          {#if diagnostics.issues.length > 0}
            <p class="text-xs text-warning mt-1">Go to Recovery page for details</p>
          {/if}
        {:else}
          <p class="text-text-muted text-sm">Could not run diagnostics</p>
        {/if}
      </div>

    </div>

    <!-- Setup Status Summary -->
    {#if setupStatus}
      <div class="bg-surface-1 border border-border rounded-lg p-5">
        <h3 class="font-semibold text-text-primary mb-4">Configuration Status</h3>
        <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
          <div class="text-center p-3 rounded-lg {setupStatus.hasLuks ? 'bg-success/10' : 'bg-surface-2'}">
            <div class="text-lg mb-1">{setupStatus.hasLuks ? '&#10003;' : '&#9711;'}</div>
            <p class="text-xs font-medium {setupStatus.hasLuks ? 'text-success' : 'text-text-muted'}">LUKS Detected</p>
          </div>
          <div class="text-center p-3 rounded-lg {setupStatus.tpm2Enrolled ? 'bg-success/10' : 'bg-surface-2'}">
            <div class="text-lg mb-1">{setupStatus.tpm2Enrolled ? '&#10003;' : '&#9711;'}</div>
            <p class="text-xs font-medium {setupStatus.tpm2Enrolled ? 'text-success' : 'text-text-muted'}">TPM2 Enrolled</p>
          </div>
          <div class="text-center p-3 rounded-lg {setupStatus.clevisBindings > 0 ? 'bg-success/10' : 'bg-surface-2'}">
            <div class="text-lg mb-1">{setupStatus.clevisBindings > 0 ? '&#10003;' : '&#9711;'}</div>
            <p class="text-xs font-medium {setupStatus.clevisBindings > 0 ? 'text-success' : 'text-text-muted'}">Clevis Bound</p>
          </div>
          <div class="text-center p-3 rounded-lg {setupStatus.hasBoot ? 'bg-success/10' : 'bg-surface-2'}">
            <div class="text-lg mb-1">{setupStatus.hasBoot ? '&#10003;' : '&#9711;'}</div>
            <p class="text-xs font-medium {setupStatus.hasBoot ? 'text-success' : 'text-text-muted'}">Boot Entries</p>
          </div>
          <div class="text-center p-3 rounded-lg {setupStatus.hasBackups ? 'bg-success/10' : 'bg-surface-2'}">
            <div class="text-lg mb-1">{setupStatus.hasBackups ? '&#10003;' : '&#9711;'}</div>
            <p class="text-xs font-medium {setupStatus.hasBackups ? 'text-success' : 'text-text-muted'}">Backups</p>
          </div>
          <div class="text-center p-3 rounded-lg {setupStatus.hasLvm ? 'bg-success/10' : 'bg-surface-2'}">
            <div class="text-lg mb-1">{setupStatus.hasLvm ? '&#10003;' : '&#9711;'}</div>
            <p class="text-xs font-medium {setupStatus.hasLvm ? 'text-success' : 'text-text-muted'}">LVM Active</p>
          </div>
        </div>

        {#if setupStatus.configuredDevices.length > 0}
          <div class="mt-4 space-y-2">
            {#each setupStatus.configuredDevices as dev}
              <div class="flex items-center gap-3 text-sm bg-surface-2 rounded-lg px-3 py-2">
                <span class="font-mono text-text-primary">{dev.device}</span>
                <span class="text-text-muted">&middot;</span>
                <span class="text-text-secondary">{dev.slotCount} key slot(s)</span>
                {#if dev.hasTpm2}
                  <span class="text-[10px] bg-success/10 text-success px-1.5 py-0.5 rounded font-bold">TPM2</span>
                {/if}
                {#if dev.hasFido2}
                  <span class="text-[10px] bg-accent/10 text-accent px-1.5 py-0.5 rounded font-bold">FIDO2</span>
                {/if}
                {#if dev.hasRecovery}
                  <span class="text-[10px] bg-warning/10 text-warning px-1.5 py-0.5 rounded font-bold">RECOVERY</span>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Privilege Warning Banner -->
    {#if overview.privilege && overview.privilege.level !== 'Root'}
      <div class="bg-warning/10 border border-warning/30 rounded-lg p-4 flex items-center gap-4">
        <span class="text-warning text-xl shrink-0">&#128274;</span>
        <div>
          <p class="text-sm font-medium text-warning">Running as user: {overview.privilege.username}</p>
          <p class="text-xs text-text-secondary mt-0.5">
            {#if overview.privilege.escalation_method === 'Pkexec'}
              Privileged operations will prompt for authentication via polkit.
            {:else if overview.privilege.escalation_method === 'Sudo'}
              Privileged operations will use sudo for authentication.
            {:else}
              No privilege escalation available. Some operations may fail. Run as root for full functionality.
            {/if}
          </p>
        </div>
      </div>
    {/if}
  {/if}
</div>
