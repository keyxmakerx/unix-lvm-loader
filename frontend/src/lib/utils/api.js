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

// ─── Backup ───
export const listBackups = (backupType = null) => invoke('list_backups', { backupType });
export const verifyBackup = (backupId) => invoke('verify_backup', { backupId });
export const verifyAllBackups = () => invoke('verify_all_backups');
export const backupLuksHeader = (device) => invoke('backup_luks_header', { device });
export const restoreLuksHeader = (device, backupId) => invoke('restore_luks_header', { device, backupId });
export const backupCrypttab = () => invoke('backup_crypttab');

// ─── Theme ───
export const listThemes = () => invoke('list_themes');
export const setActiveTheme = (themeId) => invoke('set_active_theme', { themeId });
export const getThemeThumbnail = (themeId) => invoke('get_theme_thumbnail', { themeId });
export const getThemeScreenshot = (themeId) => invoke('get_theme_screenshot', { themeId });

// ─── Logging ───
export const getRecentLogs = (count = 100) => invoke('get_recent_logs', { count });
export const getAuditLogs = (count = 100) => invoke('get_audit_logs', { count });
export const exportLogs = () => invoke('export_logs');
