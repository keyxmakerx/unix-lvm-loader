/**
 * Maps backend error messages to user-friendly messages with guidance.
 * Returns { title, message, hint } or null if no special handling needed.
 */
export function friendlyError(error) {
  const msg = error?.message || String(error);

  // Privilege / permission errors
  if (
    msg.includes('Permission denied') ||
    msg.includes('Operation not permitted') ||
    msg.includes('EPERM') ||
    msg.includes('EACCES')
  ) {
    return {
      title: 'Permission Denied',
      message: 'This operation requires root (administrator) privileges.',
      hint: 'Try running the application as root, or ensure polkit (pkexec) is available for graphical privilege escalation.',
    };
  }

  // cryptsetup not found
  if (msg.includes('cryptsetup') && (msg.includes('not found') || msg.includes('No such file'))) {
    return {
      title: 'cryptsetup Not Found',
      message: 'The cryptsetup utility is not installed on this system.',
      hint: 'Install it with your package manager: apt install cryptsetup / dnf install cryptsetup / pacman -S cryptsetup',
    };
  }

  // systemd-cryptenroll errors
  if (msg.includes('cryptenroll') && msg.includes('failed')) {
    if (msg.includes('No TPM2')) {
      return {
        title: 'No TPM2 Device Found',
        message: 'systemd-cryptenroll could not find a TPM2 device on this system.',
        hint: 'Ensure your system has a TPM2 chip and that the tpm2-tss drivers are installed. Check with: systemctl status tpm2-abrmd',
      };
    }
    if (msg.includes('FIDO2') || msg.includes('fido2')) {
      return {
        title: 'FIDO2 Enrollment Failed',
        message: 'Could not communicate with a FIDO2 security key.',
        hint: 'Ensure your security key is plugged in and that libfido2 is installed. You may need to touch the key during enrollment.',
      };
    }
  }

  // blkid / volume scanning issues
  if (msg.includes('blkid') && msg.includes('failed')) {
    return {
      title: 'Volume Scan Failed',
      message: 'Could not scan block devices. This usually requires root privileges.',
      hint: 'Run the application as root to scan all LUKS volumes.',
    };
  }

  // clevis not installed
  if (msg.includes('Clevis not installed') || msg.includes('NotInstalled')) {
    return {
      title: 'Clevis Not Installed',
      message: 'The Clevis toolkit for network-bound disk encryption is not available.',
      hint: 'Install Clevis with your package manager. The Network Unlock page will show install instructions for your distro.',
    };
  }

  // Tang server unreachable
  if (msg.includes('TangUnreachable') || msg.includes('Tang server unreachable')) {
    return {
      title: 'Tang Server Unreachable',
      message: 'Could not connect to the Tang key server.',
      hint: 'Verify the server URL and that the Tang service is running. Test with: curl -sf <url>/adv',
    };
  }

  // Generic command failures
  if (msg.includes('Command failed') || msg.includes('CommandFailed')) {
    return {
      title: 'Command Failed',
      message: msg.replace(/^Command failed:\s*/, '').replace(/^CommandFailed:\s*/, ''),
      hint: 'Check the Logs page for more details about what went wrong.',
    };
  }

  // Lock error (internal)
  if (msg.includes('LOCK_ERROR')) {
    return {
      title: 'Internal Error',
      message: 'Failed to acquire application state. This is a bug.',
      hint: 'Try restarting the application. If this persists, please report it.',
    };
  }

  return null;
}

/**
 * Format an error for display — returns the friendly version if available,
 * otherwise the raw message.
 */
export function formatError(error) {
  const friendly = friendlyError(error);
  if (friendly) return friendly;
  return {
    title: 'Error',
    message: error?.message || String(error),
    hint: null,
  };
}
