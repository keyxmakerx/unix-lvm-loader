import { invoke } from '@tauri-apps/api/core';

// ─── System ───
export const getSystemOverview = () => invoke('get_system_overview');
export const detectDistro = () => invoke('detect_distro');

// ─── LUKS ───
export const scanLuksVolumes = () => invoke('scan_luks_volumes');
export const getLuksInfo = (device) => invoke('get_luks_info', { device });
export const checkCryptenrollAvailable = () => invoke('check_cryptenroll_available');
export const enrollTpm2 = (device) => invoke('enroll_tpm2', { device });
export const enrollFido2 = (device) => invoke('enroll_fido2', { device });
export const enrollRecoveryKey = (device) => invoke('enroll_recovery_key', { device });

// ─── LVM ───
export const scanLvm = () => invoke('scan_lvm');

// ─── Boot ───
export const scanBootEntries = () => invoke('scan_boot_entries');
export const setDefaultBoot = (entryId) => invoke('set_default_boot', { entryId });

// ─── Privilege ───
export const getPrivilegeInfo = () => invoke('get_privilege_info');

// ─── Initramfs ───
export const getInitramfsInfo = () => invoke('get_initramfs_info');
export const rebuildInitramfs = () => invoke('rebuild_initramfs');

// ─── Backup ───
export const listBackups = (backupType = null) => invoke('list_backups', { backupType });
export const verifyBackup = (backupId) => invoke('verify_backup', { backupId });
export const verifyAllBackups = () => invoke('verify_all_backups');
export const backupLuksHeader = (device) => invoke('backup_luks_header', { device });
export const restoreLuksHeader = (device, backupId) => invoke('restore_luks_header', { device, backupId });
export const backupCrypttab = () => invoke('backup_crypttab');

// ─── Clevis/Tang ───
export const getClevisStatus = () => invoke('get_clevis_status');
export const verifyTangServer = (url) => invoke('verify_tang_server', { url });
export const listClevisBindings = (device) => invoke('list_clevis_bindings', { device });
export const bindTang = (device, tangUrl, thumbprint = null) => invoke('bind_tang', { device, tangUrl, thumbprint });
export const bindSss = (device, threshold, pins) => invoke('bind_sss', { device, threshold, pins });
export const unbindClevis = (device, slot) => invoke('unbind_clevis', { device, slot });
export const getClevisInstallInstructions = () => invoke('get_clevis_install_instructions');

// ─── Recovery ───
export const runDiagnostics = () => invoke('run_diagnostics');
export const getRecoveryBackups = () => invoke('get_recovery_backups');

// ─── Theme ───
export const listThemes = () => invoke('list_themes');
export const setActiveTheme = (themeId) => invoke('set_active_theme', { themeId });
export const getThemeThumbnail = (themeId) => invoke('get_theme_thumbnail', { themeId });
export const getThemeScreenshot = (themeId) => invoke('get_theme_screenshot', { themeId });
export const browseRepoThemes = (repoUrl) => invoke('browse_repo_themes', { repoUrl });
export const installRepoTheme = (themeId, downloadUrl) => invoke('install_repo_theme', { themeId, downloadUrl });
export const uninstallTheme = (themeId) => invoke('uninstall_theme', { themeId });

// ─── Logging ───
export const getRecentLogs = (count = 100) => invoke('get_recent_logs', { count });
export const getAuditLogs = (count = 100) => invoke('get_audit_logs', { count });
export const exportLogs = () => invoke('export_logs');

// ─── Demo Mode ───
export const getDemoMode = () => invoke('get_demo_mode');
export const setDemoMode = (enabled) => invoke('set_demo_mode', { enabled });
