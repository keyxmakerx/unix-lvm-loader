#!/usr/bin/env bash
#
# unix-lvm-loader build script
# Self-contained — downloads Node.js and Rust locally if not found.
# Tested on Nobara / Fedora. Should work on any RPM or DEB distro.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/.build-tools"
NODE_VERSION="22.14.0"
NODE_DIR="$BUILD_DIR/node-v${NODE_VERSION}-linux-x64"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }
step()  { echo -e "\n${BOLD}==> $*${NC}"; }

# ─── Step 1: System dependencies ────────────────────────────────────────────

step "Checking system dependencies"

MISSING_PKGS=()

# Tauri build requirements + runtime deps
# https://tauri.app/start/prerequisites/#linux
REQUIRED_CMDS=(
    "gcc:gcc"
    "pkg-config:pkg-config"
    "wget:wget"
    "file:file"
    "tar:tar"
)

for entry in "${REQUIRED_CMDS[@]}"; do
    cmd="${entry%%:*}"
    pkg="${entry##*:}"
    if ! command -v "$cmd" &>/dev/null; then
        MISSING_PKGS+=("$pkg")
    fi
done

# Check for required -devel libraries via pkg-config
REQUIRED_LIBS=(
    "webkit2gtk-4.1:webkit2gtk4.1-devel"
    "gtk+-3.0:gtk3-devel"
    "libsoup-3.0:libsoup3-devel"
    "glib-2.0:glib2-devel"
    "cairo:cairo-devel"
    "gdk-pixbuf-2.0:gdk-pixbuf2-devel"
    "pango:pango-devel"
    "openssl:openssl-devel"
    "atk:atk-devel"
)

# DEB-based equivalents (detected below)
REQUIRED_LIBS_DEB=(
    "webkit2gtk-4.1:libwebkit2gtk-4.1-dev"
    "gtk+-3.0:libgtk-3-dev"
    "libsoup-3.0:libsoup-3.0-dev"
    "glib-2.0:libglib2.0-dev"
    "cairo:libcairo2-dev"
    "gdk-pixbuf-2.0:libgdk-pixbuf-2.0-dev"
    "pango:libpango1.0-dev"
    "openssl:libssl-dev"
    "atk:libatk1.0-dev"
)

# Detect package manager
if command -v dnf &>/dev/null; then
    PKG_MGR="dnf"
    INSTALL_CMD="sudo dnf install -y"
    LIB_LIST=("${REQUIRED_LIBS[@]}")
    # AppImage needs FUSE
    if ! rpm -q fuse-libs &>/dev/null 2>&1 && ! rpm -q fuse3-libs &>/dev/null 2>&1; then
        MISSING_PKGS+=("fuse-libs")
    fi
    # librsvg2-devel for AppImage icon rendering
    if ! rpm -q librsvg2-devel &>/dev/null 2>&1; then
        MISSING_PKGS+=("librsvg2-devel")
    fi
elif command -v apt-get &>/dev/null; then
    PKG_MGR="apt"
    INSTALL_CMD="sudo apt-get install -y"
    LIB_LIST=("${REQUIRED_LIBS_DEB[@]}")
    if ! dpkg -l libfuse2 &>/dev/null 2>&1; then
        MISSING_PKGS+=("libfuse2")
    fi
    if ! dpkg -l librsvg2-dev &>/dev/null 2>&1; then
        MISSING_PKGS+=("librsvg2-dev")
    fi
else
    warn "Unknown package manager. You may need to install dependencies manually."
    PKG_MGR=""
    INSTALL_CMD=""
    LIB_LIST=("${REQUIRED_LIBS[@]}")
fi

if command -v pkg-config &>/dev/null; then
    for entry in "${LIB_LIST[@]}"; do
        lib="${entry%%:*}"
        pkg="${entry##*:}"
        if ! pkg-config --exists "$lib" 2>/dev/null; then
            MISSING_PKGS+=("$pkg")
        fi
    done
fi

if [ ${#MISSING_PKGS[@]} -gt 0 ]; then
    warn "Missing system packages: ${MISSING_PKGS[*]}"
    if [ -n "$INSTALL_CMD" ]; then
        info "Installing with: $INSTALL_CMD ${MISSING_PKGS[*]}"
        $INSTALL_CMD "${MISSING_PKGS[@]}" || error "Failed to install system packages. Try manually:\n  $INSTALL_CMD ${MISSING_PKGS[*]}"
    else
        error "Please install these packages manually: ${MISSING_PKGS[*]}"
    fi
fi
info "System dependencies OK"

# ─── Step 2: Node.js (local, no system install) ─────────────────────────────

step "Setting up Node.js"

if [ -x "$NODE_DIR/bin/node" ]; then
    info "Using local Node.js $(${NODE_DIR}/bin/node --version) from $NODE_DIR"
else
    info "Downloading Node.js v${NODE_VERSION} (local, won't touch your system)..."
    mkdir -p "$BUILD_DIR"
    NODE_TAR="node-v${NODE_VERSION}-linux-x64.tar.xz"
    NODE_URL="https://nodejs.org/dist/v${NODE_VERSION}/${NODE_TAR}"

    if command -v curl &>/dev/null; then
        curl -fSL --progress-bar -o "$BUILD_DIR/$NODE_TAR" "$NODE_URL"
    elif command -v wget &>/dev/null; then
        wget -q --show-progress -O "$BUILD_DIR/$NODE_TAR" "$NODE_URL"
    else
        error "Need curl or wget to download Node.js"
    fi

    tar xf "$BUILD_DIR/$NODE_TAR" -C "$BUILD_DIR"
    rm -f "$BUILD_DIR/$NODE_TAR"
    info "Node.js v${NODE_VERSION} installed to $NODE_DIR"
fi

export PATH="$NODE_DIR/bin:$PATH"

# ─── Step 3: Rust toolchain ─────────────────────────────────────────────────

step "Setting up Rust"

if command -v rustc &>/dev/null && command -v cargo &>/dev/null; then
    RUST_VER="$(rustc --version | awk '{print $2}')"
    info "Using system Rust $RUST_VER"
else
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi

    if ! command -v rustc &>/dev/null; then
        info "Installing Rust via rustup (user-local, no sudo needed)..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        source "$HOME/.cargo/env"
    fi
    info "Using Rust $(rustc --version | awk '{print $2}')"
fi

# ─── Step 4: Install frontend dependencies ──────────────────────────────────

step "Installing frontend dependencies"

cd "$SCRIPT_DIR/frontend"

if [ -d "node_modules" ] && [ -f "node_modules/.package-lock.json" ]; then
    info "node_modules exists, running npm ci for reproducible install..."
    npm ci --no-audit --no-fund 2>&1 | tail -3
else
    info "Installing npm packages..."
    npm install --no-audit --no-fund 2>&1 | tail -3
fi

# ─── Step 5: Build ──────────────────────────────────────────────────────────

step "Building unix-lvm-loader"

info "Building frontend + Rust backend + packaging..."
npx tauri build 2>&1 | while IFS= read -r line; do
    # Show key progress lines, suppress noise
    case "$line" in
        *"Compiling unix-lvm-loader"*) info "$line" ;;
        *"Finished"*)                  info "$line" ;;
        *"Bundling"*)                  info "$line" ;;
        *"built in"*)                  info "$line" ;;
        *"warning:"*)                  warn "$line" ;;
        *"error"*)                     echo "$line" ;;
    esac
done

# ─── Step 6: Output ─────────────────────────────────────────────────────────

RELEASE_DIR="$SCRIPT_DIR/frontend/src-tauri/target/release"
BUNDLE_DIR="$RELEASE_DIR/bundle"

step "Build complete!"
echo ""

if [ -f "$BUNDLE_DIR/appimage/"*.AppImage ]; then
    APPIMAGE=$(ls "$BUNDLE_DIR/appimage/"*.AppImage)
    info "AppImage:  $APPIMAGE"
    SIZE=$(du -h "$APPIMAGE" | cut -f1)
    info "Size:      $SIZE"
fi
if [ -f "$BUNDLE_DIR/deb/"*.deb ]; then
    info "Deb:       $(ls "$BUNDLE_DIR/deb/"*.deb)"
fi
if [ -f "$BUNDLE_DIR/rpm/"*.rpm ]; then
    info "RPM:       $(ls "$BUNDLE_DIR/rpm/"*.rpm)"
fi

echo ""
echo -e "${BOLD}To run:${NC}"
echo "  chmod +x $APPIMAGE"
echo "  $APPIMAGE"
echo ""
echo -e "${BOLD}To run in demo mode (no real LUKS/LVM needed):${NC}"
echo "  UNIX_LVM_LOADER_DEMO=1 $APPIMAGE"
echo ""
