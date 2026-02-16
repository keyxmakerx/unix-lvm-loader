use crate::logging::{self, AuditLogger, LogCategory, LogLevel};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LuksError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Safety check failed: {0}")]
    SafetyCheck(String),
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
}

/// LUKS version
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LuksVersion {
    Luks1,
    Luks2,
    Unknown,
}

/// Key derivation function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Kdf {
    Pbkdf2,
    Argon2i,
    Argon2id,
    Unknown(String),
}

/// State of a single LUKS key slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeySlot {
    pub slot_number: u32,
    pub enabled: bool,
    pub key_type: String,     // "passphrase", "keyfile", "tpm2", "fido2", etc.
    pub kdf: Option<String>,
    pub priority: Option<String>,
}

/// Token (LUKS2 only) — represents TPM2, FIDO2, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuksToken {
    pub token_id: u32,
    pub token_type: String,   // "systemd-tpm2", "systemd-fido2", "clevis", etc.
    pub keyslots: Vec<u32>,
}

/// Full dump of a LUKS volume's metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuksInfo {
    pub device: String,
    pub uuid: String,
    pub version: LuksVersion,
    pub cipher: String,
    pub key_size_bits: u32,
    pub hash: String,
    pub label: Option<String>,
    pub key_slots: Vec<KeySlot>,
    pub tokens: Vec<LuksToken>,
    pub total_slots: u32,
    pub active_passphrase_slots: u32,
}

/// Detect all LUKS volumes on the system
pub fn detect_luks_volumes(logger: &AuditLogger) -> Result<Vec<String>, LuksError> {
    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::SystemCheck,
            "Scanning for LUKS volumes",
            None,
            None,
        ))
        .ok();

    let output = Command::new("blkid")
        .args(["-t", "TYPE=crypto_LUKS", "-o", "device"])
        .output()?;

    if !output.status.success() {
        // blkid returns non-zero if no matches — that's OK
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let devices: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::SystemCheck,
            format!("Found {} LUKS volume(s)", devices.len()),
            Some(serde_json::json!({"devices": &devices})),
            None,
        ))
        .ok();

    Ok(devices)
}

/// Get detailed info about a LUKS volume by parsing `cryptsetup luksDump`
pub fn get_luks_info(device: &str, logger: &AuditLogger) -> Result<LuksInfo, LuksError> {
    logger
        .log_operation(&logging::entry(
            LogLevel::Debug,
            LogCategory::LuksOperation,
            format!("Reading LUKS info for {}", device),
            None,
            None,
        ))
        .ok();

    let output = Command::new("cryptsetup")
        .args(["luksDump", device])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LuksError::CommandFailed(format!(
            "luksDump failed for {}: {}",
            device, stderr
        )));
    }

    let dump = String::from_utf8_lossy(&output.stdout).to_string();
    parse_luks_dump(device, &dump)
}

/// Parse the output of `cryptsetup luksDump`
fn parse_luks_dump(device: &str, dump: &str) -> Result<LuksInfo, LuksError> {
    let version = if dump.contains("Version:       2") || dump.contains("LUKS header information for") && dump.contains("Version:") {
        if dump.contains("Version:       2") {
            LuksVersion::Luks2
        } else if dump.contains("Version:       1") {
            LuksVersion::Luks1
        } else {
            LuksVersion::Unknown
        }
    } else {
        LuksVersion::Unknown
    };

    // Parse UUID
    let uuid = extract_field(dump, "UUID")
        .unwrap_or_default();

    // Parse cipher
    let cipher = extract_field(dump, "Cipher name")
        .or_else(|| extract_field(dump, "Cipher"))
        .unwrap_or_default();

    // Parse cipher mode and combine
    let cipher_mode = extract_field(dump, "Cipher mode").unwrap_or_default();
    let full_cipher = if cipher_mode.is_empty() {
        cipher
    } else {
        format!("{}-{}", cipher, cipher_mode)
    };

    // Parse key size
    let key_size_str = extract_field(dump, "MK bits")
        .or_else(|| extract_field(dump, "Key"))
        .unwrap_or_default();
    let key_size_bits: u32 = key_size_str
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Parse hash
    let hash = extract_field(dump, "Hash spec")
        .or_else(|| extract_field(dump, "Hash"))
        .unwrap_or_default();

    // Parse label (LUKS2 only)
    let label = extract_field(dump, "Label");

    // Parse key slots
    let key_slots = parse_key_slots(dump, &version);
    let active_passphrase_slots = key_slots.iter().filter(|s| s.enabled).count() as u32;

    // Parse tokens (LUKS2 only)
    let tokens = if version == LuksVersion::Luks2 {
        parse_tokens(dump)
    } else {
        vec![]
    };

    let total_slots = match version {
        LuksVersion::Luks1 => 8,
        LuksVersion::Luks2 => 32,
        LuksVersion::Unknown => 8,
    };

    Ok(LuksInfo {
        device: device.to_string(),
        uuid,
        version,
        cipher: full_cipher,
        key_size_bits,
        hash,
        label,
        key_slots,
        tokens,
        total_slots,
        active_passphrase_slots,
    })
}

fn extract_field(dump: &str, field: &str) -> Option<String> {
    for line in dump.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(field) {
            if let Some(value) = trimmed.split(':').nth(1) {
                let v = value.trim().to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn parse_key_slots(dump: &str, version: &LuksVersion) -> Vec<KeySlot> {
    let mut slots = Vec::new();

    match version {
        LuksVersion::Luks1 => {
            // LUKS1: "Key Slot 0: ENABLED" / "Key Slot 1: DISABLED"
            let re = Regex::new(r"Key Slot (\d+): (ENABLED|DISABLED)").unwrap();
            for cap in re.captures_iter(dump) {
                let slot_num: u32 = cap[1].parse().unwrap_or(0);
                let enabled = &cap[2] == "ENABLED";
                slots.push(KeySlot {
                    slot_number: slot_num,
                    enabled,
                    key_type: if enabled { "passphrase".into() } else { "empty".into() },
                    kdf: None,
                    priority: None,
                });
            }
        }
        LuksVersion::Luks2 => {
            // LUKS2: Parse "Keyslots:" section entries
            let mut in_keyslots = false;
            let mut current_slot: Option<u32> = None;

            for line in dump.lines() {
                let trimmed = line.trim();

                if trimmed == "Keyslots:" {
                    in_keyslots = true;
                    continue;
                }
                if trimmed == "Tokens:" || trimmed == "Digests:" {
                    in_keyslots = false;
                    continue;
                }

                if in_keyslots {
                    // Match slot header like "  0: luks2"
                    if let Some(cap) = Regex::new(r"^(\d+): (\w+)")
                        .unwrap()
                        .captures(trimmed)
                    {
                        let slot_num: u32 = cap[1].parse().unwrap_or(0);
                        let slot_type = cap[2].to_string();
                        slots.push(KeySlot {
                            slot_number: slot_num,
                            enabled: true,
                            key_type: slot_type,
                            kdf: None,
                            priority: None,
                        });
                        current_slot = Some(slot_num);
                    }

                    // Parse KDF for current slot
                    if let Some(slot_num) = current_slot {
                        if trimmed.starts_with("KDF:") {
                            if let Some(slot) = slots.iter_mut().find(|s| s.slot_number == slot_num) {
                                slot.kdf = Some(trimmed.replace("KDF:", "").trim().to_string());
                            }
                        }
                        if trimmed.starts_with("Priority:") {
                            if let Some(slot) = slots.iter_mut().find(|s| s.slot_number == slot_num) {
                                slot.priority = Some(trimmed.replace("Priority:", "").trim().to_string());
                            }
                        }
                    }
                }
            }
        }
        LuksVersion::Unknown => {}
    }

    slots
}

fn parse_tokens(dump: &str) -> Vec<LuksToken> {
    let mut tokens = Vec::new();
    let mut in_tokens = false;
    let mut current_token: Option<u32> = None;

    for line in dump.lines() {
        let trimmed = line.trim();

        if trimmed == "Tokens:" {
            in_tokens = true;
            continue;
        }
        if trimmed == "Digests:" {
            in_tokens = false;
            continue;
        }

        if in_tokens {
            // Match token header like "  0: systemd-tpm2"
            if let Some(cap) = Regex::new(r"^(\d+): (.+)")
                .unwrap()
                .captures(trimmed)
            {
                let token_id: u32 = cap[1].parse().unwrap_or(0);
                let token_type = cap[2].trim().to_string();
                tokens.push(LuksToken {
                    token_id,
                    token_type,
                    keyslots: vec![],
                });
                current_token = Some(token_id);
            }

            // Parse keyslots for current token
            if let Some(tid) = current_token {
                if trimmed.starts_with("Keyslot:") {
                    if let Some(token) = tokens.iter_mut().find(|t| t.token_id == tid) {
                        let slot_str = trimmed.replace("Keyslot:", "").trim().to_string();
                        if let Ok(slot) = slot_str.parse::<u32>() {
                            token.keyslots.push(slot);
                        }
                    }
                }
            }
        }
    }

    tokens
}

/// Count how many passphrase-type key slots are active
/// Used for safety checks — never allow removing the last one
pub fn count_passphrase_slots(info: &LuksInfo) -> u32 {
    info.key_slots
        .iter()
        .filter(|s| s.enabled && (s.key_type == "passphrase" || s.key_type == "luks2"))
        .count() as u32
}

/// Safety check: would this operation leave the user locked out?
pub fn safety_check_key_removal(info: &LuksInfo, slot_to_remove: u32) -> Result<(), LuksError> {
    let remaining = info
        .key_slots
        .iter()
        .filter(|s| s.enabled && s.slot_number != slot_to_remove)
        .count();

    if remaining == 0 {
        return Err(LuksError::SafetyCheck(
            "Cannot remove the last active key slot. You would be permanently locked out of your encrypted volume. \
             Add another key slot first, then remove this one."
                .into(),
        ));
    }

    Ok(())
}

/// Check if systemd-cryptenroll is available (for TPM2/FIDO2)
pub fn has_cryptenroll() -> bool {
    Command::new("systemd-cryptenroll")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Enroll a TPM2 device into a LUKS2 volume
pub fn enroll_tpm2(
    device: &str,
    info: &LuksInfo,
    logger: &AuditLogger,
) -> Result<(), LuksError> {
    if info.version != LuksVersion::Luks2 {
        return Err(LuksError::SafetyCheck(
            "TPM2 enrollment requires LUKS2. This volume is LUKS1.".into(),
        ));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Warning,
            LogCategory::Tpm2Operation,
            format!("Enrolling TPM2 device for {}", device),
            Some(serde_json::json!({
                "device": device,
                "uuid": info.uuid,
            })),
            Some(format!("systemd-cryptenroll --wipe-slot=tpm2 {}", device)),
        ))
        .ok();

    let output = Command::new("systemd-cryptenroll")
        .args(["--tpm2-device=auto", device])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        logger
            .log_audit(&logging::entry(
                LogLevel::Error,
                LogCategory::Tpm2Operation,
                format!("TPM2 enrollment FAILED for {}: {}", device, stderr),
                None,
                None,
            ))
            .ok();
        return Err(LuksError::CommandFailed(format!(
            "systemd-cryptenroll TPM2 failed: {}",
            stderr
        )));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Info,
            LogCategory::Tpm2Operation,
            format!("TPM2 enrolled successfully for {}", device),
            None,
            Some(format!("systemd-cryptenroll --wipe-slot=tpm2 {}", device)),
        ))
        .ok();

    Ok(())
}

/// Enroll a FIDO2 device into a LUKS2 volume
pub fn enroll_fido2(
    device: &str,
    info: &LuksInfo,
    logger: &AuditLogger,
) -> Result<(), LuksError> {
    if info.version != LuksVersion::Luks2 {
        return Err(LuksError::SafetyCheck(
            "FIDO2 enrollment requires LUKS2. This volume is LUKS1.".into(),
        ));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Warning,
            LogCategory::Fido2Operation,
            format!("Enrolling FIDO2 device for {}", device),
            Some(serde_json::json!({
                "device": device,
                "uuid": info.uuid,
            })),
            Some(format!("systemd-cryptenroll --wipe-slot=fido2 {}", device)),
        ))
        .ok();

    let output = Command::new("systemd-cryptenroll")
        .args(["--fido2-device=auto", device])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LuksError::CommandFailed(format!(
            "systemd-cryptenroll FIDO2 failed: {}",
            stderr
        )));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Info,
            LogCategory::Fido2Operation,
            format!("FIDO2 enrolled successfully for {}", device),
            None,
            Some(format!("systemd-cryptenroll --wipe-slot=fido2 {}", device)),
        ))
        .ok();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_field() {
        let dump = "Version:       2\nUUID:          abc-def-123\nCipher name:   aes\nCipher mode:   xts-plain64\nHash spec:     sha256\nMK bits:       512";
        assert_eq!(extract_field(dump, "UUID"), Some("abc-def-123".to_string()));
        assert_eq!(extract_field(dump, "Cipher name"), Some("aes".to_string()));
        assert_eq!(extract_field(dump, "Cipher mode"), Some("xts-plain64".to_string()));
        assert_eq!(extract_field(dump, "MK bits"), Some("512".to_string()));
        assert_eq!(extract_field(dump, "Nonexistent"), None);
    }

    #[test]
    fn test_parse_luks2_dump() {
        let dump = r#"LUKS header information for /dev/sda3

Version:       2
Epoch:         5
Metadata area: 16384 [bytes]
Keyslots area: 16744448 [bytes]
UUID:          12345678-abcd-ef01-2345-6789abcdef01
Label:         (no label)
Subsystem:     (no subsystem)
Flags:         (no flags)

Data segments:
  0: crypt
	offset: 16777216 [bytes]
	length: (whole device)
	cipher: aes-xts-plain64
	sector: 512

Keyslots:
  0: luks2
	Key:        512 bits
	Priority:   normal
	Cipher:     aes-xts-plain64
	Cipher key: 512 bits
	PBKDF:      argon2id
  1: luks2
	Key:        512 bits
	Priority:   normal
	Cipher:     aes-xts-plain64
	Cipher key: 512 bits
	PBKDF:      argon2id
Tokens:
  0: systemd-tpm2
	Keyslot:    1
Digests:
  0: pbkdf2
	Hash:       sha256
"#;
        let info = parse_luks_dump("/dev/sda3", dump).unwrap();
        assert_eq!(info.version, LuksVersion::Luks2);
        assert_eq!(info.uuid, "12345678-abcd-ef01-2345-6789abcdef01");
        assert_eq!(info.total_slots, 32);
        assert_eq!(info.key_slots.len(), 2);
        assert!(info.key_slots[0].enabled);
        assert_eq!(info.key_slots[0].slot_number, 0);
        assert_eq!(info.key_slots[1].slot_number, 1);
        assert_eq!(info.active_passphrase_slots, 2);
        assert_eq!(info.tokens.len(), 1);
        assert_eq!(info.tokens[0].token_type, "systemd-tpm2");
        assert_eq!(info.tokens[0].keyslots, vec![1]);
    }

    #[test]
    fn test_parse_luks1_dump() {
        let dump = r#"LUKS header information for /dev/sdb1

Version:       1
Cipher name:   aes
Cipher mode:   xts-plain64
Hash spec:     sha256
Payload offset: 4096
MK bits:       256
MK digest:     aa bb cc dd ee ff 00 11 22 33 44 55 66 77 88 99 aa bb cc dd

Key Slot 0: ENABLED
	Iterations:  1234567
Key Slot 1: DISABLED
Key Slot 2: DISABLED
Key Slot 3: DISABLED
Key Slot 4: DISABLED
Key Slot 5: ENABLED
Key Slot 6: DISABLED
Key Slot 7: DISABLED
"#;
        let info = parse_luks_dump("/dev/sdb1", dump).unwrap();
        assert_eq!(info.version, LuksVersion::Luks1);
        assert_eq!(info.total_slots, 8);
        assert_eq!(info.key_slots.len(), 8);
        assert!(info.key_slots[0].enabled);
        assert!(!info.key_slots[1].enabled);
        assert!(info.key_slots[5].enabled);
        assert_eq!(info.active_passphrase_slots, 2);
        assert!(info.tokens.is_empty()); // LUKS1 has no tokens
    }

    #[test]
    fn test_parse_tokens_multiple() {
        let dump = "Tokens:\n  0: systemd-tpm2\n\tKeyslot:    0\n  1: systemd-fido2\n\tKeyslot:    2\nDigests:\n";
        let tokens = parse_tokens(dump);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_type, "systemd-tpm2");
        assert_eq!(tokens[0].keyslots, vec![0]);
        assert_eq!(tokens[1].token_type, "systemd-fido2");
        assert_eq!(tokens[1].keyslots, vec![2]);
    }

    #[test]
    fn test_parse_tokens_empty() {
        let dump = "Tokens:\nDigests:\n";
        let tokens = parse_tokens(dump);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_count_passphrase_slots() {
        let info = LuksInfo {
            device: "/dev/sda".into(), uuid: "test".into(),
            version: LuksVersion::Luks2, cipher: "aes".into(),
            key_size_bits: 512, hash: "sha256".into(), label: None,
            key_slots: vec![
                KeySlot { slot_number: 0, enabled: true, key_type: "luks2".into(), kdf: None, priority: None },
                KeySlot { slot_number: 1, enabled: true, key_type: "luks2".into(), kdf: None, priority: None },
                KeySlot { slot_number: 2, enabled: false, key_type: "empty".into(), kdf: None, priority: None },
            ],
            tokens: vec![], total_slots: 32, active_passphrase_slots: 2,
        };
        assert_eq!(count_passphrase_slots(&info), 2);
    }

    #[test]
    fn test_safety_check_allows_removal_with_remaining() {
        let info = LuksInfo {
            device: "/dev/sda".into(), uuid: "test".into(),
            version: LuksVersion::Luks2, cipher: "aes".into(),
            key_size_bits: 512, hash: "sha256".into(), label: None,
            key_slots: vec![
                KeySlot { slot_number: 0, enabled: true, key_type: "luks2".into(), kdf: None, priority: None },
                KeySlot { slot_number: 1, enabled: true, key_type: "luks2".into(), kdf: None, priority: None },
            ],
            tokens: vec![], total_slots: 32, active_passphrase_slots: 2,
        };
        assert!(safety_check_key_removal(&info, 0).is_ok());
    }

    #[test]
    fn test_safety_check_blocks_last_slot_removal() {
        let info = LuksInfo {
            device: "/dev/sda".into(), uuid: "test".into(),
            version: LuksVersion::Luks2, cipher: "aes".into(),
            key_size_bits: 512, hash: "sha256".into(), label: None,
            key_slots: vec![
                KeySlot { slot_number: 0, enabled: true, key_type: "luks2".into(), kdf: None, priority: None },
            ],
            tokens: vec![], total_slots: 32, active_passphrase_slots: 1,
        };
        let result = safety_check_key_removal(&info, 0);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("last active key slot"));
    }

    #[test]
    fn test_safety_check_ignores_disabled_slots() {
        let info = LuksInfo {
            device: "/dev/sda".into(), uuid: "test".into(),
            version: LuksVersion::Luks2, cipher: "aes".into(),
            key_size_bits: 512, hash: "sha256".into(), label: None,
            key_slots: vec![
                KeySlot { slot_number: 0, enabled: true, key_type: "luks2".into(), kdf: None, priority: None },
                KeySlot { slot_number: 1, enabled: false, key_type: "empty".into(), kdf: None, priority: None },
            ],
            tokens: vec![], total_slots: 32, active_passphrase_slots: 1,
        };
        // Only 1 enabled slot, removing it should fail even though slot 1 exists (disabled)
        assert!(safety_check_key_removal(&info, 0).is_err());
    }
}

/// Generate a recovery key and enroll it
pub fn enroll_recovery_key(
    device: &str,
    info: &LuksInfo,
    logger: &AuditLogger,
) -> Result<String, LuksError> {
    if info.version != LuksVersion::Luks2 {
        return Err(LuksError::SafetyCheck(
            "Recovery key enrollment requires LUKS2.".into(),
        ));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Warning,
            LogCategory::KeySlotChange,
            format!("Enrolling recovery key for {}", device),
            None,
            None,
        ))
        .ok();

    let output = Command::new("systemd-cryptenroll")
        .args(["--recovery-key", device])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LuksError::CommandFailed(format!(
            "Recovery key enrollment failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    logger
        .log_audit(&logging::entry(
            LogLevel::Info,
            LogCategory::KeySlotChange,
            format!("Recovery key enrolled for {}", device),
            None,
            None,
        ))
        .ok();

    Ok(stdout)
}
