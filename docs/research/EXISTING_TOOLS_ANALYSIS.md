# Existing LUKS/LVM Tools Analysis

> Research conducted Feb 2026 to inform unix-lvm-loader architecture decisions.

## Tools Studied

### 1. zuluCrypt (Most Feature-Rich Existing Tool)
- **Repo:** https://github.com/mhogomchungu/zuluCrypt
- **Architecture:** C backend (zuluCrypt-cli) + Qt GUI (zuluCrypt-gui)
- **Supports:** LUKS1/2, PLAIN dm-crypt, TrueCrypt, VeraCrypt, BitLocker
- **Distros:** Fedora, Debian, Ubuntu repos; AUR for Arch
- **Backup:** Can backup/restore LUKS headers
- **No TPM2/FIDO2/Clevis support**

**What worked:**
- Comprehensive format support
- Header backup/restore built in
- Separate CLI and GUI components

**What failed (lessons for us):**
- Mount point creation uses non-standard paths → file managers can't find them
- GUI freezes when creating encrypted containers (no async)
- Library updates (libgcrypt) broke the app → pin dependencies
- Paths with accent characters not supported → use proper Unicode handling
- Too complex for beginners despite broad feature set → keep it simple

### 2. luckyLUKS (Best UX Design)
- **Repo:** https://github.com/jas-per/luckyLUKS
- **Architecture:** Python GUI with privilege separation
- **Key insight:** UI runs as normal user, privileged ops in separate helper processes

**What worked:**
- Excellent security model (privilege separation)
- Simple, focused interface
- Desktop shortcuts for quick access

**What failed:**
- Too simple for power users
- No advanced features (TPM2, etc.)

### 3. GNOME Disks
- Built into GNOME desktop
- Can create/manage LUKS encrypted partitions
- Limited to ext4 for encrypted partitions
- No TPM2/FIDO2 GUI
- Cannot configure root filesystem encryption

### 4. KDE Partition Manager
- First GUI tool supporting LUKS partition resizing
- LUKS2 support added in v3.3 (described as "initial")
- Hardcoded LUKS2 settings less secure than manual config
- LUKS partitions can only be moved when closed

### 5. n01d-forge (Only Tauri-based LUKS tool found)
- **Repo:** https://github.com/bad-antics/n01d-forge
- **Architecture:** Tauri desktop app
- Image burning tool with LUKS2 encryption support
- Proves Tauri + LUKS is viable architecture
- Different use case (image burning vs system encryption management)

### 6. Cockpit (Web-based, RHEL)
- Web console with LUKS management via Storage page
- Integrates with Clevis/Tang for auto-unlock
- Cannot configure LUKS on root filesystem
- Good model for remote/headless management

### 7. systemd-cryptenroll (CLI only — gap we fill)
- **THE key tool** for modern LUKS2 management
- Enrolls TPM2, FIDO2, PKCS#11 into LUKS2 volumes
- **No GUI exists for this** — this is our primary opportunity
- LUKS2 only (stores metadata in JSON token area)

### 8. Clevis/Tang (CLI only)
- Network-bound disk encryption (NBDE)
- Automated LUKS unlocking via network
- No dedicated GUI (only Cockpit integration)
- Intermittent auto-unlock failures reported

## Critical Gap We Fill

**No mainstream GUI tool exists that combines:**
1. TPM2 enrollment (systemd-cryptenroll GUI)
2. FIDO2 enrollment
3. Clevis/Tang configuration
4. LUKS header backup/restore
5. Multi-distro support with proper initramfs handling
6. Comprehensive logging and audit trail

## Key Lessons for Our Architecture

1. **Privilege separation** (luckyLUKS model) — UI as normal user, ops via polkit/sudo
2. **Async everything** — zuluCrypt's GUI freezing is unacceptable for a system tool
3. **Standard mount points** — don't invent custom paths
4. **Pin dependencies** — don't let libcryptsetup updates break us
5. **Focus** — don't try to support TrueCrypt/VeraCrypt/BitLocker; just LUKS2
6. **Backup first** — every destructive operation must offer backup first
7. **Logging** — no existing tool has proper audit logging; we differentiate here
