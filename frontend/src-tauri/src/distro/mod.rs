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
        let _fields = parse_os_release(content);
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
        let _fields = parse_os_release(content);
        let id_like = vec!["arch".to_string()];
        let family = classify_family("cachyos", &id_like, None, false);
        assert_eq!(family, DistroFamily::Arch);
    }

    #[test]
    fn test_parse_fedora_workstation() {
        let content = r#"
NAME="Fedora Linux"
VERSION="40 (Workstation Edition)"
ID=fedora
VERSION_ID=40
VARIANT_ID=workstation
PRETTY_NAME="Fedora Linux 40 (Workstation Edition)"
ID_LIKE=""
"#;
        let fields = parse_os_release(content);
        assert_eq!(fields.get("ID").unwrap(), "fedora");
        assert_eq!(fields.get("VARIANT_ID").unwrap(), "workstation");

        let family = classify_family("fedora", &[], Some("workstation"), false);
        assert_eq!(family, DistroFamily::Fedora);
    }

    #[test]
    fn test_parse_fedora_silverblue() {
        let family = classify_family("fedora", &[], Some("silverblue"), true);
        assert_eq!(family, DistroFamily::Atomic);
    }

    #[test]
    fn test_parse_fedora_kinoite() {
        let family = classify_family("fedora", &[], Some("kinoite"), true);
        assert_eq!(family, DistroFamily::Atomic);
    }

    #[test]
    fn test_parse_nobara() {
        let content = r#"
NAME="Nobara Linux"
ID=nobara
ID_LIKE="fedora"
VERSION_ID="39"
PRETTY_NAME="Nobara Linux 39"
"#;
        let fields = parse_os_release(content);
        assert_eq!(fields.get("ID").unwrap(), "nobara");

        let id_like = vec!["fedora".to_string()];
        let family = classify_family("nobara", &id_like, None, false);
        assert_eq!(family, DistroFamily::Fedora);
    }

    #[test]
    fn test_parse_opensuse() {
        let family = classify_family("opensuse-tumbleweed", &[], None, false);
        assert_eq!(family, DistroFamily::Suse);
    }

    #[test]
    fn test_parse_manjaro() {
        let id_like = vec!["arch".to_string()];
        let family = classify_family("manjaro", &id_like, None, false);
        assert_eq!(family, DistroFamily::Arch);
    }

    #[test]
    fn test_parse_endeavouros() {
        let family = classify_family("endeavouros", &["arch".to_string()], None, false);
        assert_eq!(family, DistroFamily::Arch);
    }

    #[test]
    fn test_parse_pop_os() {
        let id_like = vec!["ubuntu".to_string(), "debian".to_string()];
        let family = classify_family("pop", &id_like, None, false);
        assert_eq!(family, DistroFamily::Debian);
    }

    #[test]
    fn test_parse_linux_mint() {
        let id_like = vec!["ubuntu".to_string()];
        let family = classify_family("linuxmint", &id_like, None, false);
        assert_eq!(family, DistroFamily::Debian);
    }

    #[test]
    fn test_parse_debian() {
        let family = classify_family("debian", &[], None, false);
        assert_eq!(family, DistroFamily::Debian);
    }

    #[test]
    fn test_unknown_distro() {
        let family = classify_family("gentoo", &[], None, false);
        assert_eq!(family, DistroFamily::Unknown);
    }

    #[test]
    fn test_parse_os_release_handles_comments() {
        let content = "# This is a comment\nID=test\n# Another comment\nNAME=\"Test\"";
        let fields = parse_os_release(content);
        assert_eq!(fields.get("ID").unwrap(), "test");
        assert_eq!(fields.get("NAME").unwrap(), "Test");
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn test_parse_os_release_handles_single_quotes() {
        let content = "ID='ubuntu'\nNAME='Ubuntu'";
        let fields = parse_os_release(content);
        assert_eq!(fields.get("ID").unwrap(), "ubuntu");
    }

    #[test]
    fn test_parse_os_release_handles_empty_lines() {
        let content = "\n\nID=test\n\n\nNAME=\"Test\"\n\n";
        let fields = parse_os_release(content);
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn test_distro_icon_known() {
        assert_eq!(distro_icon("ubuntu"), Some("ubuntu".to_string()));
        assert_eq!(distro_icon("fedora"), Some("fedora".to_string()));
        assert_eq!(distro_icon("arch"), Some("arch".to_string()));
        assert_eq!(distro_icon("bazzite"), Some("bazzite".to_string()));
        assert_eq!(distro_icon("cachyos"), Some("cachyos".to_string()));
        assert_eq!(distro_icon("nobara"), Some("nobara".to_string()));
    }

    #[test]
    fn test_distro_icon_unknown() {
        assert_eq!(distro_icon("gentoo"), None);
        assert_eq!(distro_icon("void"), None);
    }

    #[test]
    fn test_initramfs_rebuild_command_dracut() {
        let info = DistroInfo {
            id: "fedora".into(), name: "Fedora".into(), version: "40".into(),
            id_like: vec![], variant_id: None, family: DistroFamily::Fedora,
            initramfs: InitramfsSystem::Dracut, package_manager: PackageManager::Dnf,
            is_immutable: false, pretty_name: "Fedora 40".into(), icon_name: None,
        };
        let cmd = initramfs_rebuild_command(&info);
        assert_eq!(cmd, vec!["dracut", "--force"]);
    }

    #[test]
    fn test_initramfs_rebuild_command_mkinitcpio() {
        let info = DistroInfo {
            id: "arch".into(), name: "Arch".into(), version: "".into(),
            id_like: vec![], variant_id: None, family: DistroFamily::Arch,
            initramfs: InitramfsSystem::Mkinitcpio, package_manager: PackageManager::Pacman,
            is_immutable: false, pretty_name: "Arch Linux".into(), icon_name: None,
        };
        let cmd = initramfs_rebuild_command(&info);
        assert_eq!(cmd, vec!["mkinitcpio", "-P"]);
    }

    #[test]
    fn test_initramfs_rebuild_command_initramfs_tools() {
        let info = DistroInfo {
            id: "ubuntu".into(), name: "Ubuntu".into(), version: "24.04".into(),
            id_like: vec![], variant_id: None, family: DistroFamily::Debian,
            initramfs: InitramfsSystem::InitramfsTools, package_manager: PackageManager::Apt,
            is_immutable: false, pretty_name: "Ubuntu 24.04".into(), icon_name: None,
        };
        let cmd = initramfs_rebuild_command(&info);
        assert_eq!(cmd, vec!["update-initramfs", "-u"]);
    }

    #[test]
    fn test_initramfs_rebuild_command_atomic() {
        let info = DistroInfo {
            id: "fedora".into(), name: "Bazzite".into(), version: "41".into(),
            id_like: vec![], variant_id: Some("bazzite".into()), family: DistroFamily::Atomic,
            initramfs: InitramfsSystem::Dracut, package_manager: PackageManager::RpmOstree,
            is_immutable: true, pretty_name: "Bazzite 41".into(), icon_name: None,
        };
        let cmd = initramfs_rebuild_command(&info);
        assert_eq!(cmd, vec!["rpm-ostree", "initramfs", "--enable"]);
    }

    #[test]
    fn test_grub_update_command_per_family() {
        let make_info = |family: DistroFamily| DistroInfo {
            id: "test".into(), name: "Test".into(), version: "1".into(),
            id_like: vec![], variant_id: None, family,
            initramfs: InitramfsSystem::Unknown, package_manager: PackageManager::Unknown,
            is_immutable: false, pretty_name: "Test".into(), icon_name: None,
        };

        assert_eq!(grub_update_command(&make_info(DistroFamily::Debian)), vec!["update-grub"]);
        assert_eq!(grub_update_command(&make_info(DistroFamily::Fedora)), vec!["grub2-mkconfig", "-o", "/boot/grub2/grub.cfg"]);
        assert_eq!(grub_update_command(&make_info(DistroFamily::Arch)), vec!["grub-mkconfig", "-o", "/boot/grub/grub.cfg"]);
        assert!(grub_update_command(&make_info(DistroFamily::Unknown)).is_empty());
    }
}
