#!/usr/bin/env bash
set -e

echo "=== Building Stasis (100% Pure Rust Native Binary) in release mode ==="
cargo build --release

# Clean up legacy scripts
rm -f "${HOME}/.local/bin/sysmon-tui" "${HOME}/.local/share/applications/sysmon-tui.desktop" "${HOME}/.local/bin/stasis-desktop" "${HOME}/.local/lib/stasis/stasis-window" 2>/dev/null || true
rm -rf "${HOME}/.local/lib/stasis" 2>/dev/null || true

# User Paths
USER_BIN="${HOME}/.local/bin"
USER_APP="${HOME}/.local/share/applications"
USER_ICON="${HOME}/.local/share/icons/hicolor/scalable/apps"
USER_PIXMAP="${HOME}/.local/share/pixmaps"

mkdir -p "${USER_BIN}" "${USER_APP}" "${USER_ICON}" "${USER_PIXMAP}"

echo "Installing Stasis standalone binary to ${USER_BIN}/stasis..."
rm -f "${USER_BIN}/stasis" 2>/dev/null || true
install -m 755 target/release/stasis "${USER_BIN}/stasis"

echo "Installing desktop icon..."
install -m 644 assets/stasis.svg "${USER_ICON}/stasis.svg"
install -m 644 assets/stasis.svg "${USER_PIXMAP}/stasis.svg"

echo "Installing desktop launcher to ${USER_APP}/stasis.desktop..."
install -m 644 stasis.desktop "${USER_APP}/stasis.desktop"

# If running as root or if system dirs writable, also install system-wide
if [ "$(id -u)" -eq 0 ] || [ -w "/usr/local/bin" ]; then
    echo "Installing system-wide..."
    mkdir -p "/usr/share/applications" "/usr/share/icons/hicolor/scalable/apps" "/usr/share/pixmaps" 2>/dev/null || true
    install -m 755 target/release/stasis "/usr/local/bin/stasis" 2>/dev/null || true
    install -m 644 assets/stasis.svg "/usr/share/icons/hicolor/scalable/apps/stasis.svg" 2>/dev/null || true
    install -m 644 assets/stasis.svg "/usr/share/pixmaps/stasis.svg" 2>/dev/null || true
    install -m 644 stasis.desktop "/usr/share/applications/stasis.desktop" 2>/dev/null || true
fi

# Update desktop database
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${USER_APP}" 2>/dev/null || true
fi

# Update icon cache
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${USER_ICON}/../../.." 2>/dev/null || true
fi

echo ""
echo "✔ Done! Stasis is installed as a 100% Pure Rust Native Desktop Application."
echo "• Launch Desktop GUI:    Click 'Stasis' in Dock or run 'stasis'"
echo "• Launch in Terminal:    Run 'stasis --cli' or 'stasis -i'"
echo "• Zero Python:           Pure Rust standalone binary with zero external script runtime!"
