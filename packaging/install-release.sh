#!/usr/bin/env bash
set -euo pipefail

# Zero-compile installer for pre-built SmartCal Linux release archives.
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="${HERE}/smartcal"

if [[ ! -x "${BIN_PATH}" ]]; then
  echo "Error: prebuilt 'smartcal' binary not found next to install-release.sh"
  exit 1
fi

echo "==> Installing pre-built SmartCal binary..."
sudo install -Dm755 "${BIN_PATH}" "/usr/local/bin/smartcal"
sudo install -Dm644 "${HERE}/packaging/desktop/smartcal.desktop" "/usr/share/applications/smartcal.desktop"
sudo install -Dm644 "${HERE}/packaging/icons/smartcal.svg" "/usr/share/icons/hicolor/scalable/apps/smartcal.svg"
sudo install -Dm644 "${HERE}/packaging/udev/99-smartcal-permissions.rules" "/etc/udev/rules.d/99-smartcal-permissions.rules"

if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update -qq || true
  sudo apt-get install -y xinput x11-xserver-utils libinput-tools fonts-dejavu-core || true
fi

sudo udevadm control --reload-rules || true
sudo udevadm trigger --subsystem-match=input || true

TARGET_USER="${SUDO_USER:-$USER}"
if [[ -n "${TARGET_USER}" && "${TARGET_USER}" != "root" ]]; then
  sudo usermod -aG input "${TARGET_USER}" || true
fi

echo "==> Installed! Run 'smartcal' or launch 'Smart Board Calibration' from your menu."
echo "    Note: Log out and log back in once if you were just added to the 'input' group."
