mod backup;
mod boot;
mod commands;
mod distro;
mod logging;
mod luks;
mod lvm;
mod theme;

use commands::AppState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Determine data directory
    let data_dir = dirs_data_dir().join("unix-lvm-loader");
    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    // Initialize core systems
    let logger = logging::AuditLogger::new(&data_dir)
        .expect("Failed to initialize audit logger");
    let backup_manager = backup::BackupManager::new(&data_dir)
        .expect("Failed to initialize backup manager");
    let theme_manager = theme::ThemeManager::new(&data_dir)
        .expect("Failed to initialize theme manager");

    // Create default themes if fresh install
    theme_manager.create_default_themes().ok();

    // Log startup
    logger
        .log_operation(&logging::entry(
            logging::LogLevel::Info,
            logging::LogCategory::SystemCheck,
            "unix-lvm-loader starting up",
            None,
            None,
        ))
        .ok();

    let app_state = Mutex::new(AppState {
        logger,
        backup_manager,
        theme_manager,
        data_dir,
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // System
            commands::get_system_overview,
            commands::detect_distro,
            // LUKS
            commands::scan_luks_volumes,
            commands::get_luks_info,
            commands::check_cryptenroll_available,
            commands::enroll_tpm2,
            commands::enroll_fido2,
            commands::enroll_recovery_key,
            // LVM
            commands::scan_lvm,
            // Boot
            commands::scan_boot_entries,
            commands::set_default_boot,
            // Backup
            commands::list_backups,
            commands::verify_backup,
            commands::verify_all_backups,
            commands::backup_luks_header,
            commands::restore_luks_header,
            commands::backup_crypttab,
            // Theme
            commands::list_themes,
            commands::set_active_theme,
            commands::get_theme_thumbnail,
            commands::get_theme_screenshot,
            // Logging
            commands::get_recent_logs,
            commands::get_audit_logs,
            commands::export_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Get the standard data directory for the current user
fn dirs_data_dir() -> std::path::PathBuf {
    // Try XDG_DATA_HOME first, then ~/.local/share
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return std::path::PathBuf::from(xdg);
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home).join(".local").join("share");
    }
    // Last resort
    std::path::PathBuf::from("/tmp")
}
