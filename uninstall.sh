#!/usr/bin/env bash
set -euo pipefail

echo "==> SmartCal Uninstaller"

sudo rm -f /usr/local/bin/smartcal /usr/bin/smartcal
sudo rm -f /usr/share/applications/smartcal.desktop
sudo rm -f /usr/share/icons/hicolor/scalable/apps/smartcal.svg
sudo rm -f /etc/udev/rules.d/99-smartcal-permissions.rules
sudo rm -f /etc/udev/rules.d/99-smartcal-calibration.rules

if command -v udevadm >/dev/null 2>&1; then
  sudo udevadm control --reload-rules || true
  sudo udevadm trigger --subsystem-match=input || true
fi

rm -f "${HOME}/.config/autostart/smartcal-autostart.desktop"
rm -f "${HOME}/.config/systemd/user/smartcal-autostart.service"

echo "==> SmartCal binaries and system rules removed."
echo "    Saved user calibration profiles in ${HOME}/.config/smartcal/ were preserved."
echo "    Delete ${HOME}/.config/smartcal/ manually if you wish to remove profiles too."
