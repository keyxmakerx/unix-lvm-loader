# Multi-Distro LUKS/LVM Support Matrix

> How each target distro handles encryption, boot, and initramfs.

## Distro Detection Strategy

**Recommended detection hierarchy:**
1. Parse `/etc/os-release` → `ID`, `ID_LIKE`, `VARIANT_ID` fields
2. Check for `ostree` command → identifies immutable Fedora variants
3. Detect initramfs tool: `dracut` vs `mkinitcpio` vs `update-initramfs`
4. Detect package manager: `dnf`, `apt`, `pacman`, `zypper`, `rpm-ostree`
5. Check specific release files as fallback

```
Ubuntu:        ID=ubuntu,           ID_LIKE=debian
Debian:        ID=debian
Fedora:        ID=fedora
Nobara:        ID_LIKE=fedora       (check /etc/nobara-release)
Bazzite:       ID=fedora,           VARIANT_ID contains "bazzite"  (IMMUTABLE)
Fedora Atomic: ID=fedora,           VARIANT_ID=silverblue|kinoite  (IMMUTABLE)
Arch:          ID=arch
CachyOS:       ID=cachyos,          ID_LIKE=arch
openSUSE:      ID=opensuse-leap | opensuse-tumbleweed
```

---

## Per-Distro Analysis

### Ubuntu / Debian Family

| Property | Value |
|----------|-------|
| LUKS version | LUKS2 (20.04+), LUKS1 (18.04) |
| Initramfs | `initramfs-tools` (NOT dracut) |
| Boot loader | GRUB2 |
| crypttab location | `/etc/crypttab` |
| crypttab format | Debian-specific extensions: `check`, `checkarguments`, `initramfs`, `keyscript` |
| Immutable | No |
| Initramfs rebuild | `update-initramfs -u` |
| Package manager | `apt` |

**Gotchas:**
- `initramfs-tools` does NOT support TPM2 devices natively (dracut recommended for TPM)
- Debian 13 changed from `initramfs` option to `x-initrd.attach` in crypttab
- Custom keyscripts possible: `keyscript=/path/to/script.sh`

**TPM2 Status:** Available but requires patches/workarounds. Clevis-tpm2 in repos.

---

### Fedora / Nobara

| Property | Value |
|----------|-------|
| LUKS version | LUKS2 (default) |
| Initramfs | `dracut` |
| Boot loader | GRUB2 |
| crypttab location | `/etc/crypttab` |
| crypttab format | Standard + systemd options (`x-systemd.device-timeout=0`) |
| Immutable | No (standard Fedora/Nobara) |
| Initramfs rebuild | `dracut --force` |
| Package manager | `dnf` |

**Nobara specifics:**
- Switched from LUKS1 to LUKS2 in March 2023
- Uses Calamares installer
- Gaming-focused (WINE, codecs, drivers pre-installed)
- Completely independent project from Fedora

**Gotchas:**
- Anaconda installer only supports passphrases (not keyfiles) during install
- LUKS2 with Argon2 requires custom GRUB for boot partition

**TPM2 Status:** Native via `systemd-cryptenroll` (Fedora 35+). Full Clevis support.

---

### Bazzite / Fedora Atomic (IMMUTABLE)

| Property | Value |
|----------|-------|
| LUKS version | LUKS2 |
| Initramfs | `dracut` (with ostree integration) |
| Boot loader | GRUB2 |
| crypttab location | `/etc/crypttab` (on read-only root) |
| crypttab format | TPM options: `tpm2-device=auto,discard` |
| Immutable | **YES** (read-only root filesystem) |
| Initramfs rebuild | `rpm-ostree initramfs --enable` |
| Package manager | `rpm-ostree` (NOT dnf) |

**CRITICAL: Immutable filesystem handling:**
```bash
# Cannot directly edit system files
# Must use rpm-ostree for system modifications
rpm-ostree initramfs --enable --arg=--add --arg=crypt

# Kernel args for LUKS
# rd.luks.options=tpm2-device=auto
```

**Bazzite specifics:**
- Has `ujust setup-luks-tpm-unlock` convenience command
- Framework 16 LUKS TPM unlock doesn't prompt for fingerprint (known issue)
- Some users report keyfile-based crypttab entries failing with ujust

**TPM2 Status:** Native support. TPM2 in initramfs since Fedora 41+.

---

### Arch Linux

| Property | Value |
|----------|-------|
| LUKS version | LUKS1 or LUKS2 (user choice) |
| Initramfs | `mkinitcpio` |
| Boot loader | GRUB2 or systemd-boot (user choice) |
| crypttab location | `/etc/crypttab` |
| crypttab format | Standard (but mkinitcpio primarily uses kernel params via GRUB) |
| Immutable | No |
| Initramfs rebuild | `mkinitcpio -P` |
| Package manager | `pacman` |

**mkinitcpio hooks (ORDER MATTERS):**
```
# Traditional hooks:
HOOKS=(base udev autodetect modconf block encrypt lvm2 filesystems keyboard fsck)

# Systemd variant (more features):
HOOKS=(base systemd autodetect modconf block sd-encrypt sd-lvm2 filesystems keyboard fsck)
```

**Gotchas:**
- `encrypt` hook does NOT support unlocking multiple encrypted disks
- `encrypt` hook does NOT support detached LUKS headers
- `sd-encrypt` hook is more feature-rich than `encrypt` hook
- No automated installer — manual setup required
- ArchWiki recommends `grub-improved-luks2-git` AUR package for Argon2

**TPM2 Status:** Available. Clevis in community repos. Booster (alt initramfs) has better Clevis support.

---

### CachyOS (Arch-based)

| Property | Value |
|----------|-------|
| LUKS version | LUKS2 (as of Jan 2026) |
| Initramfs | `mkinitcpio` |
| Boot loader | GRUB2 (default), systemd-boot option |
| crypttab location | `/etc/crypttab` |
| Immutable | No |
| Initramfs rebuild | `mkinitcpio -P` |
| Package manager | `pacman` |

**Gotchas:**
- GUI installer recently switched from LUKS1 to LUKS2
- GRUB unlock can take ~1 minute on Btrfs (CPU limitations in GRUB)
- Keyboard layout during GRUB LUKS password entry may not match install selection (issue #254)
- Switching bootloaders on encrypted systems requires careful steps

**TPM2 Status:** Available via community repos. `cachy-chroot` has LUKS support.

---

### openSUSE (Leap / Tumbleweed)

| Property | Value |
|----------|-------|
| LUKS version | LUKS1 or LUKS2 (user choice in installer) |
| Initramfs | `dracut` |
| Boot loader | GRUB2 |
| crypttab location | `/etc/crypttab` |
| Immutable | No (MicroOS variant is, but less common) |
| Initramfs rebuild | `dracut --force` |
| Package manager | `zypper` |

**Gotchas:**
- Default cryptsetup uses Argon2id, but GRUB only supports PBKDF2
- For encrypted root, MUST use PBKDF2 KDF function
- Dracut keyfile setup: `dd if=/dev/random bs=256 count=1 of=/etc/cryptsetup-keys.d/$NAME.key`
- Must add `install_items+=" /etc/cryptsetup-keys.d/* "` to dracut.conf

**TPM2 Status:** Full TPM2 and FIDO2 support documented. Tested on Leap 15.0-15.4 and Tumbleweed.

---

## Initramfs Systems Comparison

| System | Distros | LUKS Support | Multi-disk | Rebuild Command |
|--------|---------|-------------|-----------|----------------|
| **dracut** | Fedora, openSUSE, RHEL | Native auto-detect | Yes | `dracut --force` |
| **mkinitcpio** | Arch, CachyOS, Manjaro | Via hooks (order matters) | Limited (`encrypt` hook) | `mkinitcpio -P` |
| **initramfs-tools** | Debian, Ubuntu | Via scripts | Yes | `update-initramfs -u` |

## Boot Loader LUKS2 Support

**GRUB2 + LUKS2 limitation:**
- GRUB supports LUKS2 but **ONLY with PBKDF2** (not Argon2)
- GRUB 2.06+ added initial LUKS2 support
- GRUB 2.12 improved but still no Argon2
- **Workarounds:**
  - Use PBKDF2 for `/boot` partition
  - Use AUR/community GRUB patches (Arch, CachyOS)
  - Use systemd-boot + dracut instead of GRUB

## TPM2/FIDO2/Clevis Support Matrix

| Distro | systemd-cryptenroll | Clevis | TPM2 Auto-unlock | Notes |
|--------|-------------------|--------|-----------------|-------|
| Ubuntu | Yes (newer) | Yes | Limited | initramfs-tools TPM issues |
| Debian | Limited | Yes | No (yet) | dracut replacement underway |
| Fedora | Yes (35+) | Yes | Yes | Best native support |
| Nobara | Yes | Yes | Yes | Same as Fedora |
| Bazzite | Yes | Yes | Yes | `ujust` helper available |
| Arch | Yes | Yes (community) | Yes | sd-encrypt hook recommended |
| CachyOS | Yes | Yes (community) | Yes | Same hooks as Arch |
| openSUSE | Yes | Yes | Yes | Well-documented |

## Our Implementation Strategy

### Distro Family Abstraction

```
DistroFamily::Debian    → Ubuntu, Debian, Linux Mint, Pop!_OS
DistroFamily::Fedora    → Fedora, Nobara
DistroFamily::Atomic    → Bazzite, Fedora Silverblue/Kinoite  (IMMUTABLE)
DistroFamily::Arch      → Arch, CachyOS, Manjaro, EndeavourOS
DistroFamily::Suse      → openSUSE Leap, Tumbleweed
```

### Per-Family Operations

| Operation | Debian | Fedora | Atomic | Arch | SUSE |
|-----------|--------|--------|--------|------|------|
| Rebuild initramfs | `update-initramfs -u` | `dracut --force` | `rpm-ostree initramfs --enable` | `mkinitcpio -P` | `dracut --force` |
| Install packages | `apt install` | `dnf install` | `rpm-ostree install` | `pacman -S` | `zypper install` |
| crypttab extras | `initramfs` or `x-initrd.attach` | standard | via kernel args | kernel params preferred | standard |
| Modify boot config | Edit `/etc/default/grub` + `update-grub` | Edit + `grub2-mkconfig` | Kernel args via `rpm-ostree kargs` | Edit + `grub-mkconfig` | Edit + `grub2-mkconfig` |
