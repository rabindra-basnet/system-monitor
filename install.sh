#!/usr/bin/env bash
set -e

echo "================================================================"
echo "          Stasis System Optimizer — Installation System Checks  "
echo "================================================================"

# 1. Check Rust Toolchain
if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ Error: Rust toolchain (cargo) is not installed."
    echo "Install Rust via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi
echo "✔ [1/4] Rust toolchain detected ($(cargo --version))"

# 2. Check GTK3 and VTE runtime libraries for Native Desktop GUI mode
check_gtk_dependencies() {
    echo "🔍 [2/4] Checking GTK3 & VTE libraries for Desktop GUI window..."
    
    HAS_GTK=0
    HAS_VTE=0
    
    if ldconfig -p 2>/dev/null | grep -q 'libgtk-3\.so'; then
        HAS_GTK=1
    fi
    if ldconfig -p 2>/dev/null | grep -q 'libvte-2\.91\.so'; then
        HAS_VTE=1
    fi
    
    # Fallback search paths
    if [ "$HAS_GTK" -eq 0 ]; then
        if ls /usr/lib*/libgtk-3.so* /usr/lib/*-linux-gnu*/libgtk-3.so* /lib/*-linux-gnu*/libgtk-3.so* 1>/dev/null 2>&1; then
            HAS_GTK=1
        fi
    fi
    if [ "$HAS_VTE" -eq 0 ]; then
        if ls /usr/lib*/libvte-2.91.so* /usr/lib/*-linux-gnu*/libvte-2.91.so* /lib/*-linux-gnu*/libvte-2.91.so* 1>/dev/null 2>&1; then
            HAS_VTE=1
        fi
    fi

    if [ "$HAS_GTK" -eq 1 ] && [ "$HAS_VTE" -eq 1 ]; then
        echo "✔ [2/4] GTK3 (libgtk-3.so.0) and VTE (libvte-2.91.so.0) found (Full Native GUI Supported)"
        return 0
    fi

    echo ""
    echo "⚠️  [2/4] Missing GTK3 / VTE GUI libraries required for native desktop single-window mode:"
    [ "$HAS_GTK" -eq 0 ] && echo "    • Missing: libgtk-3.so.0 (GTK 3 runtime)"
    [ "$HAS_VTE" -eq 0 ] && echo "    • Missing: libvte-2.91.so.0 (VTE terminal widget)"
    echo ""

    INSTALL_CMD=""
    if command -v apt-get >/dev/null 2>&1; then
        INSTALL_CMD="sudo apt-get install -y libgtk-3-0 libvte-2.91-0"
        echo "    💡 Recommended for Debian/Ubuntu: ${INSTALL_CMD}"
    elif command -v dnf >/dev/null 2>&1; then
        INSTALL_CMD="sudo dnf install -y gtk3 vte291"
        echo "    💡 Recommended for Fedora/RHEL:   ${INSTALL_CMD}"
    elif command -v pacman >/dev/null 2>&1; then
        INSTALL_CMD="sudo pacman -S --needed gtk3 vte3"
        echo "    💡 Recommended for Arch/Manjaro:  ${INSTALL_CMD}"
    elif command -v zypper >/dev/null 2>&1; then
        INSTALL_CMD="sudo zypper install -y libgtk-3-0 libvte-2_91-0"
        echo "    💡 Recommended for openSUSE:      ${INSTALL_CMD}"
    elif command -v apk >/dev/null 2>&1; then
        INSTALL_CMD="sudo apk add gtk+3.0 vte3"
        echo "    💡 Recommended for Alpine:        ${INSTALL_CMD}"
    fi

    if [ -n "$INSTALL_CMD" ]; then
        if [ "$(id -u)" -eq 0 ]; then
            echo "    🔧 Installing GTK3/VTE automatically as root..."
            ${INSTALL_CMD#sudo } || true
        elif [ -t 0 ]; then
            read -r -p "    Would you like to install GTK3 & VTE dependencies now with sudo? [Y/n] " prompt
            if [[ "$prompt" =~ ^[Yy]?$ ]]; then
                echo "    🔧 Installing GUI dependencies..."
                eval "$INSTALL_CMD" || {
                    echo "    ⚠️ Installation failed. Stasis will default to Terminal CLI mode."
                }
            fi
        else
            echo "    ℹ️  Running in non-interactive mode. Install GUI libraries above to enable Desktop GUI."
        fi
    fi
}

check_gtk_dependencies

# 3. Check clipboard integration
if command -v wl-copy >/dev/null 2>&1 || command -v xclip >/dev/null 2>&1; then
    echo "✔ [3/4] Clipboard utility detected (wl-copy/xclip for [y] copy)"
else
    echo "ℹ️  [3/4] Tip: Install 'wl-copy' (Wayland) or 'xclip' (X11) to enable instant [y] process clipboard copying."
fi

# 4. Build Release Binary
echo "🔨 [4/4] Building Stasis (100% Pure Rust Native Binary) in release mode..."
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
echo "================================================================"
echo "✔ Done! Stasis is installed as a 100% Pure Rust Native Application."
echo "• Launch Desktop GUI:    Click 'Stasis' in Dock or run 'stasis'"
echo "• Launch in Terminal:    Run 'stasis --cli' or 'stasis -i'"
echo "• Zero Python:           Pure Rust standalone binary with zero external script runtime!"
echo "================================================================"
