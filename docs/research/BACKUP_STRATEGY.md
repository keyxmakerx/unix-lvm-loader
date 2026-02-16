# Backup & Recovery Strategy

> Every destructive operation MUST offer backup first. This is non-negotiable.

## Backup Layers

### Layer 1: LUKS Header Backup (CRITICAL)

**Why:** Without the LUKS header, encrypted data is permanently lost. A corrupted header = total data loss.

**Backup command:**
```bash
cryptsetup luksHeaderBackup /dev/sdX --header-backup-file /path/to/backup.img
```

**Restore command:**
```bash
cryptsetup luksHeaderRestore /dev/sdX --header-backup-file /path/to/backup.img
```

**Our implementation rules:**
1. **Automatic header backup** before ANY key slot modification
2. **Automatic header backup** before re-encryption
3. **Automatic header backup** before TPM2/FIDO2 enrollment
4. Backup files stored with timestamp: `luks-header-{UUID}-{YYYYMMDD-HHMMSS}.img`
5. User prompted for backup location (default: `~/.local/share/unix-lvm-loader/backups/`)
6. Backup file integrity verified with SHA-256 checksum stored alongside
7. **Warning:** Backup files contain key material — must be stored encrypted or on secure media

### Layer 2: Key Slot Inventory

**LUKS2 supports 32 key slots.** Before modifying slots:
1. Enumerate all active slots via `cryptsetup luksDump`
2. Log slot state (which slots are active, their types)
3. Ensure at least one passphrase slot remains active after any operation
4. **NEVER allow removal of the last passphrase slot** — this is unrecoverable

**Our safety check:**
```
IF (operation removes a key slot) AND (remaining_passphrase_slots <= 1):
    BLOCK the operation
    WARN: "This would remove your last passphrase. You would be locked out."
```

### Layer 3: crypttab Backup

Before modifying `/etc/crypttab`:
1. Copy to `/etc/crypttab.bak.{timestamp}`
2. Also store in our backup directory
3. Log the diff between old and new crypttab

### Layer 4: Initramfs Backup

Before rebuilding initramfs:
1. Copy current initramfs image
2. Store path and checksum
3. If rebuild fails, provide restore instructions

**Per-distro backup locations:**
- Debian/Ubuntu: `/boot/initrd.img-$(uname -r)` → backup as `.bak`
- Fedora/SUSE: `/boot/initramfs-$(uname -r).img` → backup as `.bak`
- Arch: `/boot/initramfs-linux.img` → backup as `.bak`

### Layer 5: Boot Config Backup

Before modifying GRUB/systemd-boot config:
1. Backup `/etc/default/grub` (or equivalent)
2. Backup generated GRUB config
3. Log changes made

---

## Recovery Scenarios

### Scenario 1: LUKS Header Corrupted
**Symptoms:** `cryptsetup open` fails, `luksDump` shows corruption
**Recovery:**
```bash
# From live USB:
cryptsetup luksHeaderRestore /dev/sdX --header-backup-file /path/to/backup.img
```
**Our tool provides:** One-click header restore from backup archive

### Scenario 2: Re-encryption Interrupted
**Symptoms:** System crash during online re-encryption
**Recovery:**
- LUKS2 has built-in auto-recovery on next open
- Check status: `cryptsetup luksDump /dev/sdX` — look for reencryption segment
- Resume: `cryptsetup reencrypt --resume-only /dev/sdX`
**Our tool provides:** Detect interrupted re-encryption and offer resume

### Scenario 3: TPM2 Enrollment Failed Mid-Process
**Symptoms:** TPM2 token enrolled but initramfs not updated
**Recovery:**
1. System still boots with passphrase (we never remove passphrase first)
2. Re-run enrollment or remove failed TPM2 token
3. Rebuild initramfs
**Our tool provides:** Enrollment is atomic — either fully succeeds or rolls back

### Scenario 4: Boot Failure After crypttab/initramfs Change
**Symptoms:** System hangs at boot, drops to initramfs shell
**Recovery:**
1. Boot from live USB
2. Mount root filesystem
3. Restore crypttab from backup
4. Rebuild initramfs from chroot
5. Reboot
**Our tool provides:** Step-by-step guided recovery mode

### Scenario 5: Forgotten Passphrase
**Symptoms:** User can't unlock volume
**Recovery:**
- If another key slot exists (e.g., recovery key): use that
- If LUKS header backup exists with old passphrase: restore header, use old passphrase
- If no backup and no other slot: **PERMANENT DATA LOSS — no recovery possible**
**Our tool provides:**
- Strongly encourage setting up recovery key during initial setup
- Export recovery key to printable format (like BitLocker recovery key)
- QR code option for recovery key

---

## Backup Storage Strategy

### Default backup location
```
~/.local/share/unix-lvm-loader/
├── backups/
│   ├── headers/
│   │   ├── luks-header-{UUID}-20260216-143000.img
│   │   └── luks-header-{UUID}-20260216-143000.img.sha256
│   ├── crypttab/
│   │   └── crypttab.20260216-143000
│   ├── initramfs/
│   │   └── initramfs-6.8.0.img.20260216-143000
│   └── boot-config/
│       └── grub.default.20260216-143000
├── logs/
│   ├── operations.log       # All operations with timestamps
│   └── audit.log            # Security-relevant events
└── config/
    └── settings.toml
```

### External backup option
- User can specify external USB/drive for backups
- Option to encrypt backups with a separate passphrase
- Export backup bundle as single encrypted archive

---

## Operation Safety Protocol

**Every destructive operation follows this sequence:**

```
1. PRE-FLIGHT CHECK
   ├── Verify LUKS volume is accessible
   ├── Check available key slots
   ├── Verify backup storage has space
   └── Log operation intent

2. BACKUP PHASE
   ├── Create LUKS header backup
   ├── Verify backup integrity (read back + checksum)
   ├── Backup crypttab if will be modified
   └── Backup initramfs if will be rebuilt

3. CONFIRMATION
   ├── Show user exactly what will happen
   ├── Show backup locations
   ├── Require explicit confirmation
   └── For critical ops: require typing confirmation phrase

4. EXECUTION
   ├── Execute operation
   ├── Log every step with timestamps
   ├── On failure: attempt rollback
   └── On success: verify result

5. VERIFICATION
   ├── Verify LUKS volume still accessible
   ├── Verify key slots in expected state
   ├── Test new configuration if possible
   └── Log final state

6. POST-OP
   ├── Show summary to user
   ├── Remind about backup locations
   └── Update audit log
```
