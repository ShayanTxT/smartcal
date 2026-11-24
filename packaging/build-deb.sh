#!/usr/bin/env bash
set -euo pipefail

# Builds a Debian/Ubuntu .deb package for SmartCal using dpkg-deb.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${VERSION:-1.0.0}"
ARCH="${ARCH:-amd64}"
PKG_NAME="smartcal_${VERSION}_${ARCH}"
STAGE="${ROOT_DIR}/target/deb/${PKG_NAME}"

cd "${ROOT_DIR}"
cargo build --release --bin smartcal

rm -rf "${STAGE}"
mkdir -p "${STAGE}/DEBIAN"
mkdir -p "${STAGE}/usr/bin"
mkdir -p "${STAGE}/usr/share/applications"
mkdir -p "${STAGE}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${STAGE}/lib/udev/rules.d"
mkdir -p "${STAGE}/usr/lib/systemd/user"

install -m755 "${ROOT_DIR}/target/release/smartcal" "${STAGE}/usr/bin/smartcal"
install -m644 "${ROOT_DIR}/packaging/desktop/smartcal.desktop" "${STAGE}/usr/share/applications/smartcal.desktop"
install -m644 "${ROOT_DIR}/packaging/icons/smartcal.svg" "${STAGE}/usr/share/icons/hicolor/scalable/apps/smartcal.svg"
install -m644 "${ROOT_DIR}/packaging/udev/99-smartcal-permissions.rules" "${STAGE}/lib/udev/rules.d/99-smartcal-permissions.rules"
install -m644 "${ROOT_DIR}/packaging/systemd/smartcal-autostart.service" "${STAGE}/usr/lib/systemd/user/smartcal-autostart.service"

cat > "${STAGE}/DEBIAN/control" <<EOF
Package: smartcal
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: SmartCal Maintainers <maintainers@smartcal.local>
Depends: libc6, libgl1 | libgl1-mesa-glx, libegl1, libx11-6, libxkbcommon0, libxkbcommon-x11-0, udev
Recommends: xinput, x11-xserver-utils, libinput-tools, fonts-dejavu-core, fonts-vazirmatn
Description: Smart Board & Interactive Projector Calibration Suite
 Professional Linux desktop utility for calibrating touchscreens, interactive
 whiteboards, and ceiling/wall projectors across X11 and Wayland using
 projective homography, RANSAC, polynomial/TPS models, and noise filtering.
EOF

cat > "${STAGE}/DEBIAN/postinst" <<'EOF'
#!/usr/bin/env bash
set -e
if command -v udevadm >/dev/null 2>&1; then
  udevadm control --reload-rules || true
  udevadm trigger --subsystem-match=input || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications || true
fi
EOF
chmod 755 "${STAGE}/DEBIAN/postinst"

mkdir -p "${ROOT_DIR}/dist"
dpkg-deb --build --root-owner-group "${STAGE}" "${ROOT_DIR}/dist/${PKG_NAME}.deb"
echo "==> Built Debian package: ${ROOT_DIR}/dist/${PKG_NAME}.deb"
