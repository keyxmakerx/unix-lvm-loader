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
    Ok(parse_pvs_output(&stdout))
}

fn parse_pvs_output(stdout: &str) -> Vec<PhysicalVolume> {
    stdout
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
        .collect()
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
    Ok(parse_vgs_output(&stdout))
}

fn parse_vgs_output(stdout: &str) -> Vec<VolumeGroup> {
    stdout
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
        .collect()
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
    Ok(parse_lvs_output(&stdout))
}

fn parse_lvs_output(stdout: &str) -> Vec<LogicalVolume> {
    stdout
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
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pvs_output() {
        let output = "  /dev/sda3|vg_root|500.00|100.00|abc-123-def\n";
        let pvs = parse_pvs_output(output);
        assert_eq!(pvs.len(), 1);
        assert_eq!(pvs[0].pv_name, "/dev/sda3");
        assert_eq!(pvs[0].vg_name, "vg_root");
        assert_eq!(pvs[0].pv_size, "500.00");
        assert_eq!(pvs[0].pv_free, "100.00");
        assert_eq!(pvs[0].pv_uuid, "abc-123-def");
    }

    #[test]
    fn test_parse_pvs_multiple() {
        let output = "  /dev/sda3|vg0|500.00|0.00|uuid1\n  /dev/sdb1|vg1|1000.00|500.00|uuid2\n";
        let pvs = parse_pvs_output(output);
        assert_eq!(pvs.len(), 2);
        assert_eq!(pvs[0].pv_name, "/dev/sda3");
        assert_eq!(pvs[1].pv_name, "/dev/sdb1");
    }

    #[test]
    fn test_parse_pvs_empty() {
        let pvs = parse_pvs_output("");
        assert!(pvs.is_empty());
    }

    #[test]
    fn test_parse_pvs_malformed_line() {
        let output = "  /dev/sda3|vg0\n  /dev/sdb1|vg1|1000.00|500.00|uuid2\n";
        let pvs = parse_pvs_output(output);
        assert_eq!(pvs.len(), 1); // First line too short, second is OK
        assert_eq!(pvs[0].pv_name, "/dev/sdb1");
    }

    #[test]
    fn test_parse_vgs_output() {
        let output = "  vg_root|500.00|100.00|1|3|vg-uuid-1\n";
        let vgs = parse_vgs_output(output);
        assert_eq!(vgs.len(), 1);
        assert_eq!(vgs[0].vg_name, "vg_root");
        assert_eq!(vgs[0].vg_size, "500.00");
        assert_eq!(vgs[0].vg_free, "100.00");
        assert_eq!(vgs[0].pv_count, 1);
        assert_eq!(vgs[0].lv_count, 3);
        assert_eq!(vgs[0].vg_uuid, "vg-uuid-1");
    }

    #[test]
    fn test_parse_vgs_multiple() {
        let output = "  vg0|200.00|50.00|1|2|uuid-a\n  vg1|400.00|200.00|2|1|uuid-b\n";
        let vgs = parse_vgs_output(output);
        assert_eq!(vgs.len(), 2);
        assert_eq!(vgs[0].vg_name, "vg0");
        assert_eq!(vgs[1].vg_name, "vg1");
        assert_eq!(vgs[1].pv_count, 2);
    }

    #[test]
    fn test_parse_vgs_empty() {
        let vgs = parse_vgs_output("");
        assert!(vgs.is_empty());
    }

    #[test]
    fn test_parse_lvs_output() {
        let output = "  root|vg0|100.00|/dev/vg0/root|lv-uuid-1|-wi-ao---|||\n";
        let lvs = parse_lvs_output(output);
        assert_eq!(lvs.len(), 1);
        assert_eq!(lvs[0].lv_name, "root");
        assert_eq!(lvs[0].vg_name, "vg0");
        assert_eq!(lvs[0].lv_size, "100.00");
        assert_eq!(lvs[0].lv_path, "/dev/vg0/root");
        assert_eq!(lvs[0].lv_attr, "-wi-ao---");
    }

    #[test]
    fn test_parse_lvs_with_pool_and_origin() {
        let output = "  snap1|vg0|10.00|/dev/vg0/snap1|lv-uuid-2|swi-a-s---|pool0|root\n";
        let lvs = parse_lvs_output(output);
        assert_eq!(lvs.len(), 1);
        assert_eq!(lvs[0].pool_lv, Some("pool0".to_string()));
        assert_eq!(lvs[0].origin, Some("root".to_string()));
    }

    #[test]
    fn test_parse_lvs_without_optional_fields() {
        let output = "  root|vg0|100.00|/dev/vg0/root|lv-uuid|attr||\n";
        let lvs = parse_lvs_output(output);
        assert_eq!(lvs.len(), 1);
        assert_eq!(lvs[0].pool_lv, None);
        assert_eq!(lvs[0].origin, None);
    }

    #[test]
    fn test_parse_lvs_multiple() {
        let output = "  root|vg0|50.00|/dev/vg0/root|u1|-wi-ao---||\n  swap|vg0|8.00|/dev/vg0/swap|u2|-wi-ao---||\n  home|vg0|200.00|/dev/vg0/home|u3|-wi-ao---||\n";
        let lvs = parse_lvs_output(output);
        assert_eq!(lvs.len(), 3);
        assert_eq!(lvs[0].lv_name, "root");
        assert_eq!(lvs[1].lv_name, "swap");
        assert_eq!(lvs[2].lv_name, "home");
    }

    #[test]
    fn test_parse_lvs_empty() {
        let lvs = parse_lvs_output("");
        assert!(lvs.is_empty());
    }
}
