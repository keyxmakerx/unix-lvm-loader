use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LogError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Lock poisoned")]
    LockPoisoned,
}

/// Severity level for audit log entries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Category of operation being logged
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogCategory {
    LuksOperation,
    LvmOperation,
    BackupCreated,
    BackupRestored,
    InitramfsRebuild,
    BootConfigChange,
    Tpm2Operation,
    Fido2Operation,
    ClevisOperation,
    KeySlotChange,
    DistroDetection,
    ThemeChange,
    SystemCheck,
    UserAction,
    Recovery,
}

/// A single audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub category: LogCategory,
    pub message: String,
    pub details: Option<serde_json::Value>,
    /// If this was a state-changing operation, what can undo it
    pub rollback_hint: Option<String>,
}

/// Manages operation and audit logging with file persistence
pub struct AuditLogger {
    #[allow(dead_code)]
    log_dir: PathBuf,
    operations_file: Mutex<PathBuf>,
    audit_file: Mutex<PathBuf>,
}

impl AuditLogger {
    /// Create a new logger, initializing log directory and files
    pub fn new(base_dir: &Path) -> Result<Self, LogError> {
        let log_dir = base_dir.join("logs");
        fs::create_dir_all(&log_dir)?;

        let operations_file = log_dir.join("operations.jsonl");
        let audit_file = log_dir.join("audit.jsonl");

        Ok(Self {
            log_dir,
            operations_file: Mutex::new(operations_file),
            audit_file: Mutex::new(audit_file),
        })
    }

    /// Log an operation (general operations log)
    pub fn log_operation(&self, entry: &LogEntry) -> Result<(), LogError> {
        let path = self.operations_file.lock().map_err(|_| LogError::LockPoisoned)?;
        self.append_entry(&path, entry)
    }

    /// Log a security-relevant audit event
    pub fn log_audit(&self, entry: &LogEntry) -> Result<(), LogError> {
        let path = self.audit_file.lock().map_err(|_| LogError::LockPoisoned)?;
        self.append_entry(&path, entry)?;
        // Also log to operations for completeness
        let ops_path = self.operations_file.lock().map_err(|_| LogError::LockPoisoned)?;
        self.append_entry(&ops_path, entry)
    }

    /// Append a log entry as a JSON line
    fn append_entry(&self, path: &Path, entry: &LogEntry) -> Result<(), LogError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let json = serde_json::to_string(entry)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    /// Read recent log entries (last N entries from operations log)
    pub fn read_recent(&self, count: usize) -> Result<Vec<LogEntry>, LogError> {
        let path = self.operations_file.lock().map_err(|_| LogError::LockPoisoned)?;
        self.read_entries(&path, count)
    }

    /// Read recent audit entries
    pub fn read_recent_audit(&self, count: usize) -> Result<Vec<LogEntry>, LogError> {
        let path = self.audit_file.lock().map_err(|_| LogError::LockPoisoned)?;
        self.read_entries(&path, count)
    }

    /// Read entries from a JSONL file, returning the last N
    fn read_entries(&self, path: &Path, count: usize) -> Result<Vec<LogEntry>, LogError> {
        if !path.exists() {
            return Ok(vec![]);
        }
        let content = fs::read_to_string(path)?;
        let entries: Vec<LogEntry> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        let start = if entries.len() > count {
            entries.len() - count
        } else {
            0
        };
        Ok(entries[start..].to_vec())
    }

    /// Get all entries matching a category
    #[allow(dead_code)]
    pub fn filter_by_category(&self, category: &LogCategory) -> Result<Vec<LogEntry>, LogError> {
        let path = self.operations_file.lock().map_err(|_| LogError::LockPoisoned)?;
        if !path.exists() {
            return Ok(vec![]);
        }
        let content = fs::read_to_string(&*path)?;
        let entries: Vec<LogEntry> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .filter(|e: &LogEntry| &e.category == category)
            .collect();
        Ok(entries)
    }

    /// Export all logs as a single JSON array (for debugging/support)
    pub fn export_all(&self) -> Result<String, LogError> {
        let entries = self.read_recent(usize::MAX)?;
        Ok(serde_json::to_string_pretty(&entries)?)
    }

    /// Get the log directory path
    #[allow(dead_code)]
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }
}

/// Convenience function to create a log entry
pub fn entry(
    level: LogLevel,
    category: LogCategory,
    message: impl Into<String>,
    details: Option<serde_json::Value>,
    rollback_hint: Option<String>,
) -> LogEntry {
    LogEntry {
        timestamp: Utc::now(),
        level,
        category,
        message: message.into(),
        details,
        rollback_hint,
    }
}
