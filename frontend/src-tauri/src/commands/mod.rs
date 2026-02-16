use crate::backup::{BackupManager, BackupRecord, BackupType};
use crate::boot::BootState;
use crate::clevis;
use crate::distro::DistroInfo;
use crate::logging::{self, AuditLogger, LogCategory, LogEntry, LogLevel};
use crate::luks::LuksInfo;
use crate::lvm::LvmState;
use crate::recovery;
use crate::theme::{Theme, ThemeManager};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

/// Application state shared across Tauri commands
pub struct AppState {
    pub logger: AuditLogger,
    pub backup_manager: BackupManager,
    pub theme_manager: ThemeManager,
    #[allow(dead_code)]
    pub data_dir: PathBuf,
    pub demo_mode: bool,
}

/// Wrapper for error results sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl From<crate::distro::DistroError> for AppError {
    fn from(e: crate::distro::DistroError) -> Self {
        AppError {
            code: "DISTRO_ERROR".into(),
            message: e.to_string(),
            details: None,
        }
    }
}

impl From<crate::luks::LuksError> for AppError {
    fn from(e: crate::luks::LuksError) -> Self {
        AppError {
            code: "LUKS_ERROR".into(),
            message: e.to_string(),
            details: None,
        }
    }
}

impl From<crate::lvm::LvmError> for AppError {
    fn from(e: crate::lvm::LvmError) -> Self {
        AppError {
            code: "LVM_ERROR".into(),
            message: e.to_string(),
            details: None,
        }
    }
}

impl From<crate::boot::BootError> for AppError {
    fn from(e: crate::boot::BootError) -> Self {
        AppError {
            code: "BOOT_ERROR".into(),
            message: e.to_string(),
            details: None,
        }
    }
}

impl From<crate::backup::BackupError> for AppError {
    fn from(e: crate::backup::BackupError) -> Self {
        AppError {
            code: "BACKUP_ERROR".into(),
            message: e.to_string(),
            details: None,
        }
    }
}

impl From<crate::theme::ThemeError> for AppError {
    fn from(e: crate::theme::ThemeError) -> Self {
        AppError {
            code: "THEME_ERROR".into(),
            message: e.to_string(),
            details: None,
        }
    }
}

impl From<crate::clevis::ClevisError> for AppError {
    fn from(e: crate::clevis::ClevisError) -> Self {
        AppError {
            code: "CLEVIS_ERROR".into(),
            message: e.to_string(),
            details: None,
        }
    }
}

impl From<crate::recovery::RecoveryError> for AppError {
    fn from(e: crate::recovery::RecoveryError) -> Self {
        AppError {
            code: "RECOVERY_ERROR".into(),
            message: e.to_string(),
            details: None,
        }
    }
}

// ═══════════════════════════════════════════════════════
// DISTRO COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn detect_distro(state: State<'_, Mutex<AppState>>) -> Result<DistroInfo, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    let info = crate::distro::detect()?;

    state
        .logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::DistroDetection,
            format!("Detected: {} ({})", info.pretty_name, info.id),
            Some(serde_json::to_value(&info).unwrap_or_default()),
            None,
        ))
        .ok();

    Ok(info)
}

// ═══════════════════════════════════════════════════════
// LUKS COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn scan_luks_volumes(state: State<'_, Mutex<AppState>>) -> Result<Vec<String>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(vec!["/dev/sda3".into(), "/dev/nvme0n1p3".into()]);
    }
    Ok(crate::luks::detect_luks_volumes(&state.logger)?)
}

#[tauri::command]
pub fn get_luks_info(
    device: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<LuksInfo, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(crate::demo::demo_luks_info(&device));
    }
    Ok(crate::luks::get_luks_info(&device, &state.logger)?)
}

#[tauri::command]
pub fn check_cryptenroll_available() -> bool {
    crate::luks::has_cryptenroll()
}

#[tauri::command]
pub fn enroll_tpm2(
    device: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    // Get LUKS info for safety checks
    let info = crate::luks::get_luks_info(&device, &state.logger)?;

    // SAFETY: Backup header before enrollment
    state
        .backup_manager
        .backup_luks_header(&device, &info.uuid, &state.logger)?;

    crate::luks::enroll_tpm2(&device, &info, &state.logger)?;
    Ok(())
}

#[tauri::command]
pub fn enroll_fido2(
    device: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    let info = crate::luks::get_luks_info(&device, &state.logger)?;

    // SAFETY: Backup header before enrollment
    state
        .backup_manager
        .backup_luks_header(&device, &info.uuid, &state.logger)?;

    crate::luks::enroll_fido2(&device, &info, &state.logger)?;
    Ok(())
}

#[tauri::command]
pub fn enroll_recovery_key(
    device: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<String, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    let info = crate::luks::get_luks_info(&device, &state.logger)?;

    // SAFETY: Backup header before enrollment
    state
        .backup_manager
        .backup_luks_header(&device, &info.uuid, &state.logger)?;

    Ok(crate::luks::enroll_recovery_key(&device, &info, &state.logger)?)
}

// ═══════════════════════════════════════════════════════
// LVM COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn scan_lvm(state: State<'_, Mutex<AppState>>) -> Result<LvmState, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        let overview = crate::demo::demo_system_overview();
        return Ok(overview.lvm);
    }
    Ok(crate::lvm::scan(&state.logger)?)
}

// ═══════════════════════════════════════════════════════
// BOOT COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn scan_boot_entries(
    state: State<'_, Mutex<AppState>>,
) -> Result<BootState, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        let overview = crate::demo::demo_system_overview();
        return overview.boot.ok_or_else(|| AppError {
            code: "DEMO_ERROR".into(),
            message: "No demo boot data".into(),
            details: None,
        });
    }
    let distro = crate::distro::detect()?;
    Ok(crate::boot::scan_boot_entries(&distro, &state.logger)?)
}

#[tauri::command]
pub fn set_default_boot(
    entry_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    let distro = crate::distro::detect()?;
    let boot_state = crate::boot::scan_boot_entries(&distro, &state.logger)?;

    // SAFETY: Backup boot config before changing
    state
        .backup_manager
        .backup_boot_config("/etc/default/grub", &state.logger)
        .ok(); // OK if grub config doesn't exist (systemd-boot)

    crate::boot::set_default_entry(&entry_id, &boot_state.boot_loader, &distro, &state.logger)?;
    Ok(())
}

// ═══════════════════════════════════════════════════════
// INITRAMFS COMMANDS
// ═══════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
pub struct InitramfsInfo {
    pub rebuild_needed: bool,
    pub description: String,
    pub initramfs_system: String,
}

#[tauri::command]
pub fn get_initramfs_info(
    state: State<'_, Mutex<AppState>>,
) -> Result<InitramfsInfo, AppError> {
    let _state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    let distro = crate::distro::detect()?;
    let rebuild_needed = crate::initramfs::check_rebuild_needed(&distro);
    let description = crate::initramfs::describe_rebuild(&distro);

    Ok(InitramfsInfo {
        rebuild_needed,
        description,
        initramfs_system: format!("{:?}", distro.initramfs),
    })
}

#[tauri::command]
pub fn rebuild_initramfs(
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::initramfs::RebuildResult, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    let distro = crate::distro::detect()?;

    let result = crate::initramfs::rebuild(&distro, &state.backup_manager, &state.logger)
        .map_err(|e| AppError {
            code: "INITRAMFS_ERROR".into(),
            message: e.to_string(),
            details: None,
        })?;

    Ok(result)
}

// ═══════════════════════════════════════════════════════
// BACKUP COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn list_backups(
    backup_type: Option<String>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<BackupRecord>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    if state.demo_mode {
        return Ok(crate::demo::demo_backups(backup_type.as_deref()));
    }
    match backup_type.as_deref() {
        Some("luks_header") => Ok(state.backup_manager.list_by_type(&BackupType::LuksHeader)?),
        Some("crypttab") => Ok(state.backup_manager.list_by_type(&BackupType::Crypttab)?),
        Some("initramfs") => Ok(state.backup_manager.list_by_type(&BackupType::Initramfs)?),
        Some("boot_config") => Ok(state.backup_manager.list_by_type(&BackupType::BootConfig)?),
        _ => Ok(state.backup_manager.list_all()?),
    }
}

#[tauri::command]
pub fn verify_backup(
    backup_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(true); // All demo backups are "valid"
    }
    Ok(state.backup_manager.verify_backup(&backup_id)?)
}

#[tauri::command]
pub fn verify_all_backups(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<(BackupRecord, bool)>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        let backups = crate::demo::demo_backups(None);
        return Ok(backups.into_iter().map(|b| (b, true)).collect());
    }
    Ok(state.backup_manager.verify_all()?)
}

#[tauri::command]
pub fn backup_luks_header(
    device: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<BackupRecord, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    let info = crate::luks::get_luks_info(&device, &state.logger)?;
    Ok(state
        .backup_manager
        .backup_luks_header(&device, &info.uuid, &state.logger)?)
}

#[tauri::command]
pub fn restore_luks_header(
    device: String,
    backup_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    Ok(state
        .backup_manager
        .restore_luks_header(&device, &backup_id, &state.logger)?)
}

#[tauri::command]
pub fn backup_crypttab(
    state: State<'_, Mutex<AppState>>,
) -> Result<BackupRecord, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    Ok(state.backup_manager.backup_crypttab(&state.logger)?)
}

// ═══════════════════════════════════════════════════════
// PRIVILEGE COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn get_privilege_info() -> crate::privilege::PrivilegeInfo {
    crate::privilege::detect()
}

// ═══════════════════════════════════════════════════════
// THEME COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn list_themes(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<Theme>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(crate::demo::demo_themes());
    }
    Ok(state.theme_manager.list_installed()?)
}

#[tauri::command]
pub fn set_active_theme(
    theme_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    state
        .logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::ThemeChange,
            format!("Setting active theme to: {}", theme_id),
            None,
            None,
        ))
        .ok();

    state.theme_manager.set_active_theme(&theme_id)?;
    Ok(())
}

#[tauri::command]
pub fn get_theme_thumbnail(
    theme_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<String>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    Ok(state.theme_manager.get_thumbnail_data(&theme_id)?)
}

#[tauri::command]
pub fn get_theme_screenshot(
    theme_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<String>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    Ok(state.theme_manager.get_screenshot_data(&theme_id)?)
}

#[tauri::command]
pub fn browse_repo_themes(
    repo_url: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<Theme>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    Ok(state.theme_manager.browse_repo(&repo_url)?)
}

#[tauri::command]
pub fn install_repo_theme(
    theme_id: String,
    download_url: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<Theme, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    state
        .logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::ThemeChange,
            format!("Installing theme: {} from {}", theme_id, download_url),
            None,
            None,
        ))
        .ok();

    Ok(state.theme_manager.install_from_repo(&theme_id, &download_url)?)
}

#[tauri::command]
pub fn uninstall_theme(
    theme_id: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    state
        .logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::ThemeChange,
            format!("Uninstalling theme: {}", theme_id),
            None,
            None,
        ))
        .ok();

    state.theme_manager.uninstall(&theme_id)?;
    Ok(())
}

// ═══════════════════════════════════════════════════════
// LOGGING COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn get_recent_logs(
    count: Option<usize>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<LogEntry>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(crate::demo::demo_recent_logs(count.unwrap_or(100)));
    }
    Ok(state
        .logger
        .read_recent(count.unwrap_or(100))
        .map_err(|e| AppError {
            code: "LOG_ERROR".into(),
            message: e.to_string(),
            details: None,
        })?)
}

#[tauri::command]
pub fn get_audit_logs(
    count: Option<usize>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<LogEntry>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(crate::demo::demo_audit_logs(count.unwrap_or(100)));
    }
    Ok(state
        .logger
        .read_recent_audit(count.unwrap_or(100))
        .map_err(|e| AppError {
            code: "LOG_ERROR".into(),
            message: e.to_string(),
            details: None,
        })?)
}

#[tauri::command]
pub fn export_logs(
    state: State<'_, Mutex<AppState>>,
) -> Result<String, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    Ok(state.logger.export_all().map_err(|e| AppError {
        code: "LOG_ERROR".into(),
        message: e.to_string(),
        details: None,
    })?)
}

// ═══════════════════════════════════════════════════════
// CLEVIS COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn get_clevis_status(
    state: State<'_, Mutex<AppState>>,
) -> Result<clevis::ClevisStatus, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(crate::demo::demo_clevis_status());
    }
    Ok(clevis::detect_status())
}

#[tauri::command]
pub fn verify_tang_server(
    url: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<clevis::TangAdvertisement, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    Ok(clevis::verify_tang_server(&url, &state.logger)?)
}

#[tauri::command]
pub fn list_clevis_bindings(
    device: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<clevis::ClevisBinding>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(crate::demo::demo_clevis_bindings());
    }
    Ok(clevis::list_bindings(&device, &state.logger)?)
}

#[tauri::command]
pub fn bind_tang(
    device: String,
    tang_url: String,
    thumbprint: Option<String>,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    // SAFETY: Backup LUKS header and crypttab before binding
    let info = crate::luks::get_luks_info(&device, &state.logger)?;
    state
        .backup_manager
        .backup_luks_header(&device, &info.uuid, &state.logger)?;
    state.backup_manager.backup_crypttab(&state.logger).ok(); // OK if crypttab missing

    clevis::bind_tang(
        &device,
        &tang_url,
        thumbprint.as_deref(),
        &state.logger,
    )?;
    Ok(())
}

#[tauri::command]
pub fn bind_sss(
    device: String,
    threshold: u32,
    pins: Vec<clevis::SssPin>,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    // SAFETY: Backup LUKS header before SSS binding
    let info = crate::luks::get_luks_info(&device, &state.logger)?;
    state
        .backup_manager
        .backup_luks_header(&device, &info.uuid, &state.logger)?;

    let policy = clevis::SssPolicy { threshold, pins };
    clevis::bind_sss(&device, &policy, &state.logger)?;
    Ok(())
}

#[tauri::command]
pub fn unbind_clevis(
    device: String,
    slot: u32,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    // SAFETY: Backup before unbinding
    let info = crate::luks::get_luks_info(&device, &state.logger)?;
    state
        .backup_manager
        .backup_luks_header(&device, &info.uuid, &state.logger)?;

    clevis::unbind(&device, slot, &state.logger)?;
    Ok(())
}

#[tauri::command]
pub fn get_clevis_install_instructions() -> String {
    clevis::get_install_instructions()
}

// ═══════════════════════════════════════════════════════
// RECOVERY COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn run_diagnostics(
    state: State<'_, Mutex<AppState>>,
) -> Result<recovery::DiagnosticResult, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(crate::demo::demo_diagnostics());
    }
    Ok(recovery::run_diagnostics(
        &state.backup_manager,
        &state.logger,
    ))
}

#[tauri::command]
pub fn get_recovery_backups(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<crate::backup::BackupRecord>, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    if state.demo_mode {
        return Ok(crate::demo::demo_recovery_backups());
    }
    Ok(recovery::get_recovery_backups(&state.backup_manager)?)
}

// ═══════════════════════════════════════════════════════
// SYSTEM INFO COMMAND (full dashboard data)
// ═══════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemOverview {
    pub distro: Option<DistroInfo>,
    pub luks_volumes: Vec<String>,
    pub lvm: LvmState,
    pub boot: Option<BootState>,
    pub backup_count: usize,
    pub has_cryptenroll: bool,
    pub privilege: crate::privilege::PrivilegeInfo,
}

#[tauri::command]
pub fn get_system_overview(
    state: State<'_, Mutex<AppState>>,
) -> Result<SystemOverview, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;

    if state.demo_mode {
        return Ok(crate::demo::demo_system_overview());
    }

    let distro = crate::distro::detect().ok();
    let luks_volumes = crate::luks::detect_luks_volumes(&state.logger).unwrap_or_default();
    let lvm = crate::lvm::scan(&state.logger).unwrap_or(LvmState {
        available: false,
        physical_volumes: vec![],
        volume_groups: vec![],
        logical_volumes: vec![],
    });

    let boot = distro
        .as_ref()
        .and_then(|d| crate::boot::scan_boot_entries(d, &state.logger).ok());

    let backup_count = state.backup_manager.list_all().map(|b| b.len()).unwrap_or(0);
    let has_cryptenroll = crate::luks::has_cryptenroll();
    let privilege = crate::privilege::detect();

    Ok(SystemOverview {
        distro,
        luks_volumes,
        lvm,
        boot,
        backup_count,
        has_cryptenroll,
        privilege,
    })
}

// ═══════════════════════════════════════════════════════
// DEMO MODE COMMANDS
// ═══════════════════════════════════════════════════════

#[tauri::command]
pub fn get_demo_mode(state: State<'_, Mutex<AppState>>) -> Result<bool, AppError> {
    let state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    Ok(state.demo_mode)
}

#[tauri::command]
pub fn set_demo_mode(
    enabled: bool,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Failed to acquire state lock".into(),
        details: None,
    })?;
    state.demo_mode = enabled;
    state
        .logger
        .log_operation(&crate::logging::entry(
            crate::logging::LogLevel::Info,
            crate::logging::LogCategory::UserAction,
            format!("Demo mode {}", if enabled { "enabled" } else { "disabled" }),
            None,
            None,
        ))
        .ok();
    Ok(())
}
