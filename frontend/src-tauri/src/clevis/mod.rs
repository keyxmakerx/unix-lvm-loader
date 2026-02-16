use crate::logging::{self, AuditLogger, LogCategory, LogLevel};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClevisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Clevis not installed")]
    NotInstalled,
    #[error("Tang server unreachable: {0}")]
    TangUnreachable(String),
    #[allow(dead_code)]
    #[error("Safety check failed: {0}")]
    SafetyCheck(String),
    #[allow(dead_code)]
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Status of the Clevis/Tang toolchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClevisStatus {
    pub clevis_installed: bool,
    pub clevis_luks_installed: bool,
    pub tang_client_installed: bool,
    pub clevis_version: Option<String>,
}

/// A Clevis binding on a LUKS device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClevisBinding {
    pub slot: u32,
    pub pin: String,           // "tang", "sss", "tpm2"
    pub config: String,        // JSON config string
    pub server_url: Option<String>,
}

/// Tang server advertisement info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TangAdvertisement {
    pub url: String,
    pub reachable: bool,
    pub thumbprint: Option<String>,
    pub advertisement_json: Option<String>,
}

/// Shamir Secret Sharing policy for multi-factor unlock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SssPolicy {
    pub threshold: u32,      // How many pins must succeed
    pub pins: Vec<SssPin>,   // The pins to combine
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SssPin {
    pub pin_type: String,    // "tang", "tpm2"
    pub config: serde_json::Value,
}

/// Check what Clevis components are installed
pub fn detect_status() -> ClevisStatus {
    let clevis_installed = which_exists("clevis");
    let clevis_luks_installed = which_exists("clevis-luks-bind");
    let tang_client_installed = clevis_installed; // tang is a clevis pin, not separate binary

    let clevis_version = if clevis_installed {
        Command::new("clevis")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    // Some versions output to stderr
                    let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                    if !stderr.is_empty() {
                        Some(stderr)
                    } else {
                        None
                    }
                }
            })
    } else {
        None
    };

    ClevisStatus {
        clevis_installed,
        clevis_luks_installed,
        tang_client_installed,
        clevis_version,
    }
}

/// Verify a Tang server is reachable and get its advertisement
pub fn verify_tang_server(url: &str, logger: &AuditLogger) -> Result<TangAdvertisement, ClevisError> {
    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::ClevisOperation,
            format!("Verifying Tang server at {}", url),
            None,
            None,
        ))
        .ok();

    // Fetch the advertisement using curl (more reliable than clevis for just checking)
    let adv_url = format!("{}/adv", url.trim_end_matches('/'));
    let output = Command::new("curl")
        .args(["-sf", "--connect-timeout", "10", &adv_url])
        .output()?;

    if !output.status.success() {
        logger
            .log_operation(&logging::entry(
                LogLevel::Warning,
                LogCategory::ClevisOperation,
                format!("Tang server unreachable at {}", url),
                None,
                None,
            ))
            .ok();

        return Ok(TangAdvertisement {
            url: url.to_string(),
            reachable: false,
            thumbprint: None,
            advertisement_json: None,
        });
    }

    let adv_json = String::from_utf8_lossy(&output.stdout).to_string();

    // Try to get the thumbprint using jose (if available)
    let thumbprint = get_tang_thumbprint(url);

    logger
        .log_operation(&logging::entry(
            LogLevel::Info,
            LogCategory::ClevisOperation,
            format!(
                "Tang server verified at {} (thumbprint: {})",
                url,
                thumbprint.as_deref().unwrap_or("unknown")
            ),
            Some(serde_json::json!({
                "url": url,
                "thumbprint": thumbprint,
            })),
            None,
        ))
        .ok();

    Ok(TangAdvertisement {
        url: url.to_string(),
        reachable: true,
        thumbprint,
        advertisement_json: Some(adv_json),
    })
}

/// Get the thumbprint of a Tang server's signing key
fn get_tang_thumbprint(url: &str) -> Option<String> {
    // Method 1: Use clevis tang helper if available
    // Method 2: Fetch adv and use jose to compute thumbprint
    let adv_url = format!("{}/adv", url.trim_end_matches('/'));

    let curl_output = Command::new("curl")
        .args(["-sf", &adv_url])
        .output()
        .ok()?;

    if !curl_output.status.success() {
        return None;
    }

    // Pipe the advertisement through jose to get thumbprint
    // jose fmt -j- -g payload -y -o- | jose jwk use -i- -r -u verify -o- | jose jwk thp -i-
    let jose_output = Command::new("bash")
        .args([
            "-c",
            &format!(
                "curl -sf '{}' | jose fmt -j- -g payload -y -o- | jose jwk use -i- -r -u verify -o- | jose jwk thp -i-",
                adv_url
            ),
        ])
        .output()
        .ok()?;

    if jose_output.status.success() {
        let thp = String::from_utf8_lossy(&jose_output.stdout).trim().to_string();
        if !thp.is_empty() {
            return Some(thp);
        }
    }

    None
}

/// List existing Clevis bindings on a LUKS device
pub fn list_bindings(device: &str, logger: &AuditLogger) -> Result<Vec<ClevisBinding>, ClevisError> {
    if !which_exists("clevis") {
        return Err(ClevisError::NotInstalled);
    }

    logger
        .log_operation(&logging::entry(
            LogLevel::Debug,
            LogCategory::ClevisOperation,
            format!("Listing Clevis bindings for {}", device),
            None,
            None,
        ))
        .ok();

    let output = Command::new("clevis")
        .args(["luks", "list", "-d", device])
        .output()?;

    if !output.status.success() {
        // clevis luks list returns non-zero if no bindings — that's OK
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No bindings") || stderr.is_empty() {
            return Ok(vec![]);
        }
        return Err(ClevisError::CommandFailed(format!(
            "clevis luks list failed: {}", stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_clevis_list(&stdout)
}

/// Parse output of `clevis luks list`
/// Format: "1: tang '{"url":"http://tang.example.com"}'"
fn parse_clevis_list(output: &str) -> Result<Vec<ClevisBinding>, ClevisError> {
    let mut bindings = Vec::new();
    let re = Regex::new(r#"^(\d+):\s+(\w+)\s+'(.+)'"#).unwrap();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(cap) = re.captures(trimmed) {
            let slot: u32 = cap[1].parse().unwrap_or(0);
            let pin = cap[2].to_string();
            let config = cap[3].to_string();

            // Extract server URL from tang config
            let server_url = if pin == "tang" {
                serde_json::from_str::<serde_json::Value>(&config)
                    .ok()
                    .and_then(|v| v["url"].as_str().map(|s| s.to_string()))
            } else {
                None
            };

            bindings.push(ClevisBinding {
                slot,
                pin,
                config,
                server_url,
            });
        }
    }

    Ok(bindings)
}

/// Bind a LUKS device to a Tang server for network-based auto-unlock
pub fn bind_tang(
    device: &str,
    tang_url: &str,
    thumbprint: Option<&str>,
    logger: &AuditLogger,
) -> Result<(), ClevisError> {
    if !which_exists("clevis") {
        return Err(ClevisError::NotInstalled);
    }

    // First verify the server is reachable
    let adv = verify_tang_server(tang_url, logger)?;
    if !adv.reachable {
        return Err(ClevisError::TangUnreachable(tang_url.to_string()));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Warning,
            LogCategory::ClevisOperation,
            format!("Binding {} to Tang server {}", device, tang_url),
            Some(serde_json::json!({
                "device": device,
                "tang_url": tang_url,
                "thumbprint": thumbprint,
            })),
            Some(format!("clevis luks unbind -d {} -s <slot>", device)),
        ))
        .ok();

    // Build the tang config
    let config = if let Some(thp) = thumbprint {
        format!(r#"{{"url":"{}","thp":"{}"}}"#, tang_url, thp)
    } else if let Some(thp) = adv.thumbprint.as_deref() {
        format!(r#"{{"url":"{}","thp":"{}"}}"#, tang_url, thp)
    } else {
        // Without a thumbprint, clevis will prompt for trust-on-first-use
        // We pass -y to auto-accept (user has already verified via our UI)
        format!(r#"{{"url":"{}"}}"#, tang_url)
    };

    let output = Command::new("clevis")
        .args(["luks", "bind", "-y", "-d", device, "tang", &config])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        logger
            .log_audit(&logging::entry(
                LogLevel::Error,
                LogCategory::ClevisOperation,
                format!("Tang binding FAILED for {}: {}", device, stderr),
                None,
                None,
            ))
            .ok();
        return Err(ClevisError::CommandFailed(format!(
            "clevis luks bind failed: {}", stderr
        )));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Info,
            LogCategory::ClevisOperation,
            format!("Tang binding successful for {} → {}", device, tang_url),
            Some(serde_json::json!({
                "device": device,
                "tang_url": tang_url,
            })),
            Some(format!(
                "To remove: clevis luks unbind -d {} -s <slot>",
                device
            )),
        ))
        .ok();

    Ok(())
}

/// Bind a LUKS device using Shamir Secret Sharing (multi-factor)
pub fn bind_sss(
    device: &str,
    policy: &SssPolicy,
    logger: &AuditLogger,
) -> Result<(), ClevisError> {
    if !which_exists("clevis") {
        return Err(ClevisError::NotInstalled);
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Warning,
            LogCategory::ClevisOperation,
            format!(
                "Binding {} with SSS policy (threshold: {}/{} pins)",
                device, policy.threshold, policy.pins.len()
            ),
            Some(serde_json::to_value(policy).unwrap_or_default()),
            Some(format!("clevis luks unbind -d {} -s <slot>", device)),
        ))
        .ok();

    // Build SSS config JSON
    let pins_json: serde_json::Map<String, serde_json::Value> = policy
        .pins
        .iter()
        .fold(serde_json::Map::new(), |mut map, pin| {
            map.insert(pin.pin_type.clone(), pin.config.clone());
            map
        });

    let sss_config = serde_json::json!({
        "t": policy.threshold,
        "pins": pins_json,
    });

    let config_str = serde_json::to_string(&sss_config)
        .map_err(|e| ClevisError::CommandFailed(e.to_string()))?;

    let output = Command::new("clevis")
        .args(["luks", "bind", "-y", "-d", device, "sss", &config_str])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        logger
            .log_audit(&logging::entry(
                LogLevel::Error,
                LogCategory::ClevisOperation,
                format!("SSS binding FAILED for {}: {}", device, stderr),
                None,
                None,
            ))
            .ok();
        return Err(ClevisError::CommandFailed(format!(
            "clevis luks bind sss failed: {}", stderr
        )));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Info,
            LogCategory::ClevisOperation,
            format!("SSS binding successful for {}", device),
            None,
            Some(format!("clevis luks unbind -d {} -s <slot>", device)),
        ))
        .ok();

    Ok(())
}

/// Remove a Clevis binding from a specific slot
pub fn unbind(
    device: &str,
    slot: u32,
    logger: &AuditLogger,
) -> Result<(), ClevisError> {
    if !which_exists("clevis") {
        return Err(ClevisError::NotInstalled);
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Warning,
            LogCategory::ClevisOperation,
            format!("Removing Clevis binding on {} slot {}", device, slot),
            Some(serde_json::json!({
                "device": device,
                "slot": slot,
            })),
            None,
        ))
        .ok();

    let slot_str = slot.to_string();
    let output = Command::new("clevis")
        .args(["luks", "unbind", "-f", "-d", device, "-s", &slot_str])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ClevisError::CommandFailed(format!(
            "clevis luks unbind failed: {}", stderr
        )));
    }

    logger
        .log_audit(&logging::entry(
            LogLevel::Info,
            LogCategory::ClevisOperation,
            format!("Clevis binding removed from {} slot {}", device, slot),
            None,
            None,
        ))
        .ok();

    Ok(())
}

/// Get install instructions for the current distro
pub fn get_install_instructions() -> String {
    // Try to detect distro family
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        if content.contains("ID=fedora") || content.contains("ID=nobara") || content.contains("ID=bazzite") {
            return "sudo dnf install clevis clevis-luks clevis-dracut".to_string();
        }
        if content.contains("ID=ubuntu") || content.contains("ID=debian") || content.contains("ID=linuxmint") {
            return "sudo apt install clevis clevis-luks clevis-initramfs".to_string();
        }
        if content.contains("ID=arch") || content.contains("ID=cachyos") || content.contains("ID=endeavouros") {
            return "sudo pacman -S clevis".to_string();
        }
        if content.contains("ID=opensuse") {
            return "sudo zypper install clevis clevis-luks".to_string();
        }
    }
    "Install the 'clevis' and 'clevis-luks' packages for your distribution".to_string()
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clevis_list_tang() {
        let output = "1: tang '{\"url\":\"http://tang.example.com:7500\"}'\n";
        let bindings = parse_clevis_list(output).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].slot, 1);
        assert_eq!(bindings[0].pin, "tang");
        assert_eq!(bindings[0].server_url, Some("http://tang.example.com:7500".to_string()));
    }

    #[test]
    fn test_parse_clevis_list_multiple() {
        let output = "1: tang '{\"url\":\"http://tang1.local:7500\"}'\n3: tang '{\"url\":\"http://tang2.local:7500\"}'\n";
        let bindings = parse_clevis_list(output).unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].slot, 1);
        assert_eq!(bindings[1].slot, 3);
        assert_eq!(bindings[1].server_url, Some("http://tang2.local:7500".to_string()));
    }

    #[test]
    fn test_parse_clevis_list_sss() {
        let output = "2: sss '{\"t\":1,\"pins\":{\"tang\":{\"url\":\"http://tang.local\"}}}'\n";
        let bindings = parse_clevis_list(output).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].slot, 2);
        assert_eq!(bindings[0].pin, "sss");
        assert_eq!(bindings[0].server_url, None);
    }

    #[test]
    fn test_parse_clevis_list_tpm2() {
        let output = "1: tpm2 '{\"hash\":\"sha256\",\"key\":\"ecc\"}'\n";
        let bindings = parse_clevis_list(output).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].pin, "tpm2");
        assert_eq!(bindings[0].server_url, None);
    }

    #[test]
    fn test_parse_clevis_list_empty() {
        let bindings = parse_clevis_list("").unwrap();
        assert!(bindings.is_empty());
    }

    #[test]
    fn test_parse_clevis_list_whitespace() {
        let bindings = parse_clevis_list("\n  \n\n").unwrap();
        assert!(bindings.is_empty());
    }

    #[test]
    fn test_parse_clevis_list_with_thumbprint() {
        let output = "1: tang '{\"url\":\"http://tang.local\",\"thp\":\"dGVzdHRodW1icHJpbnQ\"}'\n";
        let bindings = parse_clevis_list(output).unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].server_url, Some("http://tang.local".to_string()));
        assert!(bindings[0].config.contains("thp"));
    }

    #[test]
    fn test_detect_status_returns_struct() {
        let status = detect_status();
        // clevis_installed and tang_client_installed should be the same
        assert_eq!(status.clevis_installed, status.tang_client_installed);
    }

    #[test]
    fn test_get_install_instructions_returns_string() {
        let instructions = get_install_instructions();
        assert!(!instructions.is_empty());
        assert!(instructions.to_lowercase().contains("clevis"));
    }
}
