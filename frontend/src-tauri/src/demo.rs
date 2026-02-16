//! Demo/simulation mode: returns realistic mock data so the UI
//! can be tested on machines without LUKS volumes or LVM.
//!
//! Activated via the `set_demo_mode` Tauri command or the
//! `UNIX_LVM_LOADER_DEMO=1` environment variable.

use crate::boot::{BootEntry, BootLoader, BootState};
use crate::clevis::{ClevisBinding, ClevisStatus};
use crate::commands::SystemOverview;
use crate::distro::{DistroFamily, DistroInfo, InitramfsSystem, PackageManager};
use crate::luks::{KeySlot, LuksInfo, LuksToken, LuksVersion};
use crate::lvm::{LogicalVolume, LvmState, PhysicalVolume, VolumeGroup};
use crate::privilege::{EscalationMethod, PrivilegeInfo, PrivilegeLevel};
use crate::recovery::{
    CheckStatus, DiagnosticResult, HealthCheck, OverallHealth,
};

/// Generate a realistic system overview for demo purposes.
/// Simulates a Fedora 41 workstation with LUKS2 on LVM, dual-boot with Windows.
pub fn demo_system_overview() -> SystemOverview {
    SystemOverview {
        distro: Some(DistroInfo {
            id: "fedora".into(),
            name: "Fedora Linux".into(),
            version: "41".into(),
            id_like: vec!["fedora".into()],
            variant_id: Some("workstation".into()),
            pretty_name: "Fedora Linux 41 (Workstation Edition)".into(),
            family: DistroFamily::Fedora,
            initramfs: InitramfsSystem::Dracut,
            package_manager: PackageManager::Dnf,
            is_immutable: false,
            icon_name: Some("fedora".into()),
        }),
        luks_volumes: vec![
            "/dev/sda3".into(),
            "/dev/nvme0n1p3".into(),
        ],
        lvm: LvmState {
            available: true,
            physical_volumes: vec![
                PhysicalVolume {
                    pv_name: "/dev/mapper/luks-a1b2c3d4".into(),
                    vg_name: "vg_fedora".into(),
                    pv_size: "476.44".into(),
                    pv_free: "50.00".into(),
                    pv_uuid: "Abc123-DeF4-5678-GhIj-KlMn0pQrStUv".into(),
                },
            ],
            volume_groups: vec![
                VolumeGroup {
                    vg_name: "vg_fedora".into(),
                    vg_size: "476.44".into(),
                    vg_free: "50.00".into(),
                    pv_count: 1,
                    lv_count: 3,
                    vg_uuid: "VgUuId-1234-5678-AbCd-EfGhIjKlMnOp".into(),
                },
            ],
            logical_volumes: vec![
                LogicalVolume {
                    lv_name: "root".into(),
                    vg_name: "vg_fedora".into(),
                    lv_size: "100.00".into(),
                    lv_path: "/dev/vg_fedora/root".into(),
                    lv_uuid: "LvRoot-Uuid-1234".into(),
                    lv_attr: "-wi-ao---".into(),
                    pool_lv: None,
                    origin: None,
                },
                LogicalVolume {
                    lv_name: "home".into(),
                    vg_name: "vg_fedora".into(),
                    lv_size: "318.44".into(),
                    lv_path: "/dev/vg_fedora/home".into(),
                    lv_uuid: "LvHome-Uuid-5678".into(),
                    lv_attr: "-wi-ao---".into(),
                    pool_lv: None,
                    origin: None,
                },
                LogicalVolume {
                    lv_name: "swap".into(),
                    vg_name: "vg_fedora".into(),
                    lv_size: "8.00".into(),
                    lv_path: "/dev/vg_fedora/swap".into(),
                    lv_uuid: "LvSwap-Uuid-9012".into(),
                    lv_attr: "-wi-ao---".into(),
                    pool_lv: None,
                    origin: None,
                },
            ],
        },
        boot: Some(BootState {
            boot_loader: BootLoader::Grub2,
            entries: vec![
                BootEntry {
                    id: "grub-0".into(),
                    title: "Fedora Linux (6.12.5-200.fc41.x86_64)".into(),
                    linux_kernel: Some("/boot/vmlinuz-6.12.5-200.fc41.x86_64".into()),
                    initrd: Some("/boot/initramfs-6.12.5-200.fc41.x86_64.img".into()),
                    options: Some("root=/dev/mapper/vg_fedora-root ro rd.luks.uuid=a1b2c3d4-e5f6-7890-abcd-ef1234567890 rhgb quiet".into()),
                    is_default: true,
                    distro_id: Some("fedora".into()),
                    distro_icon: Some("fedora".into()),
                    source: "grub".into(),
                },
                BootEntry {
                    id: "grub-1".into(),
                    title: "Fedora Linux (6.11.11-300.fc41.x86_64)".into(),
                    linux_kernel: Some("/boot/vmlinuz-6.11.11-300.fc41.x86_64".into()),
                    initrd: Some("/boot/initramfs-6.11.11-300.fc41.x86_64.img".into()),
                    options: Some("root=/dev/mapper/vg_fedora-root ro rd.luks.uuid=a1b2c3d4-e5f6-7890-abcd-ef1234567890 rhgb quiet".into()),
                    is_default: false,
                    distro_id: Some("fedora".into()),
                    distro_icon: Some("fedora".into()),
                    source: "grub".into(),
                },
                BootEntry {
                    id: "grub-2".into(),
                    title: "Windows Boot Manager (on /dev/sda1)".into(),
                    linux_kernel: None,
                    initrd: None,
                    options: None,
                    is_default: false,
                    distro_id: Some("windows".into()),
                    distro_icon: Some("windows".into()),
                    source: "grub".into(),
                },
            ],
            default_entry: Some("0".into()),
            timeout: Some(5),
        }),
        backup_count: 3,
        has_cryptenroll: true,
        privilege: PrivilegeInfo {
            level: PrivilegeLevel::User,
            escalation_method: EscalationMethod::Pkexec,
            uid: 1000,
            username: "demo-user".into(),
        },
    }
}

/// Demo LUKS info for a typical LUKS2 volume with TPM2 enrolled.
pub fn demo_luks_info(device: &str) -> LuksInfo {
    LuksInfo {
        device: device.into(),
        uuid: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".into(),
        version: LuksVersion::Luks2,
        cipher: "aes-xts-plain64".into(),
        key_size_bits: 512,
        hash: "sha256".into(),
        label: Some("fedora_crypt".into()),
        key_slots: vec![
            KeySlot {
                slot_number: 0,
                enabled: true,
                key_type: "luks2".into(),
                kdf: Some("argon2id".into()),
                priority: Some("normal".into()),
            },
            KeySlot {
                slot_number: 1,
                enabled: true,
                key_type: "luks2".into(),
                kdf: Some("argon2id".into()),
                priority: Some("normal".into()),
            },
        ],
        tokens: vec![
            LuksToken {
                token_id: 0,
                token_type: "systemd-tpm2".into(),
                keyslots: vec![1],
            },
        ],
        total_slots: 32,
        active_passphrase_slots: 2,
    }
}

/// Demo Clevis status (installed with version).
pub fn demo_clevis_status() -> ClevisStatus {
    ClevisStatus {
        clevis_installed: true,
        clevis_luks_installed: true,
        tang_client_installed: true,
        clevis_version: Some("19".into()),
    }
}

/// Demo Clevis bindings on a device.
pub fn demo_clevis_bindings() -> Vec<ClevisBinding> {
    vec![
        ClevisBinding {
            slot: 2,
            pin: "tang".into(),
            config: r#"{"url":"http://tang.internal:7500"}"#.into(),
            server_url: Some("http://tang.internal:7500".into()),
        },
    ]
}

/// Demo diagnostic result — mostly healthy with one warning.
pub fn demo_diagnostics() -> DiagnosticResult {
    DiagnosticResult {
        checks: vec![
            HealthCheck {
                name: "cryptsetup".into(),
                description: "Verify cryptsetup is installed and functional".into(),
                status: CheckStatus::Pass,
                detail: Some("cryptsetup 2.7.5".into()),
            },
            HealthCheck {
                name: "LUKS volumes".into(),
                description: "Detect encrypted volumes".into(),
                status: CheckStatus::Pass,
                detail: Some("2 LUKS volume(s) detected".into()),
            },
            HealthCheck {
                name: "crypttab".into(),
                description: "Verify /etc/crypttab exists and is valid".into(),
                status: CheckStatus::Pass,
                detail: Some("2 crypttab entries OK".into()),
            },
            HealthCheck {
                name: "initramfs".into(),
                description: "Verify initramfs exists for current kernel".into(),
                status: CheckStatus::Pass,
                detail: Some("/boot/initramfs-6.12.5-200.fc41.x86_64.img (78.4 MB)".into()),
            },
            HealthCheck {
                name: "Boot config".into(),
                description: "Verify boot loader configuration".into(),
                status: CheckStatus::Pass,
                detail: Some("GRUB config OK (3 entries)".into()),
            },
            HealthCheck {
                name: "Backups".into(),
                description: "Verify backup health".into(),
                status: CheckStatus::Warning,
                detail: Some("3 backup(s) (1 LUKS header backups) — consider creating more header backups".into()),
            },
            HealthCheck {
                name: "systemd-cryptenroll".into(),
                description: "Verify systemd-cryptenroll for TPM2/FIDO2".into(),
                status: CheckStatus::Pass,
                detail: Some("systemd 256.11-1.fc41".into()),
            },
            HealthCheck {
                name: "LVM".into(),
                description: "Verify LVM volume groups".into(),
                status: CheckStatus::Pass,
                detail: Some("1 volume group(s) active".into()),
            },
        ],
        issues: vec![],
        overall_health: OverallHealth::Healthy,
    }
}
