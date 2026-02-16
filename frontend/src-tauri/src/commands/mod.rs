use crate::backup::{BackupManager, BackupRecord, BackupType};
use crate::boot::{BootEntry, BootState};
use crate::distro::DistroInfo;
use crate::logging::{self, AuditLogger, LogCategory, LogEntry, LogLevel};
use crate::luks::LuksInfo;
use crate::lvm::LvmState;
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
    pub data_dir: PathBuf,
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

    Ok(SystemOverview {
        distro,
        luks_volumes,
        lvm,
        boot,
        backup_count,
        has_cryptenroll,
    })
}
