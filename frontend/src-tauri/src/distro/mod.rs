use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DistroError {
    #[error("Failed to read os-release: {0}")]
    OsReleaseReadError(#[from] std::io::Error),
    #[error("Could not determine distribution")]
    UnknownDistro,
}

/// High-level distro family for grouping behavior
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistroFamily {
    Debian,  // Ubuntu, Debian, Mint, Pop!_OS
    Fedora,  // Fedora, Nobara
    Atomic,  // Bazzite, Fedora Silverblue/Kinoite (immutable)
    Arch,    // Arch, CachyOS, Manjaro, EndeavourOS
    Suse,    // openSUSE Leap, Tumbleweed
    Unknown,
}

/// Initramfs system used by the distro
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitramfsSystem {
    Dracut,         // Fedora, openSUSE, RHEL
    Mkinitcpio,     // Arch, CachyOS, Manjaro
    InitramfsTools, // Debian, Ubuntu
    Unknown,
}

/// Package manager used by the distro
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageManager {
    Apt,       // Debian, Ubuntu
    Dnf,       // Fedora, Nobara
    RpmOstree, // Bazzite, Silverblue (immutable)
    Pacman,    // Arch, CachyOS
    Zypper,    // openSUSE
    Unknown,
}

/// Full distro detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub id_like: Vec<String>,
    pub variant_id: Option<String>,
    pub family: DistroFamily,
    pub initramfs: InitramfsSystem,
    pub package_manager: PackageManager,
    pub is_immutable: bool,
    pub pretty_name: String,
    pub icon_name: Option<String>,
}

/// Parse /etc/os-release into key-value pairs
fn parse_os_release(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim_matches('"').trim_matches('\'');
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}

/// Detect if system is running on an immutable/ostree filesystem
fn detect_immutable() -> bool {
    // Check for ostree deployment
    Path::new("/run/ostree-booted").exists()
        || Path::new("/sysroot/ostree").exists()
        || which_exists("ostree")
}

/// Check if a command exists in PATH
fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect initramfs system by checking which tools are installed
fn detect_initramfs() -> InitramfsSystem {
    if which_exists("dracut") {
        InitramfsSystem::Dracut
    } else if which_exists("mkinitcpio") {
        InitramfsSystem::Mkinitcpio
    } else if which_exists("update-initramfs") {
        InitramfsSystem::InitramfsTools
    } else {
        InitramfsSystem::Unknown
    }
}

/// Detect package manager
fn detect_package_manager(is_immutable: bool) -> PackageManager {
    if is_immutable && which_exists("rpm-ostree") {
        PackageManager::RpmOstree
    } else if which_exists("apt") {
        PackageManager::Apt
    } else if which_exists("dnf") {
        PackageManager::Dnf
    } else if which_exists("pacman") {
        PackageManager::Pacman
    } else if which_exists("zypper") {
        PackageManager::Zypper
    } else {
        PackageManager::Unknown
    }
}

/// Classify distro into a family based on os-release fields
fn classify_family(id: &str, id_like: &[String], variant_id: Option<&str>, is_immutable: bool) -> DistroFamily {
    let id_lower = id.to_lowercase();

    // Check immutable Fedora variants first
    if is_immutable {
        if id_lower == "fedora" || id_like.iter().any(|s| s == "fedora") {
            return DistroFamily::Atomic;
        }
    }

    // Check for Bazzite specifically (always atomic)
    if id_lower == "bazzite" || variant_id.map(|v| v.contains("bazzite")).unwrap_or(false) {
        return DistroFamily::Atomic;
    }

    // Atomic variants by variant_id
    if let Some(vid) = variant_id {
        let vid_lower = vid.to_lowercase();
        if vid_lower == "silverblue" || vid_lower == "kinoite" || vid_lower == "sericea" {
            return DistroFamily::Atomic;
        }
    }

    // Arch family
    if id_lower == "arch" || id_lower == "cachyos" || id_lower == "manjaro" || id_lower == "endeavouros" {
        return DistroFamily::Arch;
    }
    if id_like.iter().any(|s| s == "arch") {
        return DistroFamily::Arch;
    }

    // Fedora family (non-atomic)
    if id_lower == "fedora" || id_lower == "nobara" {
        return DistroFamily::Fedora;
    }
    if id_like.iter().any(|s| s == "fedora") {
        return DistroFamily::Fedora;
    }

    // Debian family
    if id_lower == "ubuntu" || id_lower == "debian" || id_lower == "linuxmint" || id_lower == "pop" {
        return DistroFamily::Debian;
    }
    if id_like.iter().any(|s| s == "debian" || s == "ubuntu") {
        return DistroFamily::Debian;
    }

    // SUSE family
    if id_lower.starts_with("opensuse") || id_lower == "sles" {
        return DistroFamily::Suse;
    }
    if id_like.iter().any(|s| s.contains("suse")) {
        return DistroFamily::Suse;
    }

    DistroFamily::Unknown
}

/// Map distro ID to a likely icon/logo name
fn distro_icon(id: &str) -> Option<String> {
    let icon = match id.to_lowercase().as_str() {
        "ubuntu" => "ubuntu",
        "debian" => "debian",
        "fedora" => "fedora",
        "nobara" => "nobara",
        "bazzite" => "bazzite",
        "cachyos" => "cachyos",
        "arch" => "arch",
        "manjaro" => "manjaro",
        "endeavouros" => "endeavouros",
        "pop" => "pop-os",
        "linuxmint" => "linuxmint",
        _ if id.to_lowercase().starts_with("opensuse") => "opensuse",
        _ => return None,
    };
    Some(icon.to_string())
}

/// Main detection entrypoint — reads /etc/os-release and classifies the system
pub fn detect() -> Result<DistroInfo, DistroError> {
    detect_from_path("/etc/os-release")
}

/// Detection from a specific path (useful for testing)
pub fn detect_from_path(path: &str) -> Result<DistroInfo, DistroError> {
    let content = std::fs::read_to_string(path)?;
    let fields = parse_os_release(&content);

    let id = fields.get("ID").cloned().unwrap_or_default();
    let name = fields.get("NAME").cloned().unwrap_or_default();
    let version = fields.get("VERSION_ID").cloned().unwrap_or_default();
    let pretty_name = fields.get("PRETTY_NAME").cloned().unwrap_or_else(|| name.clone());
    let variant_id = fields.get("VARIANT_ID").cloned();
    let id_like: Vec<String> = fields
        .get("ID_LIKE")
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default();

    if id.is_empty() {
        return Err(DistroError::UnknownDistro);
    }

    let is_immutable = detect_immutable();
    let family = classify_family(&id, &id_like, variant_id.as_deref(), is_immutable);
    let initramfs = detect_initramfs();
    let package_manager = detect_package_manager(is_immutable);
    let icon_name = distro_icon(&id);

    Ok(DistroInfo {
        id,
        name,
        version,
        id_like,
        variant_id,
        family,
        initramfs,
        package_manager,
        is_immutable,
        pretty_name,
        icon_name,
    })
}

/// Get the command to rebuild initramfs for this distro
pub fn initramfs_rebuild_command(info: &DistroInfo) -> Vec<String> {
    match info.initramfs {
        InitramfsSystem::Dracut => {
            if info.is_immutable {
                vec!["rpm-ostree".into(), "initramfs".into(), "--enable".into()]
            } else {
                vec!["dracut".into(), "--force".into()]
            }
        }
        InitramfsSystem::Mkinitcpio => vec!["mkinitcpio".into(), "-P".into()],
        InitramfsSystem::InitramfsTools => vec!["update-initramfs".into(), "-u".into()],
        InitramfsSystem::Unknown => vec![],
    }
}

/// Get the command to update GRUB config for this distro
pub fn grub_update_command(info: &DistroInfo) -> Vec<String> {
    match info.family {
        DistroFamily::Debian => vec!["update-grub".into()],
        DistroFamily::Fedora | DistroFamily::Suse => {
            vec!["grub2-mkconfig".into(), "-o".into(), "/boot/grub2/grub.cfg".into()]
        }
        DistroFamily::Arch => {
            vec!["grub-mkconfig".into(), "-o".into(), "/boot/grub/grub.cfg".into()]
        }
        DistroFamily::Atomic => {
            // Immutable systems modify kernel args differently
            vec!["rpm-ostree".into(), "kargs".into()]
        }
        DistroFamily::Unknown => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ubuntu() {
        let content = r#"
NAME="Ubuntu"
VERSION="24.04 LTS (Noble Numbat)"
ID=ubuntu
ID_LIKE=debian
VERSION_ID="24.04"
PRETTY_NAME="Ubuntu 24.04 LTS"
"#;
        let fields = parse_os_release(content);
        assert_eq!(fields.get("ID").unwrap(), "ubuntu");
        assert_eq!(fields.get("ID_LIKE").unwrap(), "debian");

        let id_like = vec!["debian".to_string()];
        let family = classify_family("ubuntu", &id_like, None, false);
        assert_eq!(family, DistroFamily::Debian);
    }

    #[test]
    fn test_parse_bazzite() {
        let content = r#"
NAME="Bazzite"
ID=fedora
VARIANT_ID=bazzite
VERSION_ID="41"
PRETTY_NAME="Bazzite 41"
"#;
        let fields = parse_os_release(content);
        let id_like: Vec<String> = vec![];
        let family = classify_family("fedora", &id_like, Some("bazzite"), true);
        assert_eq!(family, DistroFamily::Atomic);
    }

    #[test]
    fn test_parse_cachyos() {
        let content = r#"
NAME="CachyOS Linux"
ID=cachyos
ID_LIKE=arch
VERSION_ID="2024.01"
PRETTY_NAME="CachyOS Linux"
"#;
        let fields = parse_os_release(content);
        let id_like = vec!["arch".to_string()];
        let family = classify_family("cachyos", &id_like, None, false);
        assert_eq!(family, DistroFamily::Arch);
    }
}
