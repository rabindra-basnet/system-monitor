#!/usr/bin/env bash
set -e

VERSION="${1:-0.1.0}"
ARCH="amd64"
PKG_NAME="stasis_${VERSION}_${ARCH}"

echo "🔨 Building Stasis binary in release mode..."
cargo build --release

echo "📦 Packaging Debian (.deb) package: ${PKG_NAME}.deb..."
rm -rf "${PKG_NAME}" "${PKG_NAME}.deb"

mkdir -p "${PKG_NAME}/DEBIAN"
mkdir -p "${PKG_NAME}/usr/bin"
mkdir -p "${PKG_NAME}/usr/share/applications"
mkdir -p "${PKG_NAME}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${PKG_NAME}/usr/share/pixmaps"

# Copy binary & assets
cp target/release/stasis "${PKG_NAME}/usr/bin/stasis"
chmod 755 "${PKG_NAME}/usr/bin/stasis"

cp assets/stasis.svg "${PKG_NAME}/usr/share/icons/hicolor/scalable/apps/stasis.svg"
cp assets/stasis.svg "${PKG_NAME}/usr/share/pixmaps/stasis.svg"
cp stasis.desktop "${PKG_NAME}/usr/share/applications/stasis.desktop"

# Create Debian control file
cat << CONTROL_EOF > "${PKG_NAME}/DEBIAN/control"
Package: stasis
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Depends: libc6 (>= 2.34), libgtk-3-0 (>= 3.24.0), libvte-2.91-0 (>= 0.60.0)
Recommends: wl-clipboard | xclip
Maintainer: Rabindra Basnet <rabindra@users.noreply.github.com>
Description: High-performance Linux system monitor and process manager
 Stasis is a 100% Pure Rust native Linux system optimizer and process manager.
 Features real-time CPU & GPU telemetry, multi-core heatmaps, Ingress/Egress
 network traffic monitoring, listening socket inspector, systemd service
 controller, autostart manager, system cache cleaner, and application uninstaller.
CONTROL_EOF

# Create Debian postinst script
cat << POSTINST_EOF > "${PKG_NAME}/DEBIAN/postinst"
#!/usr/bin/env bash
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi
exit 0
POSTINST_EOF
chmod 755 "${PKG_NAME}/DEBIAN/postinst"

# Build .deb package using dpkg-deb
dpkg-deb --build --root-owner-group "${PKG_NAME}"
rm -rf "${PKG_NAME}"

echo ""
echo "✔ Successfully created Debian package: ${PKG_NAME}.deb ($(du -h "${PKG_NAME}.deb" | cut -f1))"
echo "To install via APT, run:"
echo "    sudo apt install ./${PKG_NAME}.deb"
