use crate::logging::{self, AuditLogger, LogCategory, LogLevel};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Backup verification failed: expected {expected}, got {actual}")]
    VerificationFailed { expected: String, actual: String },
    #[error("Backup not found: {0}")]
    NotFound(String),
    #[allow(dead_code)]
    #[error("No backup directory configured")]
    NoBackupDir,
}

/// Type of backup
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupType {
    LuksHeader,
    Crypttab,
    Initramfs,
    BootConfig,
    KeySlotSnapshot,
}

/// Metadata about a stored backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: String,
    pub backup_type: BackupType,
    pub created_at: String,
    pub source_path: String,
    pub backup_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub description: String,
    /// For LUKS headers: which device UUID this belongs to
    pub device_uuid: Option<String>,
}

/// Manages the 5-layer backup system
pub struct BackupManager {
    #[allow(dead_code)]
    base_dir: PathBuf,
    headers_dir: PathBuf,
    crypttab_dir: PathBuf,
    initramfs_dir: PathBuf,
    boot_config_dir: PathBuf,
    manifest_path: PathBuf,
}

impl BackupManager {
    pub fn new(base_dir: &Path) -> Result<Self, BackupError> {
        let backups_dir = base_dir.join("backups");
        let headers_dir = backups_dir.join("headers");
        let crypttab_dir = backups_dir.join("crypttab");
        let initramfs_dir = backups_dir.join("initramfs");
        let boot_config_dir = backups_dir.join("boot-config");
        let manifest_path = backups_dir.join("manifest.json");

        fs::create_dir_all(&headers_dir)?;
        fs::create_dir_all(&crypttab_dir)?;
        fs::create_dir_all(&initramfs_dir)?;
        fs::create_dir_all(&boot_config_dir)?;

        Ok(Self {
            base_dir: backups_dir,
            headers_dir,
            crypttab_dir,
            initramfs_dir,
            boot_config_dir,
            manifest_path,
        })
    }

    /// Generate a timestamped filename
    fn timestamped_name(prefix: &str, ext: &str) -> String {
        let ts = Utc::now().format("%Y%m%d-%H%M%S");
        format!("{}-{}.{}", prefix, ts, ext)
    }

    /// Compute SHA-256 of a file
    fn sha256_file(path: &Path) -> Result<String, BackupError> {
        let data = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Save a backup record to the manifest
    fn save_record(&self, record: &BackupRecord) -> Result<(), BackupError> {
        let mut records = self.list_all()?;
        records.push(record.clone());
        let json = serde_json::to_string_pretty(&records)
            .map_err(|e| BackupError::CommandFailed(e.to_string()))?;
        fs::write(&self.manifest_path, json)?;
        Ok(())
    }

    /// List all backup records
    pub fn list_all(&self) -> Result<Vec<BackupRecord>, BackupError> {
        if !self.manifest_path.exists() {
            return Ok(vec![]);
        }
        let content = fs::read_to_string(&self.manifest_path)?;
        let records: Vec<BackupRecord> = serde_json::from_str(&content)
            .map_err(|e| BackupError::CommandFailed(e.to_string()))?;
        Ok(records)
    }

    /// List backups of a specific type
    pub fn list_by_type(&self, backup_type: &BackupType) -> Result<Vec<BackupRecord>, BackupError> {
        Ok(self
            .list_all()?
            .into_iter()
            .filter(|r| &r.backup_type == backup_type)
            .collect())
    }

    // ── Layer 1: LUKS Header Backup ──

    /// Backup a LUKS header using cryptsetup luksHeaderBackup
    /// This is the MOST CRITICAL backup — without it, data can be permanently lost
    pub fn backup_luks_header(
        &self,
        device: &str,
        device_uuid: &str,
        logger: &AuditLogger,
    ) -> Result<BackupRecord, BackupError> {
        let filename = Self::timestamped_name(
            &format!("luks-header-{}", &device_uuid[..8]),
            "img",
        );
        let backup_path = self.headers_dir.join(&filename);

        logger
            .log_audit(&logging::entry(
                LogLevel::Info,
                LogCategory::BackupCreated,
                format!("Starting LUKS header backup for {} ({})", device, device_uuid),
                Some(serde_json::json!({
                    "device": device,
                    "uuid": device_uuid,
                    "backup_path": backup_path.display().to_string(),
                })),
                None,
            ))
            .ok();

        // Execute cryptsetup luksHeaderBackup
        let output = Command::new("cryptsetup")
            .args(["luksHeaderBackup", device, "--header-backup-file"])
            .arg(&backup_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logger
                .log_audit(&logging::entry(
                    LogLevel::Error,
                    LogCategory::BackupCreated,
                    format!("LUKS header backup FAILED for {}: {}", device, stderr),
                    None,
                    None,
                ))
                .ok();
            return Err(BackupError::CommandFailed(format!(
                "cryptsetup luksHeaderBackup failed: {}",
                stderr
            )));
        }

        // Verify backup file exists and compute checksum
        let sha256 = Self::sha256_file(&backup_path)?;
        let size = fs::metadata(&backup_path)?.len();

        // Write checksum sidecar file
        let checksum_path = backup_path.with_extension("img.sha256");
        fs::write(&checksum_path, format!("{}  {}\n", sha256, filename))?;

        let record = BackupRecord {
            id: uuid::Uuid::new_v4().to_string(),
            backup_type: BackupType::LuksHeader,
            created_at: Utc::now().to_rfc3339(),
            source_path: device.to_string(),
            backup_path: backup_path.display().to_string(),
            sha256: sha256.clone(),
            size_bytes: size,
            description: format!("LUKS header backup for {} (UUID: {})", device, device_uuid),
            device_uuid: Some(device_uuid.to_string()),
        };

        self.save_record(&record)?;

        logger
            .log_audit(&logging::entry(
                LogLevel::Info,
                LogCategory::BackupCreated,
                format!(
                    "LUKS header backup completed: {} ({} bytes, SHA256: {})",
                    backup_path.display(),
                    size,
                    &sha256[..16]
                ),
                Some(serde_json::json!({
                    "backup_id": record.id,
                    "sha256": sha256,
                    "size_bytes": size,
                })),
                Some(format!(
                    "cryptsetup luksHeaderRestore {} --header-backup-file {}",
                    device,
                    backup_path.display()
                )),
            ))
            .ok();

        Ok(record)
    }

    /// Restore a LUKS header from backup
    pub fn restore_luks_header(
        &self,
        device: &str,
        backup_id: &str,
        logger: &AuditLogger,
    ) -> Result<(), BackupError> {
        let records = self.list_all()?;
        let record = records
            .iter()
            .find(|r| r.id == backup_id && r.backup_type == BackupType::LuksHeader)
            .ok_or_else(|| BackupError::NotFound(backup_id.to_string()))?;

        // Verify backup integrity before restore
        let current_sha256 = Self::sha256_file(Path::new(&record.backup_path))?;
        if current_sha256 != record.sha256 {
            return Err(BackupError::VerificationFailed {
                expected: record.sha256.clone(),
                actual: current_sha256,
            });
        }

        logger
            .log_audit(&logging::entry(
                LogLevel::Warning,
                LogCategory::BackupRestored,
                format!(
                    "RESTORING LUKS header for {} from backup {}",
                    device, backup_id
                ),
                Some(serde_json::json!({
                    "device": device,
                    "backup_id": backup_id,
                    "backup_path": record.backup_path,
                })),
                None,
            ))
            .ok();

        // Before restoring, backup the CURRENT header as safety net
        // This way we can undo the restore if needed
        let safety_filename = Self::timestamped_name("pre-restore-safety", "img");
        let safety_path = self.headers_dir.join(&safety_filename);
        let safety_output = Command::new("cryptsetup")
            .args(["luksHeaderBackup", device, "--header-backup-file"])
            .arg(&safety_path)
            .output()?;

        if safety_output.status.success() {
            logger
                .log_operation(&logging::entry(
                    LogLevel::Info,
                    LogCategory::BackupCreated,
                    format!("Safety backup of current header saved to {}", safety_path.display()),
                    None,
                    None,
                ))
                .ok();
        }

        // Execute restore
        let output = Command::new("cryptsetup")
            .args(["luksHeaderRestore", device, "--header-backup-file"])
            .arg(&record.backup_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackupError::CommandFailed(format!(
                "luksHeaderRestore failed: {}",
                stderr
            )));
        }

        logger
            .log_audit(&logging::entry(
                LogLevel::Info,
                LogCategory::BackupRestored,
                format!("LUKS header restored successfully for {}", device),
                None,
                Some(format!(
                    "To undo: cryptsetup luksHeaderRestore {} --header-backup-file {}",
                    device,
                    safety_path.display()
                )),
            ))
            .ok();

        Ok(())
    }

    // ── Layer 2: crypttab Backup ──

    pub fn backup_crypttab(&self, logger: &AuditLogger) -> Result<BackupRecord, BackupError> {
        let source = Path::new("/etc/crypttab");
        if !source.exists() {
            return Err(BackupError::NotFound("/etc/crypttab".into()));
        }

        let filename = Self::timestamped_name("crypttab", "bak");
        let backup_path = self.crypttab_dir.join(&filename);

        fs::copy(source, &backup_path)?;
        let sha256 = Self::sha256_file(&backup_path)?;
        let size = fs::metadata(&backup_path)?.len();

        let record = BackupRecord {
            id: uuid::Uuid::new_v4().to_string(),
            backup_type: BackupType::Crypttab,
            created_at: Utc::now().to_rfc3339(),
            source_path: "/etc/crypttab".into(),
            backup_path: backup_path.display().to_string(),
            sha256,
            size_bytes: size,
            description: "Backup of /etc/crypttab".into(),
            device_uuid: None,
        };

        self.save_record(&record)?;

        logger
            .log_audit(&logging::entry(
                LogLevel::Info,
                LogCategory::BackupCreated,
                format!("crypttab backed up to {}", backup_path.display()),
                None,
                Some(format!("cp {} /etc/crypttab", backup_path.display())),
            ))
            .ok();

        Ok(record)
    }

    /// Restore crypttab from a backup
    #[allow(dead_code)]
    pub fn restore_crypttab(
        &self,
        backup_id: &str,
        logger: &AuditLogger,
    ) -> Result<(), BackupError> {
        let records = self.list_all()?;
        let record = records
            .iter()
            .find(|r| r.id == backup_id && r.backup_type == BackupType::Crypttab)
            .ok_or_else(|| BackupError::NotFound(backup_id.to_string()))?;

        // Verify integrity
        let current_sha256 = Self::sha256_file(Path::new(&record.backup_path))?;
        if current_sha256 != record.sha256 {
            return Err(BackupError::VerificationFailed {
                expected: record.sha256.clone(),
                actual: current_sha256,
            });
        }

        // Backup current crypttab before overwriting
        self.backup_crypttab(logger)?;

        fs::copy(&record.backup_path, "/etc/crypttab")?;

        logger
            .log_audit(&logging::entry(
                LogLevel::Warning,
                LogCategory::BackupRestored,
                "crypttab restored from backup",
                Some(serde_json::json!({"backup_id": backup_id})),
                None,
            ))
            .ok();

        Ok(())
    }

    // ── Layer 3: Initramfs Backup ──

    pub fn backup_initramfs(
        &self,
        initramfs_path: &str,
        logger: &AuditLogger,
    ) -> Result<BackupRecord, BackupError> {
        let source = Path::new(initramfs_path);
        if !source.exists() {
            return Err(BackupError::NotFound(initramfs_path.into()));
        }

        let source_name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "initramfs".into());
        let filename = Self::timestamped_name(&source_name, "bak");
        let backup_path = self.initramfs_dir.join(&filename);

        fs::copy(source, &backup_path)?;
        let sha256 = Self::sha256_file(&backup_path)?;
        let size = fs::metadata(&backup_path)?.len();

        let record = BackupRecord {
            id: uuid::Uuid::new_v4().to_string(),
            backup_type: BackupType::Initramfs,
            created_at: Utc::now().to_rfc3339(),
            source_path: initramfs_path.into(),
            backup_path: backup_path.display().to_string(),
            sha256,
            size_bytes: size,
            description: format!("Initramfs backup of {}", initramfs_path),
            device_uuid: None,
        };

        self.save_record(&record)?;

        logger
            .log_operation(&logging::entry(
                LogLevel::Info,
                LogCategory::BackupCreated,
                format!("Initramfs backed up: {} → {}", initramfs_path, backup_path.display()),
                None,
                Some(format!("cp {} {}", backup_path.display(), initramfs_path)),
            ))
            .ok();

        Ok(record)
    }

    // ── Layer 4: Boot Config Backup ──

    pub fn backup_boot_config(
        &self,
        config_path: &str,
        logger: &AuditLogger,
    ) -> Result<BackupRecord, BackupError> {
        let source = Path::new(config_path);
        if !source.exists() {
            return Err(BackupError::NotFound(config_path.into()));
        }

        let filename = Self::timestamped_name("grub-default", "bak");
        let backup_path = self.boot_config_dir.join(&filename);

        fs::copy(source, &backup_path)?;
        let sha256 = Self::sha256_file(&backup_path)?;
        let size = fs::metadata(&backup_path)?.len();

        let record = BackupRecord {
            id: uuid::Uuid::new_v4().to_string(),
            backup_type: BackupType::BootConfig,
            created_at: Utc::now().to_rfc3339(),
            source_path: config_path.into(),
            backup_path: backup_path.display().to_string(),
            sha256,
            size_bytes: size,
            description: format!("Boot config backup of {}", config_path),
            device_uuid: None,
        };

        self.save_record(&record)?;

        logger
            .log_operation(&logging::entry(
                LogLevel::Info,
                LogCategory::BackupCreated,
                format!("Boot config backed up: {} → {}", config_path, backup_path.display()),
                None,
                Some(format!("cp {} {}", backup_path.display(), config_path)),
            ))
            .ok();

        Ok(record)
    }

    // ── Verification ──

    /// Verify integrity of a specific backup
    pub fn verify_backup(&self, backup_id: &str) -> Result<bool, BackupError> {
        let records = self.list_all()?;
        let record = records
            .iter()
            .find(|r| r.id == backup_id)
            .ok_or_else(|| BackupError::NotFound(backup_id.to_string()))?;

        let path = Path::new(&record.backup_path);
        if !path.exists() {
            return Ok(false);
        }

        let current_sha256 = Self::sha256_file(path)?;
        Ok(current_sha256 == record.sha256)
    }

    /// Verify all backups and return list of corrupted ones
    pub fn verify_all(&self) -> Result<Vec<(BackupRecord, bool)>, BackupError> {
        let records = self.list_all()?;
        let mut results = Vec::new();
        for record in records {
            let valid = self.verify_backup(&record.id).unwrap_or(false);
            results.push((record, valid));
        }
        Ok(results)
    }

    /// Get the base backup directory
    #[allow(dead_code)]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}
