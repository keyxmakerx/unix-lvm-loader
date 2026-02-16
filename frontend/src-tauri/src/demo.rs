//! Demo/simulation mode: returns realistic mock data so the UI
//! can be tested on machines without LUKS volumes or LVM.
//!
//! Activated via the `set_demo_mode` Tauri command or the
//! `UNIX_LVM_LOADER_DEMO=1` environment variable.

use crate::backup::{BackupRecord, BackupType};
use crate::boot::{BootEntry, BootLoader, BootState};
use crate::clevis::{ClevisBinding, ClevisStatus};
use crate::commands::SystemOverview;
use crate::distro::{DistroFamily, DistroInfo, InitramfsSystem, PackageManager};
use crate::logging::{LogCategory, LogEntry, LogLevel};
use crate::luks::{KeySlot, LuksInfo, LuksToken, LuksVersion};
use crate::lvm::{LogicalVolume, LvmState, PhysicalVolume, VolumeGroup};
use crate::privilege::{EscalationMethod, PrivilegeInfo, PrivilegeLevel};
use crate::recovery::{CheckStatus, DiagnosticResult, HealthCheck, OverallHealth};
use crate::theme::{Theme, ThemeStyle};
use chrono::Utc;

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

/// Demo backup records showing a realistic backup history.
pub fn demo_backups(filter: Option<&str>) -> Vec<BackupRecord> {
    let all = vec![
        BackupRecord {
            id: "bak-001".into(),
            backup_type: BackupType::LuksHeader,
            created_at: "2026-02-14T10:30:00Z".into(),
            source_path: "/dev/sda3".into(),
            backup_path: "/var/lib/unix-lvm-loader/backups/luks-header-sda3-20260214.bin".into(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            size_bytes: 16_777_216,
            description: "LUKS2 header backup for /dev/sda3 (fedora_crypt)".into(),
            device_uuid: Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890".into()),
        },
        BackupRecord {
            id: "bak-002".into(),
            backup_type: BackupType::LuksHeader,
            created_at: "2026-02-14T10:30:05Z".into(),
            source_path: "/dev/nvme0n1p3".into(),
            backup_path: "/var/lib/unix-lvm-loader/backups/luks-header-nvme0n1p3-20260214.bin".into(),
            sha256: "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592".into(),
            size_bytes: 16_777_216,
            description: "LUKS2 header backup for /dev/nvme0n1p3 (data_crypt)".into(),
            device_uuid: Some("f9e8d7c6-b5a4-3210-fedc-ba0987654321".into()),
        },
        BackupRecord {
            id: "bak-003".into(),
            backup_type: BackupType::Crypttab,
            created_at: "2026-02-14T10:31:00Z".into(),
            source_path: "/etc/crypttab".into(),
            backup_path: "/var/lib/unix-lvm-loader/backups/crypttab-20260214.bak".into(),
            sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".into(),
            size_bytes: 256,
            description: "crypttab backup (2 entries)".into(),
            device_uuid: None,
        },
        BackupRecord {
            id: "bak-004".into(),
            backup_type: BackupType::BootConfig,
            created_at: "2026-02-13T15:22:00Z".into(),
            source_path: "/etc/default/grub".into(),
            backup_path: "/var/lib/unix-lvm-loader/backups/grub-default-20260213.bak".into(),
            sha256: "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e".into(),
            size_bytes: 1024,
            description: "GRUB configuration before boot order change".into(),
            device_uuid: None,
        },
        BackupRecord {
            id: "bak-005".into(),
            backup_type: BackupType::Initramfs,
            created_at: "2026-02-12T09:15:00Z".into(),
            source_path: "/boot/initramfs-6.12.5-200.fc41.x86_64.img".into(),
            backup_path: "/var/lib/unix-lvm-loader/backups/initramfs-6.12.5-20260212.img.bak".into(),
            sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824".into(),
            size_bytes: 82_214_912,
            description: "Initramfs backup before TPM2 enrollment rebuild".into(),
            device_uuid: None,
        },
    ];

    match filter {
        Some("luks_header") => all.into_iter().filter(|b| b.backup_type == BackupType::LuksHeader).collect(),
        Some("crypttab") => all.into_iter().filter(|b| b.backup_type == BackupType::Crypttab).collect(),
        Some("initramfs") => all.into_iter().filter(|b| b.backup_type == BackupType::Initramfs).collect(),
        Some("boot_config") => all.into_iter().filter(|b| b.backup_type == BackupType::BootConfig).collect(),
        _ => all,
    }
}

/// Demo theme list with built-in themes.
pub fn demo_themes() -> Vec<Theme> {
    vec![
        Theme {
            id: "classic".into(),
            name: "Classic".into(),
            description: "Clean text-based boot menu with distro detection".into(),
            author: "unix-lvm-loader".into(),
            version: "1.0.0".into(),
            path: Some("/usr/share/unix-lvm-loader/themes/classic".into()),
            thumbnail: None,
            screenshot: None,
            installed: true,
            active: true,
            style: ThemeStyle::Text,
            repo_url: None,
            thumbnail_data: None,
        },
        Theme {
            id: "modern".into(),
            name: "Modern".into(),
            description: "Graphical boot menu with OS icons and animations (BURG-style)".into(),
            author: "unix-lvm-loader".into(),
            version: "1.0.0".into(),
            path: Some("/usr/share/unix-lvm-loader/themes/modern".into()),
            thumbnail: None,
            screenshot: None,
            installed: true,
            active: false,
            style: ThemeStyle::Graphical,
            repo_url: None,
            thumbnail_data: None,
        },
    ]
}

/// Demo log entries showing recent activity.
pub fn demo_recent_logs(count: usize) -> Vec<LogEntry> {
    let now = Utc::now();
    let entries = vec![
        LogEntry {
            timestamp: now - chrono::Duration::minutes(2),
            level: LogLevel::Info,
            category: LogCategory::DistroDetection,
            message: "Detected: Fedora Linux 41 (Workstation Edition) (fedora)".into(),
            details: None,
            rollback_hint: None,
        },
        LogEntry {
            timestamp: now - chrono::Duration::minutes(5),
            level: LogLevel::Info,
            category: LogCategory::SystemCheck,
            message: "System health check completed: Healthy".into(),
            details: None,
            rollback_hint: None,
        },
        LogEntry {
            timestamp: now - chrono::Duration::minutes(10),
            level: LogLevel::Info,
            category: LogCategory::BackupCreated,
            message: "LUKS header backup created for /dev/sda3".into(),
            details: Some(serde_json::json!({"device": "/dev/sda3", "uuid": "a1b2c3d4"})),
            rollback_hint: None,
        },
        LogEntry {
            timestamp: now - chrono::Duration::minutes(10),
            level: LogLevel::Info,
            category: LogCategory::BackupCreated,
            message: "LUKS header backup created for /dev/nvme0n1p3".into(),
            details: Some(serde_json::json!({"device": "/dev/nvme0n1p3"})),
            rollback_hint: None,
        },
        LogEntry {
            timestamp: now - chrono::Duration::minutes(11),
            level: LogLevel::Info,
            category: LogCategory::BackupCreated,
            message: "crypttab backup created".into(),
            details: None,
            rollback_hint: None,
        },
        LogEntry {
            timestamp: now - chrono::Duration::minutes(30),
            level: LogLevel::Info,
            category: LogCategory::Tpm2Operation,
            message: "TPM2 enrollment completed for /dev/sda3 (slot 1)".into(),
            details: Some(serde_json::json!({"slot": 1, "device": "/dev/sda3"})),
            rollback_hint: Some("Remove TPM2 key slot: systemd-cryptenroll --wipe-slot=1 /dev/sda3".into()),
        },
        LogEntry {
            timestamp: now - chrono::Duration::minutes(31),
            level: LogLevel::Info,
            category: LogCategory::InitramfsRebuild,
            message: "Initramfs rebuilt successfully (dracut)".into(),
            details: Some(serde_json::json!({"kernel": "6.12.5-200.fc41.x86_64"})),
            rollback_hint: Some("Restore initramfs from backup".into()),
        },
        LogEntry {
            timestamp: now - chrono::Duration::hours(1),
            level: LogLevel::Warning,
            category: LogCategory::SystemCheck,
            message: "Only 1 LUKS header backup found for /dev/nvme0n1p3, recommend at least 2".into(),
            details: None,
            rollback_hint: None,
        },
        LogEntry {
            timestamp: now - chrono::Duration::hours(2),
            level: LogLevel::Info,
            category: LogCategory::ClevisOperation,
            message: "Tang binding added to /dev/sda3 (slot 2)".into(),
            details: Some(serde_json::json!({"url": "http://tang.internal:7500", "slot": 2})),
            rollback_hint: Some("clevis luks unbind -d /dev/sda3 -s 2".into()),
        },
        LogEntry {
            timestamp: now - chrono::Duration::hours(3),
            level: LogLevel::Info,
            category: LogCategory::BootConfigChange,
            message: "Default boot entry changed to Fedora Linux (6.12.5-200.fc41.x86_64)".into(),
            details: None,
            rollback_hint: Some("Restore /etc/default/grub from backup".into()),
        },
        LogEntry {
            timestamp: now - chrono::Duration::hours(4),
            level: LogLevel::Info,
            category: LogCategory::ThemeChange,
            message: "Active theme changed to: classic".into(),
            details: None,
            rollback_hint: None,
        },
        LogEntry {
            timestamp: now - chrono::Duration::hours(5),
            level: LogLevel::Info,
            category: LogCategory::UserAction,
            message: "Demo mode enabled".into(),
            details: None,
            rollback_hint: None,
        },
    ];
    entries.into_iter().take(count).collect()
}

/// Demo audit log entries (security-relevant operations only).
pub fn demo_audit_logs(count: usize) -> Vec<LogEntry> {
    let now = Utc::now();
    let entries = vec![
        LogEntry {
            timestamp: now - chrono::Duration::minutes(10),
            level: LogLevel::Info,
            category: LogCategory::BackupCreated,
            message: "LUKS header backup created for /dev/sda3".into(),
            details: Some(serde_json::json!({"device": "/dev/sda3", "sha256": "e3b0c44298fc1c..."})),
            rollback_hint: None,
        },
        LogEntry {
            timestamp: now - chrono::Duration::minutes(30),
            level: LogLevel::Info,
            category: LogCategory::Tpm2Operation,
            message: "TPM2 key enrolled on /dev/sda3 (slot 1)".into(),
            details: Some(serde_json::json!({"slot": 1, "device": "/dev/sda3"})),
            rollback_hint: Some("systemd-cryptenroll --wipe-slot=1 /dev/sda3".into()),
        },
        LogEntry {
            timestamp: now - chrono::Duration::hours(2),
            level: LogLevel::Info,
            category: LogCategory::ClevisOperation,
            message: "Clevis Tang binding added to /dev/sda3 slot 2".into(),
            details: Some(serde_json::json!({"url": "http://tang.internal:7500"})),
            rollback_hint: Some("clevis luks unbind -d /dev/sda3 -s 2".into()),
        },
        LogEntry {
            timestamp: now - chrono::Duration::hours(3),
            level: LogLevel::Info,
            category: LogCategory::KeySlotChange,
            message: "Recovery key enrolled on /dev/sda3 (slot 3)".into(),
            details: Some(serde_json::json!({"slot": 3})),
            rollback_hint: Some("systemd-cryptenroll --wipe-slot=3 /dev/sda3".into()),
        },
    ];
    entries.into_iter().take(count).collect()
}

/// Demo recovery backups for the recovery page.
pub fn demo_recovery_backups() -> Vec<BackupRecord> {
    demo_backups(None)
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
