use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum PrivilegeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not running as root and no privilege escalation available")]
    NoEscalation,
    #[error("Privilege escalation denied by user")]
    Denied,
    #[error("Command failed: {0}")]
    CommandFailed(String),
}

/// Current privilege level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivilegeLevel {
    Root,
    User,
}

/// Method available for privilege escalation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EscalationMethod {
    Pkexec,       // Polkit (graphical prompt)
    Sudo,         // Terminal sudo
    None,         // No escalation available
}

/// Check current privilege level and available escalation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegeInfo {
    pub level: PrivilegeLevel,
    pub escalation_method: EscalationMethod,
    pub uid: u32,
    pub username: String,
}

/// Detect current privileges
pub fn detect() -> PrivilegeInfo {
    let uid = libc_geteuid();
    let level = if uid == 0 {
        PrivilegeLevel::Root
    } else {
        PrivilegeLevel::User
    };

    let username = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| format!("uid:{}", uid));

    let escalation_method = if level == PrivilegeLevel::Root {
        EscalationMethod::None // Already root
    } else if has_pkexec() {
        EscalationMethod::Pkexec
    } else if has_sudo() {
        EscalationMethod::Sudo
    } else {
        EscalationMethod::None
    };

    PrivilegeInfo {
        level,
        escalation_method,
        uid,
        username,
    }
}

/// Execute a command with privilege escalation if needed
#[allow(dead_code)]
pub fn run_privileged(
    program: &str,
    args: &[&str],
    escalation: &EscalationMethod,
) -> Result<std::process::Output, PrivilegeError> {
    let info = detect();

    if info.level == PrivilegeLevel::Root {
        // Already root, run directly
        return Ok(Command::new(program).args(args).output()?);
    }

    match escalation {
        EscalationMethod::Pkexec => {
            let mut cmd_args = vec![program];
            cmd_args.extend_from_slice(args);
            let output = Command::new("pkexec").args(&cmd_args).output()?;

            if output.status.code() == Some(126) {
                return Err(PrivilegeError::Denied);
            }

            Ok(output)
        }
        EscalationMethod::Sudo => {
            let mut cmd_args = vec!["-n", program]; // -n for non-interactive
            cmd_args.extend(args.iter());
            let output = Command::new("sudo").args(&cmd_args).output()?;

            if !output.status.success() {
                // Try interactive sudo as fallback
                let mut cmd_args = vec![program];
                cmd_args.extend(args.iter());
                return Ok(Command::new("sudo").args(&cmd_args).output()?);
            }

            Ok(output)
        }
        EscalationMethod::None => Err(PrivilegeError::NoEscalation),
    }
}

fn has_pkexec() -> bool {
    Command::new("which")
        .arg("pkexec")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_sudo() -> bool {
    Command::new("which")
        .arg("sudo")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get effective user ID (without libc crate dependency)
fn libc_geteuid() -> u32 {
    // Read from /proc/self/status
    if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
        for line in content.lines() {
            if line.starts_with("Uid:") {
                // Format: Uid: real effective saved fs
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return parts[2].parse().unwrap_or(1000);
                }
            }
        }
    }

    // Fallback: check if we can write to /etc
    if std::fs::metadata("/etc/shadow")
        .map(|m| {
            use std::os::unix::fs::MetadataExt;
            m.uid() == 0
        })
        .unwrap_or(false)
    {
        // Try writing a temp file to /etc to check
        if std::fs::write("/tmp/.unix-lvm-loader-priv-check", "").is_ok() {
            std::fs::remove_file("/tmp/.unix-lvm-loader-priv-check").ok();
        }
    }

    // Default to non-root
    1000
}

/// Install a polkit policy file for passwordless access to specific commands
/// This is the recommended approach for GUI apps that need root for specific operations
#[allow(dead_code)]
pub fn install_polkit_policy() -> Result<(), PrivilegeError> {
    let policy = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC
 "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
<policyconfig>
  <vendor>Unix LVM Loader</vendor>
  <vendor_url>https://github.com/keyxmakerx/unix-lvm-loader</vendor_url>

  <action id="com.unixlvmloader.pkexec.run">
    <description>Run Unix LVM Loader privileged operations</description>
    <message>Authentication is required to manage disk encryption</message>
    <defaults>
      <allow_any>auth_admin</allow_any>
      <allow_inactive>auth_admin</allow_inactive>
      <allow_active>auth_admin_keep</allow_active>
    </defaults>
    <annotate key="org.freedesktop.policykit.exec.path">/usr/bin/unix-lvm-loader-helper</annotate>
  </action>
</policyconfig>
"#;

    let policy_path = "/usr/share/polkit-1/actions/com.unixlvmloader.policy";

    // This needs root to install
    std::fs::write(policy_path, policy)?;
    Ok(())
}
