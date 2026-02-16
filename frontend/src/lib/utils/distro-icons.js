/**
 * Inline SVG icons for Linux distros and Windows.
 * Used by OsPicker's graphical (BURG-style) mode.
 * Each icon is a complete SVG string sized for a 64x64 viewBox.
 */

const ubuntu = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#E95420"/>
  <circle cx="32" cy="18" r="5" fill="white"/>
  <circle cx="19" cy="40" r="5" fill="white"/>
  <circle cx="45" cy="40" r="5" fill="white"/>
  <circle cx="32" cy="32" r="8" stroke="white" stroke-width="2.5" fill="none"/>
  <line x1="32" y1="24" x2="32" y2="18" stroke="white" stroke-width="2"/>
  <line x1="26" y1="36" x2="21" y2="39" stroke="white" stroke-width="2"/>
  <line x1="38" y1="36" x2="43" y2="39" stroke="white" stroke-width="2"/>
</svg>`;

const debian = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#A80030"/>
  <path d="M35 14c3 1 6 3 8 6s3 6 3 10c0 5-2 10-6 13s-9 5-14 4c-4-1-8-4-10-8s-2-9 0-13c2-5 6-8 11-10 3-1 5-1 8-2z" fill="white" opacity="0.9"/>
  <path d="M33 18c2 0 5 2 6 4s2 5 2 8c0 4-2 7-5 10-3 2-6 3-10 2-3-1-6-3-7-6s-1-7 0-10c2-4 5-6 9-7 2-1 3-1 5-1z" fill="#A80030"/>
</svg>`;

const fedora = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#3C6EB4"/>
  <rect x="22" y="18" width="4" height="28" rx="2" fill="white"/>
  <rect x="22" y="18" width="18" height="4" rx="2" fill="white"/>
  <rect x="22" y="30" width="14" height="4" rx="2" fill="white"/>
  <circle cx="38" cy="38" r="8" stroke="white" stroke-width="3" fill="none"/>
</svg>`;

const arch = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#1793D1"/>
  <path d="M32 12L20 48h6l6-18 6 18h6L32 12z" fill="white"/>
  <path d="M32 24l-3 10h6l-3-10z" fill="#1793D1"/>
</svg>`;

const manjaro = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#35BF5C"/>
  <rect x="16" y="16" width="8" height="32" rx="1" fill="white"/>
  <rect x="28" y="16" width="8" height="32" rx="1" fill="white"/>
  <rect x="40" y="24" width="8" height="24" rx="1" fill="white"/>
</svg>`;

const opensuse = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#73BA25"/>
  <circle cx="32" cy="30" r="12" stroke="white" stroke-width="3" fill="none"/>
  <circle cx="28" cy="28" r="2.5" fill="white"/>
  <circle cx="36" cy="28" r="2.5" fill="white"/>
  <path d="M27 34c2 3 7 3 10 0" stroke="white" stroke-width="2" fill="none" stroke-linecap="round"/>
</svg>`;

const linuxmint = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#87CF3E"/>
  <rect x="18" y="22" width="28" height="20" rx="3" fill="white"/>
  <rect x="22" y="26" width="8" height="12" rx="1" fill="#87CF3E"/>
  <rect x="34" y="26" width="8" height="12" rx="1" fill="#87CF3E"/>
</svg>`;

const popos = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#48B9C7"/>
  <text x="32" y="40" text-anchor="middle" fill="white" font-family="sans-serif" font-size="20" font-weight="bold">Pop!</text>
</svg>`;

const endeavouros = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#7B3FA0"/>
  <path d="M32 12L16 48h32L32 12z" fill="none" stroke="white" stroke-width="3" stroke-linejoin="round"/>
  <path d="M32 22L22 44h20L32 22z" fill="white" opacity="0.3"/>
</svg>`;

const nobara = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#7B2D8B"/>
  <circle cx="32" cy="32" r="14" stroke="white" stroke-width="3" fill="none"/>
  <path d="M32 18v28M18 32h28" stroke="white" stroke-width="2.5"/>
  <circle cx="32" cy="32" r="5" fill="white"/>
</svg>`;

const bazzite = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#1A5CFF"/>
  <path d="M22 20h20v6H28v4h12v6H28v4h14v6H22V20z" fill="white"/>
</svg>`;

const cachyos = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#0DB7ED"/>
  <path d="M40 22c-6-4-14-2-18 4s-2 14 4 18" stroke="white" stroke-width="3.5" fill="none" stroke-linecap="round"/>
  <circle cx="34" cy="34" r="6" fill="white"/>
</svg>`;

const windows = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#0078D4"/>
  <rect x="16" y="18" width="14" height="12" rx="1" fill="white"/>
  <rect x="34" y="16" width="14" height="14" rx="1" fill="white"/>
  <rect x="16" y="34" width="14" height="12" rx="1" fill="white"/>
  <rect x="34" y="34" width="14" height="14" rx="1" fill="white"/>
</svg>`;

const nixos = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#5277C3"/>
  <path d="M32 16l-14 24h8l6-10 6 10h8L32 16z" fill="white" opacity="0.9"/>
  <path d="M24 44l8-14 8 14H24z" fill="white" opacity="0.5"/>
</svg>`;

const gentoo = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#54487A"/>
  <path d="M38 16c-8 4-16 12-16 20 0 6 4 12 12 14 6-2 10-8 10-16 0-6-2-12-6-18z" fill="white" opacity="0.9"/>
</svg>`;

const voidlinux = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#478061"/>
  <rect x="18" y="18" width="28" height="28" rx="4" fill="none" stroke="white" stroke-width="3"/>
  <path d="M26 26l12 12M38 26L26 38" stroke="white" stroke-width="3" stroke-linecap="round"/>
</svg>`;

const alpine = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#0D597F"/>
  <path d="M32 16L18 44h28L32 16z" fill="white"/>
  <path d="M22 32L14 44h16L22 32z" fill="white" opacity="0.6"/>
</svg>`;

const rocky = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#10B981"/>
  <path d="M20 44l12-28 12 28H20z" fill="white" opacity="0.9"/>
  <path d="M26 44l6-14 6 14H26z" fill="#10B981"/>
</svg>`;

const alma = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#0F4266"/>
  <circle cx="32" cy="32" r="10" fill="white"/>
  <circle cx="32" cy="32" r="5" fill="#0F4266"/>
  <circle cx="32" cy="18" r="3" fill="white"/>
  <circle cx="32" cy="46" r="3" fill="white"/>
  <circle cx="18" cy="32" r="3" fill="white"/>
  <circle cx="46" cy="32" r="3" fill="white"/>
</svg>`;

const rhel = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#EE0000"/>
  <text x="32" y="38" text-anchor="middle" fill="white" font-family="sans-serif" font-size="14" font-weight="bold">RH</text>
  <circle cx="32" cy="32" r="16" stroke="white" stroke-width="2" fill="none"/>
</svg>`;

const centos = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#932279"/>
  <rect x="20" y="20" width="10" height="10" fill="#F0C800"/>
  <rect x="34" y="20" width="10" height="10" fill="#9CCD2A"/>
  <rect x="20" y="34" width="10" height="10" fill="#262577"/>
  <rect x="34" y="34" width="10" height="10" fill="white"/>
</svg>`;

// Generic Linux (Tux-inspired)
const linux = `<svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
  <circle cx="32" cy="32" r="30" fill="#FCC624"/>
  <ellipse cx="32" cy="36" rx="12" ry="16" fill="white"/>
  <ellipse cx="32" cy="36" rx="10" ry="14" fill="#333"/>
  <circle cx="28" cy="28" r="3" fill="white"/>
  <circle cx="36" cy="28" r="3" fill="white"/>
  <circle cx="28" cy="28" r="1.5" fill="black"/>
  <circle cx="36" cy="28" r="1.5" fill="black"/>
  <ellipse cx="32" cy="34" rx="3" ry="2" fill="#FCC624"/>
  <ellipse cx="32" cy="42" rx="8" ry="6" fill="white"/>
</svg>`;

// Map distro icon keys to SVGs
const iconMap = {
  ubuntu,
  debian,
  fedora,
  arch,
  manjaro,
  opensuse,
  linuxmint,
  'linux-mint': linuxmint,
  'pop-os': popos,
  'pop_os': popos,
  popos,
  endeavouros,
  nobara,
  bazzite,
  cachyos,
  windows,
  nixos,
  gentoo,
  void: voidlinux,
  voidlinux,
  alpine,
  rocky,
  rockylinux: rocky,
  alma,
  almalinux: alma,
  rhel,
  'redhat': rhel,
  centos,
  linux,
  // Aliases
  kubuntu: ubuntu,
  xubuntu: ubuntu,
  lubuntu: ubuntu,
  'ubuntu-mate': ubuntu,
  'ubuntu-budgie': ubuntu,
  'ubuntu-unity': ubuntu,
  kali: debian,
  mx: debian,
  'mx-linux': debian,
  garuda: arch,
  artix: arch,
  'arco': arch,
  arcolinux: arch,
  'opensuse-tumbleweed': opensuse,
  'opensuse-leap': opensuse,
  suse: opensuse,
  sles: opensuse,
  zorin: ubuntu,
  elementary: ubuntu,
  tuxedo: ubuntu,
};

/**
 * Get the SVG icon markup for a distro.
 * @param {string} distroIcon - The distro_icon field from a boot entry
 * @returns {string|null} SVG string or null if no icon found
 */
export function getDistroIcon(distroIcon) {
  if (!distroIcon) return null;
  return iconMap[distroIcon.toLowerCase()] || null;
}

/**
 * Get all available distro icon keys (for testing/debugging).
 */
export function getAvailableIcons() {
  return Object.keys(iconMap);
}
