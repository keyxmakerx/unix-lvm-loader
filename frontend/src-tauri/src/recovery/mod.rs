use crate::backup::{BackupManager, BackupRecord, BackupType};
use crate::logging::{self, AuditLogger, LogCategory, LogLevel};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RecoveryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Recovery action failed: {0}")]
    ActionFailed(String),
    #[allow(dead_code)]
    #[error("No recovery possible: {0}")]
    NoRecovery(String),
}

/// A detected issue that may require recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub checks: Vec<HealthCheck>,
    pub issues: Vec<Issue>,
    pub overall_health: OverallHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OverallHealth {
    Healthy,
    Warning,
    Critical,
}

/// A single health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub description: String,
    pub status: CheckStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warning,
    Fail,
    Skipped,
}

/// A detected issue with suggested recovery actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub severity: IssueSeverity,
    pub title: String,
    pub description: String,
    pub recovery_actions: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// A recovery action the user can take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    pub id: String,
    pub label: String,
    pub description: String,
    pub command_preview: Option<String>,
    pub requires_confirmation: bool,
    pub destructive: bool,
}

/// Run a full diagnostic scan
pub fn run_diagnostics(
    backup_manager: &BackupManager,
    logger: &AuditLogger,
) -> DiagnosticResult {
    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::Recovery,
            "Starting system diagnostic scan",
            None,
            None,
        ))
        .ok();

    let mut checks = Vec::new();
    let mut issues = Vec::new();

    // Check 1: cryptsetup available
    checks.push(check_cryptsetup());

    // Check 2: LUKS volumes accessible
    checks.push(check_luks_volumes());

    // Check 3: crypttab consistency
    let crypttab_check = check_crypttab();
    if crypttab_check.status == CheckStatus::Fail {
        issues.push(Issue {
            id: "crypttab-mismatch".into(),
            severity: IssueSeverity::High,
            title: "Crypttab configuration issue".into(),
            description: crypttab_check.detail.clone().unwrap_or_default(),
            recovery_actions: vec![
                RecoveryAction {
                    id: "restore-crypttab".into(),
                    label: "Restore crypttab from backup".into(),
                    description: "Restore /etc/crypttab from the most recent backup".into(),
                    command_preview: Some("cp <backup>/crypttab /etc/crypttab".into()),
                    requires_confirmation: true,
                    destructive: false,
                },
            ],
        });
    }
    checks.push(crypttab_check);

    // Check 4: initramfs exists for current kernel
    let initramfs_check = check_initramfs();
    if initramfs_check.status == CheckStatus::Fail {
        issues.push(Issue {
            id: "initramfs-missing".into(),
            severity: IssueSeverity::Critical,
            title: "Initramfs missing for current kernel".into(),
            description: "The boot image for your current kernel is missing. The system may not boot correctly after a restart.".into(),
            recovery_actions: vec![
                RecoveryAction {
                    id: "rebuild-initramfs".into(),
                    label: "Rebuild initramfs".into(),
                    description: "Regenerate the boot image for the current kernel".into(),
                    command_preview: None, // Will be distro-specific
                    requires_confirmation: true,
                    destructive: false,
                },
                RecoveryAction {
                    id: "restore-initramfs".into(),
                    label: "Restore initramfs from backup".into(),
                    description: "Restore a previously backed-up initramfs image".into(),
                    command_preview: None,
                    requires_confirmation: true,
                    destructive: false,
                },
            ],
        });
    }
    checks.push(initramfs_check);

    // Check 5: Boot loader config
    let boot_check = check_boot_config();
    if boot_check.status == CheckStatus::Fail {
        issues.push(Issue {
            id: "boot-config-issue".into(),
            severity: IssueSeverity::High,
            title: "Boot configuration issue".into(),
            description: boot_check.detail.clone().unwrap_or_default(),
            recovery_actions: vec![
                RecoveryAction {
                    id: "restore-boot-config".into(),
                    label: "Restore boot configuration from backup".into(),
                    description: "Restore GRUB/systemd-boot config from a previous backup".into(),
                    command_preview: None,
                    requires_confirmation: true,
                    destructive: false,
                },
                RecoveryAction {
                    id: "regenerate-grub".into(),
                    label: "Regenerate GRUB configuration".into(),
                    description: "Run grub-mkconfig to regenerate boot configuration".into(),
                    command_preview: Some("grub-mkconfig -o /boot/grub/grub.cfg".into()),
                    requires_confirmation: true,
                    destructive: false,
                },
            ],
        });
    }
    checks.push(boot_check);

    // Check 6: Backup health
    let backup_check = check_backups(backup_manager);
    if backup_check.status == CheckStatus::Warning || backup_check.status == CheckStatus::Fail {
        issues.push(Issue {
            id: "backup-health".into(),
            severity: if backup_check.status == CheckStatus::Fail {
                IssueSeverity::Medium
            } else {
                IssueSeverity::Low
            },
            title: "Backup health concern".into(),
            description: backup_check.detail.clone().unwrap_or_default(),
            recovery_actions: vec![
                RecoveryAction {
                    id: "create-backups".into(),
                    label: "Create fresh backups".into(),
                    description: "Backup current LUKS headers, crypttab, and boot config".into(),
                    command_preview: None,
                    requires_confirmation: false,
                    destructive: false,
                },
            ],
        });
    }
    checks.push(backup_check);

    // Check 7: systemd-cryptenroll health
    checks.push(check_cryptenroll());

    // Check 8: LVM health
    let lvm_check = check_lvm();
    if lvm_check.status == CheckStatus::Fail {
        issues.push(Issue {
            id: "lvm-issue".into(),
            severity: IssueSeverity::High,
            title: "LVM issue detected".into(),
            description: lvm_check.detail.clone().unwrap_or_default(),
            recovery_actions: vec![
                RecoveryAction {
                    id: "activate-lvm".into(),
                    label: "Activate LVM volumes".into(),
                    description: "Try to activate all volume groups".into(),
                    command_preview: Some("vgchange -ay".into()),
                    requires_confirmation: true,
                    destructive: false,
                },
            ],
        });
    }
    checks.push(lvm_check);

    // Determine overall health
    let overall_health = if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        OverallHealth::Critical
    } else if checks.iter().any(|c| c.status == CheckStatus::Warning) {
        OverallHealth::Warning
    } else {
        OverallHealth::Healthy
    };

    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::Recovery,
            format!(
                "Diagnostic complete: {:?} ({} checks, {} issues)",
                overall_health,
                checks.len(),
                issues.len()
            ),
            Some(serde_json::json!({
                "health": format!("{:?}", overall_health),
                "checks": checks.len(),
                "issues": issues.len(),
            })),
            None,
        ))
        .ok();

    DiagnosticResult {
        checks,
        issues,
        overall_health,
    }
}

/// Get available backups that could be used for recovery
pub fn get_recovery_backups(
    backup_manager: &BackupManager,
) -> Result<Vec<BackupRecord>, RecoveryError> {
    backup_manager
        .list_all()
        .map_err(|e| RecoveryError::ActionFailed(e.to_string()))
}

// ── Individual Health Checks ──

fn check_cryptsetup() -> HealthCheck {
    let status = Command::new("cryptsetup")
        .arg("--version")
        .output()
        .map(|o| {
            if o.status.success() {
                let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
                (CheckStatus::Pass, Some(version))
            } else {
                (CheckStatus::Fail, Some("cryptsetup not working".into()))
            }
        })
        .unwrap_or((CheckStatus::Fail, Some("cryptsetup not found".into())));

    HealthCheck {
        name: "cryptsetup".into(),
        description: "Verify cryptsetup is installed and functional".into(),
        status: status.0,
        detail: status.1,
    }
}

fn check_luks_volumes() -> HealthCheck {
    let output = Command::new("blkid")
        .args(["-t", "TYPE=crypto_LUKS", "-o", "device"])
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let count = stdout.lines().filter(|l| !l.trim().is_empty()).count();
            if count > 0 {
                HealthCheck {
                    name: "LUKS volumes".into(),
                    description: "Detect encrypted volumes".into(),
                    status: CheckStatus::Pass,
                    detail: Some(format!("{} LUKS volume(s) detected", count)),
                }
            } else {
                HealthCheck {
                    name: "LUKS volumes".into(),
                    description: "Detect encrypted volumes".into(),
                    status: CheckStatus::Warning,
                    detail: Some("No LUKS volumes detected".into()),
                }
            }
        }
        Err(e) => HealthCheck {
            name: "LUKS volumes".into(),
            description: "Detect encrypted volumes".into(),
            status: CheckStatus::Fail,
            detail: Some(format!("blkid failed: {}", e)),
        },
    }
}

fn check_crypttab() -> HealthCheck {
    let crypttab_path = Path::new("/etc/crypttab");
    if !crypttab_path.exists() {
        return HealthCheck {
            name: "crypttab".into(),
            description: "Verify /etc/crypttab exists and is valid".into(),
            status: CheckStatus::Warning,
            detail: Some("/etc/crypttab does not exist".into()),
        };
    }

    match std::fs::read_to_string(crypttab_path) {
        Ok(content) => {
            let entries = content
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                .count();

            if entries == 0 {
                HealthCheck {
                    name: "crypttab".into(),
                    description: "Verify /etc/crypttab exists and is valid".into(),
                    status: CheckStatus::Warning,
                    detail: Some("crypttab exists but has no entries".into()),
                }
            } else {
                // Check that referenced devices exist
                let mut missing = Vec::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let device = parts[1];
                        // UUID= references are validated differently
                        if device.starts_with('/') && !Path::new(device).exists() {
                            missing.push(device.to_string());
                        }
                    }
                }

                if missing.is_empty() {
                    HealthCheck {
                        name: "crypttab".into(),
                        description: "Verify /etc/crypttab exists and is valid".into(),
                        status: CheckStatus::Pass,
                        detail: Some(format!("{} crypttab entries OK", entries)),
                    }
                } else {
                    HealthCheck {
                        name: "crypttab".into(),
                        description: "Verify /etc/crypttab exists and is valid".into(),
                        status: CheckStatus::Fail,
                        detail: Some(format!(
                            "crypttab references missing devices: {}",
                            missing.join(", ")
                        )),
                    }
                }
            }
        }
        Err(e) => HealthCheck {
            name: "crypttab".into(),
            description: "Verify /etc/crypttab exists and is valid".into(),
            status: CheckStatus::Fail,
            detail: Some(format!("Cannot read crypttab: {}", e)),
        },
    }
}

fn check_initramfs() -> HealthCheck {
    // Get current kernel version
    let uname = match Command::new("uname").arg("-r").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => {
            return HealthCheck {
                name: "initramfs".into(),
                description: "Verify initramfs exists for current kernel".into(),
                status: CheckStatus::Skipped,
                detail: Some("Could not determine kernel version".into()),
            };
        }
    };

    // Check common initramfs locations
    let candidates = vec![
        format!("/boot/initramfs-{}.img", uname),
        format!("/boot/initrd.img-{}", uname),
        "/boot/initramfs-linux.img".to_string(),
    ];

    let found = candidates.iter().find(|p| Path::new(p).exists());

    match found {
        Some(path) => {
            // Check size — a 0-byte initramfs is effectively missing
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            if size < 1024 {
                HealthCheck {
                    name: "initramfs".into(),
                    description: "Verify initramfs exists for current kernel".into(),
                    status: CheckStatus::Fail,
                    detail: Some(format!("{} exists but is suspiciously small ({} bytes)", path, size)),
                }
            } else {
                HealthCheck {
                    name: "initramfs".into(),
                    description: "Verify initramfs exists for current kernel".into(),
                    status: CheckStatus::Pass,
                    detail: Some(format!("{} ({:.1} MB)", path, size as f64 / 1_048_576.0)),
                }
            }
        }
        None => HealthCheck {
            name: "initramfs".into(),
            description: "Verify initramfs exists for current kernel".into(),
            status: CheckStatus::Fail,
            detail: Some(format!(
                "No initramfs found for kernel {}. Checked: {}",
                uname,
                candidates.join(", ")
            )),
        },
    }
}

fn check_boot_config() -> HealthCheck {
    // Check for GRUB
    let grub_cfg = Path::new("/boot/grub/grub.cfg");
    let grub_cfg_alt = Path::new("/boot/grub2/grub.cfg");

    // Check for systemd-boot
    let sd_boot = Path::new("/boot/loader/loader.conf");

    if grub_cfg.exists() || grub_cfg_alt.exists() {
        let cfg_path = if grub_cfg.exists() { grub_cfg } else { grub_cfg_alt };
        match std::fs::read_to_string(cfg_path) {
            Ok(content) => {
                let entry_count = content.matches("menuentry ").count()
                    + content.matches("menuentry'").count();
                if entry_count > 0 {
                    HealthCheck {
                        name: "Boot config".into(),
                        description: "Verify boot loader configuration".into(),
                        status: CheckStatus::Pass,
                        detail: Some(format!("GRUB config OK ({} entries)", entry_count)),
                    }
                } else {
                    HealthCheck {
                        name: "Boot config".into(),
                        description: "Verify boot loader configuration".into(),
                        status: CheckStatus::Warning,
                        detail: Some("GRUB config exists but no menu entries found".into()),
                    }
                }
            }
            Err(e) => HealthCheck {
                name: "Boot config".into(),
                description: "Verify boot loader configuration".into(),
                status: CheckStatus::Fail,
                detail: Some(format!("Cannot read GRUB config: {}", e)),
            },
        }
    } else if sd_boot.exists() {
        // Count systemd-boot entries
        let entries_dir = Path::new("/boot/loader/entries");
        let entry_count = if entries_dir.exists() {
            std::fs::read_dir(entries_dir)
                .map(|rd| rd.filter(|e| {
                    e.as_ref()
                        .map(|e| e.path().extension().map(|ext| ext == "conf").unwrap_or(false))
                        .unwrap_or(false)
                }).count())
                .unwrap_or(0)
        } else {
            0
        };

        HealthCheck {
            name: "Boot config".into(),
            description: "Verify boot loader configuration".into(),
            status: if entry_count > 0 { CheckStatus::Pass } else { CheckStatus::Warning },
            detail: Some(format!("systemd-boot ({} entries)", entry_count)),
        }
    } else {
        HealthCheck {
            name: "Boot config".into(),
            description: "Verify boot loader configuration".into(),
            status: CheckStatus::Warning,
            detail: Some("No recognized boot loader config found".into()),
        }
    }
}

fn check_backups(backup_manager: &BackupManager) -> HealthCheck {
    match backup_manager.list_all() {
        Ok(records) => {
            if records.is_empty() {
                HealthCheck {
                    name: "Backups".into(),
                    description: "Verify backup health".into(),
                    status: CheckStatus::Warning,
                    detail: Some("No backups exist. It is strongly recommended to create backups before making changes.".into()),
                }
            } else {
                // Check for LUKS header backups specifically
                let header_backups = records
                    .iter()
                    .filter(|r| r.backup_type == BackupType::LuksHeader)
                    .count();

                let detail = format!(
                    "{} backup(s) ({} LUKS header backups)",
                    records.len(),
                    header_backups
                );

                if header_backups == 0 {
                    HealthCheck {
                        name: "Backups".into(),
                        description: "Verify backup health".into(),
                        status: CheckStatus::Warning,
                        detail: Some(format!("{} — no LUKS header backups!", detail)),
                    }
                } else {
                    HealthCheck {
                        name: "Backups".into(),
                        description: "Verify backup health".into(),
                        status: CheckStatus::Pass,
                        detail: Some(detail),
                    }
                }
            }
        }
        Err(e) => HealthCheck {
            name: "Backups".into(),
            description: "Verify backup health".into(),
            status: CheckStatus::Fail,
            detail: Some(format!("Cannot read backups: {}", e)),
        },
    }
}

fn check_cryptenroll() -> HealthCheck {
    let output = Command::new("systemd-cryptenroll")
        .arg("--version")
        .output();

    match output {
        Ok(o) if o.status.success() => HealthCheck {
            name: "systemd-cryptenroll".into(),
            description: "Verify systemd-cryptenroll for TPM2/FIDO2".into(),
            status: CheckStatus::Pass,
            detail: Some(
                String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("OK").to_string(),
            ),
        },
        _ => HealthCheck {
            name: "systemd-cryptenroll".into(),
            description: "Verify systemd-cryptenroll for TPM2/FIDO2".into(),
            status: CheckStatus::Warning,
            detail: Some("Not available — TPM2/FIDO2 enrollment will not work".into()),
        },
    }
}

fn check_lvm() -> HealthCheck {
    let output = Command::new("vgs")
        .args(["--noheadings", "--nosuffix", "-o", "vg_name"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let vg_count = stdout.lines().filter(|l| !l.trim().is_empty()).count();
            HealthCheck {
                name: "LVM".into(),
                description: "Verify LVM volume groups".into(),
                status: CheckStatus::Pass,
                detail: Some(format!("{} volume group(s) active", vg_count)),
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            HealthCheck {
                name: "LVM".into(),
                description: "Verify LVM volume groups".into(),
                status: CheckStatus::Fail,
                detail: Some(format!("vgs failed: {}", stderr.trim())),
            }
        }
        Err(_) => HealthCheck {
            name: "LVM".into(),
            description: "Verify LVM volume groups".into(),
            status: CheckStatus::Warning,
            detail: Some("LVM tools not found".into()),
        },
    }
}
