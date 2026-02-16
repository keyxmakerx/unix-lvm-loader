use crate::distro::{DistroFamily, DistroInfo};
use crate::logging::{self, AuditLogger, LogCategory, LogLevel};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BootError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Boot loader not detected")]
    NoBootLoader,
}

/// Which boot loader is installed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BootLoader {
    Grub2,
    SystemdBoot,
    Unknown,
}

/// A single boot entry (OS/kernel that can be booted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootEntry {
    pub id: String,
    pub title: String,
    pub linux_kernel: Option<String>,
    pub initrd: Option<String>,
    pub options: Option<String>,
    pub is_default: bool,
    /// Detected distro info for this entry, if we can determine it
    pub distro_id: Option<String>,
    pub distro_icon: Option<String>,
    /// Source: "grub" or "systemd-boot" or "efi"
    pub source: String,
}

/// Full boot configuration state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootState {
    pub boot_loader: BootLoader,
    pub entries: Vec<BootEntry>,
    pub default_entry: Option<String>,
    pub timeout: Option<u32>,
}

/// Detect which bootloader is in use
pub fn detect_bootloader() -> BootLoader {
    // Check for systemd-boot first
    if Path::new("/boot/efi/loader/loader.conf").exists()
        || Path::new("/efi/loader/loader.conf").exists()
        || Path::new("/boot/loader/loader.conf").exists()
    {
        // Verify bootctl works
        if Command::new("bootctl")
            .arg("status")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return BootLoader::SystemdBoot;
        }
    }

    // Check for GRUB
    if Path::new("/boot/grub/grub.cfg").exists()
        || Path::new("/boot/grub2/grub.cfg").exists()
        || Path::new("/etc/default/grub").exists()
    {
        return BootLoader::Grub2;
    }

    BootLoader::Unknown
}

/// Scan all boot entries from the detected boot loader
pub fn scan_boot_entries(
    distro: &DistroInfo,
    logger: &AuditLogger,
) -> Result<BootState, BootError> {
    let boot_loader = detect_bootloader();

    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::SystemCheck,
            format!("Detected boot loader: {:?}", boot_loader),
            None,
            None,
        ))
        .ok();

    let (entries, default_entry, timeout) = match boot_loader {
        BootLoader::Grub2 => scan_grub_entries(distro, logger)?,
        BootLoader::SystemdBoot => scan_systemd_boot_entries(logger)?,
        BootLoader::Unknown => (vec![], None, None),
    };

    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::SystemCheck,
            format!("Found {} boot entries", entries.len()),
            Some(serde_json::json!({
                "entries": entries.iter().map(|e| &e.title).collect::<Vec<_>>(),
            })),
            None,
        ))
        .ok();

    Ok(BootState {
        boot_loader,
        entries,
        default_entry,
        timeout,
    })
}

/// Parse GRUB config to extract menu entries
fn scan_grub_entries(
    _distro: &DistroInfo,
    _logger: &AuditLogger,
) -> Result<(Vec<BootEntry>, Option<String>, Option<u32>), BootError> {
    // Try multiple grub.cfg locations
    let grub_cfg_paths = [
        "/boot/grub/grub.cfg",
        "/boot/grub2/grub.cfg",
    ];

    let grub_cfg = grub_cfg_paths
        .iter()
        .find(|p| Path::new(p).exists())
        .ok_or(BootError::NoBootLoader)?;

    let content = fs::read_to_string(grub_cfg)?;

    // Parse default entry from /etc/default/grub
    let default_entry = parse_grub_default();
    let timeout = parse_grub_timeout();

    let mut entries = Vec::new();
    let menuentry_re = Regex::new(r#"menuentry\s+'([^']+)'|menuentry\s+"([^"]+)""#).unwrap();
    let linux_re = Regex::new(r"(?:linux|linuxefi)\s+(\S+)").unwrap();
    let initrd_re = Regex::new(r"(?:initrd|initrdefi)\s+(\S+)").unwrap();

    let mut entry_idx = 0;
    let mut current_title: Option<String> = None;
    let mut current_linux: Option<String> = None;
    let mut current_initrd: Option<String> = None;
    let mut current_options: Option<String> = None;
    let mut in_entry = false;
    let mut brace_depth = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(cap) = menuentry_re.captures(trimmed) {
            // Save previous entry if any
            if let Some(title) = current_title.take() {
                let is_default = default_entry
                    .as_ref()
                    .map(|d| d == &entry_idx.to_string() || d == &title)
                    .unwrap_or(entry_idx == 0);

                let (distro_id, distro_icon) = guess_distro_from_title(&title);

                entries.push(BootEntry {
                    id: format!("grub-{}", entry_idx),
                    title,
                    linux_kernel: current_linux.take(),
                    initrd: current_initrd.take(),
                    options: current_options.take(),
                    is_default,
                    distro_id,
                    distro_icon,
                    source: "grub".into(),
                });
                entry_idx += 1;
            }

            current_title = cap.get(1).or(cap.get(2)).map(|m| m.as_str().to_string());
            in_entry = true;
            brace_depth = 0;
        }

        if in_entry {
            brace_depth += trimmed.matches('{').count();
            brace_depth = brace_depth.saturating_sub(trimmed.matches('}').count());

            if let Some(cap) = linux_re.captures(trimmed) {
                current_linux = Some(cap[1].to_string());
                // Everything after the kernel path is options
                if let Some(opts_start) = trimmed.find(&cap[1]) {
                    let after_kernel = &trimmed[opts_start + cap[1].len()..];
                    if !after_kernel.trim().is_empty() {
                        current_options = Some(after_kernel.trim().to_string());
                    }
                }
            }

            if let Some(cap) = initrd_re.captures(trimmed) {
                current_initrd = Some(cap[1].to_string());
            }

            if brace_depth == 0 && trimmed.contains('}') {
                in_entry = false;
            }
        }
    }

    // Don't forget the last entry
    if let Some(title) = current_title.take() {
        let is_default = default_entry
            .as_ref()
            .map(|d| d == &entry_idx.to_string() || d == &title)
            .unwrap_or(entry_idx == 0);

        let (distro_id, distro_icon) = guess_distro_from_title(&title);

        entries.push(BootEntry {
            id: format!("grub-{}", entry_idx),
            title,
            linux_kernel: current_linux,
            initrd: current_initrd,
            options: current_options,
            is_default,
            distro_id,
            distro_icon,
            source: "grub".into(),
        });
    }

    Ok((entries, default_entry, timeout))
}

/// Parse systemd-boot entries from /boot/loader/entries/
fn scan_systemd_boot_entries(
    _logger: &AuditLogger,
) -> Result<(Vec<BootEntry>, Option<String>, Option<u32>), BootError> {
    let entries_dirs = [
        "/boot/loader/entries",
        "/boot/efi/loader/entries",
        "/efi/loader/entries",
    ];

    let entries_dir = entries_dirs
        .iter()
        .find(|p| Path::new(p).exists())
        .ok_or(BootError::NoBootLoader)?;

    // Parse loader.conf for default and timeout
    let (default_entry, timeout) = parse_loader_conf();

    let mut entries = Vec::new();

    let read_dir = fs::read_dir(entries_dir)?;
    for dir_entry in read_dir {
        let dir_entry = dir_entry?;
        let path = dir_entry.path();
        if path.extension().map(|e| e == "conf").unwrap_or(false) {
            if let Ok(entry) = parse_systemd_boot_entry(&path, &default_entry) {
                entries.push(entry);
            }
        }
    }

    // Sort by title
    entries.sort_by(|a, b| a.title.cmp(&b.title));

    Ok((entries, default_entry, timeout))
}

fn parse_systemd_boot_entry(
    path: &Path,
    default_id: &Option<String>,
) -> Result<BootEntry, BootError> {
    let content = fs::read_to_string(path)?;
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut title = file_stem.clone();
    let mut linux_kernel = None;
    let mut initrd = None;
    let mut options = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("title") {
            title = val.trim().to_string();
        } else if let Some(val) = trimmed.strip_prefix("linux") {
            linux_kernel = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("initrd") {
            initrd = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("options") {
            options = Some(val.trim().to_string());
        }
    }

    let is_default = default_id
        .as_ref()
        .map(|d| d == &file_stem || d == &format!("{}.conf", file_stem))
        .unwrap_or(false);

    let (distro_id, distro_icon) = guess_distro_from_title(&title);

    Ok(BootEntry {
        id: file_stem,
        title,
        linux_kernel,
        initrd,
        options,
        is_default,
        distro_id,
        distro_icon,
        source: "systemd-boot".into(),
    })
}

fn parse_grub_default() -> Option<String> {
    let content = fs::read_to_string("/etc/default/grub").ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("GRUB_DEFAULT=") {
            let val = trimmed
                .strip_prefix("GRUB_DEFAULT=")?
                .trim_matches('"')
                .trim_matches('\'');
            return Some(val.to_string());
        }
    }
    None
}

fn parse_grub_timeout() -> Option<u32> {
    let content = fs::read_to_string("/etc/default/grub").ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("GRUB_TIMEOUT=") {
            let val = trimmed
                .strip_prefix("GRUB_TIMEOUT=")?
                .trim_matches('"')
                .trim_matches('\'');
            return val.parse().ok();
        }
    }
    None
}

fn parse_loader_conf() -> (Option<String>, Option<u32>) {
    let loader_paths = [
        "/boot/loader/loader.conf",
        "/boot/efi/loader/loader.conf",
        "/efi/loader/loader.conf",
    ];

    for path in &loader_paths {
        if let Ok(content) = fs::read_to_string(path) {
            let mut default = None;
            let mut timeout = None;

            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(val) = trimmed.strip_prefix("default") {
                    default = Some(val.trim().to_string());
                } else if let Some(val) = trimmed.strip_prefix("timeout") {
                    timeout = val.trim().parse().ok();
                }
            }

            return (default, timeout);
        }
    }

    (None, None)
}

/// Try to guess which distro a boot entry belongs to from its title
fn guess_distro_from_title(title: &str) -> (Option<String>, Option<String>) {
    let title_lower = title.to_lowercase();

    let distros = [
        ("ubuntu", "ubuntu"),
        ("debian", "debian"),
        ("fedora", "fedora"),
        ("nobara", "nobara"),
        ("bazzite", "bazzite"),
        ("cachyos", "cachyos"),
        ("cachy", "cachyos"),
        ("arch", "arch"),
        ("manjaro", "manjaro"),
        ("endeavouros", "endeavouros"),
        ("endeavour", "endeavouros"),
        ("opensuse", "opensuse"),
        ("suse", "opensuse"),
        ("pop!_os", "pop-os"),
        ("pop os", "pop-os"),
        ("linux mint", "linuxmint"),
        ("mint", "linuxmint"),
        ("windows", "windows"),
    ];

    for (keyword, id) in &distros {
        if title_lower.contains(keyword) {
            return (Some(id.to_string()), Some(id.to_string()));
        }
    }

    // If it contains "Linux" generically
    if title_lower.contains("linux") {
        return (Some("linux".into()), Some("linux".into()));
    }

    (None, None)
}

/// Set the default boot entry
pub fn set_default_entry(
    entry_id: &str,
    boot_loader: &BootLoader,
    distro: &DistroInfo,
    logger: &AuditLogger,
) -> Result<(), BootError> {
    logger
        .log_audit(&logging::entry(
            LogLevel::Warning,
            LogCategory::BootConfigChange,
            format!("Setting default boot entry to: {}", entry_id),
            Some(serde_json::json!({"entry_id": entry_id, "boot_loader": format!("{:?}", boot_loader)})),
            None,
        ))
        .ok();

    match boot_loader {
        BootLoader::Grub2 => {
            // Modify GRUB_DEFAULT in /etc/default/grub
            let grub_default = "/etc/default/grub";
            let content = fs::read_to_string(grub_default)?;

            let mut new_lines = Vec::new();
            let mut found = false;
            for line in content.lines() {
                if line.trim().starts_with("GRUB_DEFAULT=") {
                    new_lines.push(format!("GRUB_DEFAULT=\"{}\"", entry_id));
                    found = true;
                } else {
                    new_lines.push(line.to_string());
                }
            }
            if !found {
                new_lines.push(format!("GRUB_DEFAULT=\"{}\"", entry_id));
            }

            fs::write(grub_default, new_lines.join("\n") + "\n")?;

            // Regenerate GRUB config
            let grub_cmd = crate::distro::grub_update_command(distro);
            if !grub_cmd.is_empty() {
                let output = Command::new(&grub_cmd[0])
                    .args(&grub_cmd[1..])
                    .output()?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(BootError::CommandFailed(format!(
                        "GRUB config regeneration failed: {}",
                        stderr
                    )));
                }
            }
        }
        BootLoader::SystemdBoot => {
            let output = Command::new("bootctl")
                .args(["set-default", entry_id])
                .output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(BootError::CommandFailed(format!(
                    "bootctl set-default failed: {}",
                    stderr
                )));
            }
        }
        BootLoader::Unknown => {
            return Err(BootError::NoBootLoader);
        }
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Info,
            LogCategory::BootConfigChange,
            format!("Default boot entry set to: {}", entry_id),
            None,
            None,
        ))
        .ok();

    Ok(())
}
