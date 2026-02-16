use crate::backup::BackupManager;
use crate::distro::{DistroInfo, InitramfsSystem};
use crate::logging::{self, AuditLogger, LogCategory, LogLevel};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InitramfsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("No initramfs system detected")]
    NoInitramfsSystem,
    #[error("Backup failed before rebuild: {0}")]
    BackupFailed(String),
}

/// Result of an initramfs rebuild operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildResult {
    pub success: bool,
    pub initramfs_system: String,
    pub command_run: String,
    pub stdout: String,
    pub stderr: String,
    pub backup_id: Option<String>,
}

/// Find the current initramfs image path for the running kernel
fn find_current_initramfs(distro: &DistroInfo) -> Option<String> {
    // Get current kernel version
    let uname = Command::new("uname").arg("-r").output().ok()?;
    let kernel_version = String::from_utf8_lossy(&uname.stdout).trim().to_string();

    // Check per-distro locations
    let candidates = match distro.initramfs {
        InitramfsSystem::Dracut => vec![
            format!("/boot/initramfs-{}.img", kernel_version),
            format!("/boot/initrd.img-{}", kernel_version),
        ],
        InitramfsSystem::Mkinitcpio => vec![
            "/boot/initramfs-linux.img".to_string(),
            format!("/boot/initramfs-{}.img", kernel_version),
        ],
        InitramfsSystem::InitramfsTools => vec![
            format!("/boot/initrd.img-{}", kernel_version),
            format!("/boot/initramfs-{}", kernel_version),
        ],
        InitramfsSystem::Unknown => vec![
            format!("/boot/initramfs-{}.img", kernel_version),
            format!("/boot/initrd.img-{}", kernel_version),
        ],
    };

    candidates.into_iter().find(|p| Path::new(p).exists())
}

/// Get the rebuild command for the current distro
fn get_rebuild_command(distro: &DistroInfo) -> Result<(Vec<String>, String), InitramfsError> {
    let cmd = crate::distro::initramfs_rebuild_command(distro);
    if cmd.is_empty() {
        return Err(InitramfsError::NoInitramfsSystem);
    }

    let display = cmd.join(" ");
    Ok((cmd, display))
}

/// Rebuild the initramfs with full safety protocol:
/// 1. Find current initramfs
/// 2. Backup it
/// 3. Execute rebuild
/// 4. Verify new initramfs exists
/// 5. Log everything
pub fn rebuild(
    distro: &DistroInfo,
    backup_manager: &BackupManager,
    logger: &AuditLogger,
) -> Result<RebuildResult, InitramfsError> {
    logger
        .log_audit(&logging::entry(
            LogLevel::Warning,
            LogCategory::InitramfsRebuild,
            format!(
                "Starting initramfs rebuild (system: {:?}, distro: {})",
                distro.initramfs, distro.id
            ),
            Some(serde_json::json!({
                "initramfs_system": format!("{:?}", distro.initramfs),
                "distro_id": distro.id,
                "is_immutable": distro.is_immutable,
            })),
            None,
        ))
        .ok();

    // Step 1: Find current initramfs
    let initramfs_path = find_current_initramfs(distro);
    let mut backup_id = None;

    // Step 2: Backup current initramfs if it exists
    if let Some(ref path) = initramfs_path {
        match backup_manager.backup_initramfs(path, logger) {
            Ok(record) => {
                backup_id = Some(record.id);
                logger
                    .log_operation(&logging::entry(
                        LogLevel::Info,
                        LogCategory::BackupCreated,
                        format!("Initramfs backed up before rebuild: {}", path),
                        None,
                        Some(format!("cp {} {}", record.backup_path, path)),
                    ))
                    .ok();
            }
            Err(e) => {
                // Log warning but don't abort — the rebuild might still be needed
                logger
                    .log_operation(&logging::entry(
                        LogLevel::Warning,
                        LogCategory::BackupCreated,
                        format!("Could not backup initramfs before rebuild: {}", e),
                        None,
                        None,
                    ))
                    .ok();
            }
        }
    }

    // Step 3: Get and execute rebuild command
    let (cmd, cmd_display) = get_rebuild_command(distro)?;

    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::InitramfsRebuild,
            format!("Executing: {}", cmd_display),
            None,
            None,
        ))
        .ok();

    let output = Command::new(&cmd[0])
        .args(&cmd[1..])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        logger
            .log_audit(&logging::entry(
                LogLevel::Error,
                LogCategory::InitramfsRebuild,
                format!("Initramfs rebuild FAILED: {}", stderr),
                Some(serde_json::json!({
                    "command": cmd_display,
                    "exit_code": output.status.code(),
                    "stderr": &stderr[..stderr.len().min(1000)],
                })),
                initramfs_path.as_ref().map(|p| {
                    format!(
                        "If boot fails, restore from backup: cp <backup> {}",
                        p
                    )
                }),
            ))
            .ok();

        return Ok(RebuildResult {
            success: false,
            initramfs_system: format!("{:?}", distro.initramfs),
            command_run: cmd_display,
            stdout,
            stderr,
            backup_id,
        });
    }

    // Step 4: Verify new initramfs exists
    if let Some(ref path) = initramfs_path {
        if !Path::new(path).exists() {
            logger
                .log_audit(&logging::entry(
                    LogLevel::Error,
                    LogCategory::InitramfsRebuild,
                    format!(
                        "Rebuild command succeeded but initramfs not found at {}",
                        path
                    ),
                    None,
                    None,
                ))
                .ok();
        }
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Info,
            LogCategory::InitramfsRebuild,
            "Initramfs rebuild completed successfully",
            Some(serde_json::json!({
                "command": cmd_display,
                "backup_id": backup_id,
            })),
            backup_id.as_ref().map(|id| {
                format!("To restore previous initramfs, use backup ID: {}", id)
            }),
        ))
        .ok();

    Ok(RebuildResult {
        success: true,
        initramfs_system: format!("{:?}", distro.initramfs),
        command_run: cmd_display,
        stdout,
        stderr,
        backup_id,
    })
}

/// Check if an initramfs rebuild is needed (e.g., after TPM2 enrollment)
pub fn check_rebuild_needed(distro: &DistroInfo) -> bool {
    // For TPM2/FIDO2 to work at boot, the initramfs must include
    // the appropriate modules. After enrollment, a rebuild is almost
    // always needed.
    matches!(
        distro.initramfs,
        InitramfsSystem::Dracut | InitramfsSystem::Mkinitcpio | InitramfsSystem::InitramfsTools
    )
}

/// Get a human-readable description of what the rebuild will do
pub fn describe_rebuild(distro: &DistroInfo) -> String {
    match distro.initramfs {
        InitramfsSystem::Dracut => {
            if distro.is_immutable {
                "Enable initramfs regeneration via rpm-ostree. This will take effect on next boot.".to_string()
            } else {
                "Rebuild initramfs using dracut. This regenerates the boot image with current system configuration.".to_string()
            }
        }
        InitramfsSystem::Mkinitcpio => {
            "Rebuild all initramfs presets using mkinitcpio. This regenerates boot images for all installed kernels.".to_string()
        }
        InitramfsSystem::InitramfsTools => {
            "Update initramfs using update-initramfs. This regenerates the boot image for the current kernel.".to_string()
        }
        InitramfsSystem::Unknown => {
            "Unknown initramfs system. Cannot determine rebuild command.".to_string()
        }
    }
}
