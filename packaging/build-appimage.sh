#!/usr/bin/env bash
set -euo pipefail

# Builds an AppDir and AppImage for SmartCal.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${VERSION:-1.0.0}"
APPDIR="${ROOT_DIR}/target/AppDir"

cd "${ROOT_DIR}"
cargo build --release --bin smartcal

rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/scalable/apps"

install -m755 "${ROOT_DIR}/target/release/smartcal" "${APPDIR}/usr/bin/smartcal"
install -m644 "${ROOT_DIR}/packaging/desktop/smartcal.desktop" "${APPDIR}/smartcal.desktop"
install -m644 "${ROOT_DIR}/packaging/desktop/smartcal.desktop" "${APPDIR}/usr/share/applications/smartcal.desktop"
install -m644 "${ROOT_DIR}/packaging/icons/smartcal.svg" "${APPDIR}/smartcal.svg"
install -m644 "${ROOT_DIR}/packaging/icons/smartcal.svg" "${APPDIR}/usr/share/icons/hicolor/scalable/apps/smartcal.svg"

cat > "${APPDIR}/AppRun" <<'EOF'
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
exec "${HERE}/usr/bin/smartcal" "$@"
EOF
chmod +x "${APPDIR}/AppRun"

mkdir -p "${ROOT_DIR}/dist"
OUT_APPIMAGE="${ROOT_DIR}/dist/SmartCal-${VERSION}-x86_64.AppImage"
if [[ -n "${APPIMAGETOOL_BIN:-}" && -x "${APPIMAGETOOL_BIN}" ]]; then
  ARCH=x86_64 "${APPIMAGETOOL_BIN}" "${APPDIR}" "${OUT_APPIMAGE}"
  echo "==> Built AppImage: ${OUT_APPIMAGE}"
elif command -v appimagetool >/dev/null 2>&1; then
  ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 appimagetool "${APPDIR}" "${OUT_APPIMAGE}"
  echo "==> Built AppImage: ${OUT_APPIMAGE}"
else
  echo "appimagetool not found in PATH; AppDir staged cleanly at ${APPDIR}."
fi
