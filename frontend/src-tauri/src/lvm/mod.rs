use crate::logging::{self, AuditLogger, LogCategory, LogLevel};
use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LvmError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("LVM not available")]
    NotAvailable,
}

/// Physical Volume info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalVolume {
    pub pv_name: String,
    pub vg_name: String,
    pub pv_size: String,
    pub pv_free: String,
    pub pv_uuid: String,
}

/// Volume Group info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeGroup {
    pub vg_name: String,
    pub vg_size: String,
    pub vg_free: String,
    pub pv_count: u32,
    pub lv_count: u32,
    pub vg_uuid: String,
}

/// Logical Volume info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalVolume {
    pub lv_name: String,
    pub vg_name: String,
    pub lv_size: String,
    pub lv_path: String,
    pub lv_uuid: String,
    pub lv_attr: String,
    pub pool_lv: Option<String>,
    pub origin: Option<String>,
}

/// Full LVM system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LvmState {
    pub available: bool,
    pub physical_volumes: Vec<PhysicalVolume>,
    pub volume_groups: Vec<VolumeGroup>,
    pub logical_volumes: Vec<LogicalVolume>,
}

/// Check if LVM tools are available
pub fn is_available() -> bool {
    Command::new("lvs")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Scan the full LVM state
pub fn scan(logger: &AuditLogger) -> Result<LvmState, LvmError> {
    if !is_available() {
        return Ok(LvmState {
            available: false,
            physical_volumes: vec![],
            volume_groups: vec![],
            logical_volumes: vec![],
        });
    }

    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::SystemCheck,
            "Scanning LVM state",
            None,
            None,
        ))
        .ok();

    let pvs = scan_pvs()?;
    let vgs = scan_vgs()?;
    let lvs = scan_lvs()?;

    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::SystemCheck,
            format!(
                "LVM scan complete: {} PV(s), {} VG(s), {} LV(s)",
                pvs.len(),
                vgs.len(),
                lvs.len()
            ),
            None,
            None,
        ))
        .ok();

    Ok(LvmState {
        available: true,
        physical_volumes: pvs,
        volume_groups: vgs,
        logical_volumes: lvs,
    })
}

fn scan_pvs() -> Result<Vec<PhysicalVolume>, LvmError> {
    let output = Command::new("pvs")
        .args([
            "--noheadings",
            "--nosuffix",
            "--separator", "|",
            "-o", "pv_name,vg_name,pv_size,pv_free,pv_uuid",
        ])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pvs = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.trim().split('|').collect();
            if parts.len() >= 5 {
                Some(PhysicalVolume {
                    pv_name: parts[0].trim().to_string(),
                    vg_name: parts[1].trim().to_string(),
                    pv_size: parts[2].trim().to_string(),
                    pv_free: parts[3].trim().to_string(),
                    pv_uuid: parts[4].trim().to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(pvs)
}

fn scan_vgs() -> Result<Vec<VolumeGroup>, LvmError> {
    let output = Command::new("vgs")
        .args([
            "--noheadings",
            "--nosuffix",
            "--separator", "|",
            "-o", "vg_name,vg_size,vg_free,pv_count,lv_count,vg_uuid",
        ])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let vgs = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.trim().split('|').collect();
            if parts.len() >= 6 {
                Some(VolumeGroup {
                    vg_name: parts[0].trim().to_string(),
                    vg_size: parts[1].trim().to_string(),
                    vg_free: parts[2].trim().to_string(),
                    pv_count: parts[3].trim().parse().unwrap_or(0),
                    lv_count: parts[4].trim().parse().unwrap_or(0),
                    vg_uuid: parts[5].trim().to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(vgs)
}

fn scan_lvs() -> Result<Vec<LogicalVolume>, LvmError> {
    let output = Command::new("lvs")
        .args([
            "--noheadings",
            "--nosuffix",
            "--separator", "|",
            "-o", "lv_name,vg_name,lv_size,lv_path,lv_uuid,lv_attr,pool_lv,origin",
        ])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lvs = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.trim().split('|').collect();
            if parts.len() >= 6 {
                Some(LogicalVolume {
                    lv_name: parts[0].trim().to_string(),
                    vg_name: parts[1].trim().to_string(),
                    lv_size: parts[2].trim().to_string(),
                    lv_path: parts.get(3).map(|s| s.trim().to_string()).unwrap_or_default(),
                    lv_uuid: parts.get(4).map(|s| s.trim().to_string()).unwrap_or_default(),
                    lv_attr: parts.get(5).map(|s| s.trim().to_string()).unwrap_or_default(),
                    pool_lv: parts.get(6).and_then(|s| {
                        let t = s.trim();
                        if t.is_empty() { None } else { Some(t.to_string()) }
                    }),
                    origin: parts.get(7).and_then(|s| {
                        let t = s.trim();
                        if t.is_empty() { None } else { Some(t.to_string()) }
                    }),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(lvs)
}
