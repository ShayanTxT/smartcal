#!/usr/bin/env bash
set -euo pipefail

# SmartCal Installer for Linux Classroom Desktops
# Installs dependencies, builds release binary, installs desktop entry, icon, and non-root udev rule.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${PREFIX:-/usr/local}"
BIN_DIR="${PREFIX}/bin"

echo "==> SmartCal Classroom Calibration Installer"

install_dependencies() {
  if command -v apt-get >/dev/null 2>&1; then
    echo "==> Detected Debian/Ubuntu/Mint (apt-get). Installing dependencies..."
    sudo apt-get update
    sudo apt-get install -y \
      build-essential pkg-config curl \
      xinput x11-xserver-utils libinput-tools udev \
      libgl1-mesa-dev libegl1-mesa-dev libxkbcommon-dev \
      fonts-dejavu-core
    sudo apt-get install -y fonts-vazirmatn 2>/dev/null || true
  elif command -v dnf >/dev/null 2>&1; then
    echo "==> Detected Fedora/RHEL (dnf). Installing dependencies..."
    sudo dnf install -y \
      gcc gcc-c++ pkgconf-pkg-config curl \
      xinput xrandr libinput systemd-udev \
      mesa-libGL-devel mesa-libEGL-devel libxkbcommon-devel \
      dejavu-sans-fonts
  elif command -v pacman >/dev/null 2>&1; then
    echo "==> Detected Arch/Manjaro (pacman). Installing dependencies..."
    sudo pacman -S --needed --noconfirm \
      base-devel pkgconf curl \
      xorg-xinput xorg-xrandr libinput systemd \
      mesa libxkbcommon ttf-dejavu
  elif command -v zypper >/dev/null 2>&1; then
    echo "==> Detected openSUSE (zypper). Installing dependencies..."
    sudo zypper install -y \
      gcc gcc-c++ pkg-config curl \
      xinput xrandr libinput-tools udev \
      Mesa-libGL-devel Mesa-libEGL-devel libxkbcommon-devel \
      dejavu-fonts
  else
    echo "Warning: Unknown package manager. Ensure Rust, XInput/libinput tools, OpenGL, and libxkbcommon are installed."
  fi
}

if [[ "${1:-}" != "--skip-deps" ]]; then
  install_dependencies
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Rust toolchain not found; installing via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck source=/dev/null
  source "${HOME}/.cargo/env"
fi

echo "==> Building SmartCal (release)..."
cd "${ROOT_DIR}"
cargo build --release --bin smartcal

echo "==> Installing binary to ${BIN_DIR}/smartcal..."
sudo install -Dm755 "${ROOT_DIR}/target/release/smartcal" "${BIN_DIR}/smartcal"

echo "==> Installing desktop entry and icon..."
sudo install -Dm644 "${ROOT_DIR}/packaging/desktop/smartcal.desktop" "/usr/share/applications/smartcal.desktop"
sudo install -Dm644 "${ROOT_DIR}/packaging/icons/smartcal.svg" "/usr/share/icons/hicolor/scalable/apps/smartcal.svg"

echo "==> Installing non-root udev permissions rule..."
sudo install -Dm644 "${ROOT_DIR}/packaging/udev/99-smartcal-permissions.rules" "/etc/udev/rules.d/99-smartcal-permissions.rules"
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=input

TARGET_USER="${SUDO_USER:-$USER}"
if [[ -n "${TARGET_USER}" && "${TARGET_USER}" != "root" ]]; then
  echo "==> Adding user '${TARGET_USER}' to 'input' group for non-root evdev/uinput access..."
  sudo usermod -aG input "${TARGET_USER}" || true
fi

echo ""
echo "==> Installation complete!"
echo "    Run GUI: smartcal"
echo "    Run CLI: smartcal --help"
echo "    Note: Log out and log back in once if you were just added to the 'input' group."
