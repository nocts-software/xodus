#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building Xodus Release Binaries ==="
# Force recompile of xodus-gui to pick up any UI asset changes
rm -f target/release/xodus-gui target/release/deps/xodus_gui* target/release/deps/libxodus_gui* 2>/dev/null || true
cargo build --release

if [ -d "$SCRIPT_DIR/../xgameruntime" ]; then
    echo "=== Ensuring xgameruntime libraries are up-to-date ==="
    (cd "$SCRIPT_DIR/../xgameruntime" && ./build-xgameruntime.sh)
fi

APP_DIR="$SCRIPT_DIR/build/AppDir"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/lib"
mkdir -p "$APP_DIR/usr/share/applications"
mkdir -p "$APP_DIR/usr/share/icons/hicolor/scalable/apps"

echo "=== Staging AppDir ==="
cp target/release/xodus-gui "$APP_DIR/usr/bin/"
cp target/release/xodus-cli "$APP_DIR/usr/bin/xodus"
cp target/release/xodus-service "$APP_DIR/usr/bin/"

if [ -f "$SCRIPT_DIR/../xgameruntime/xgameruntime.dll.so" ]; then
    cp "$SCRIPT_DIR/../xgameruntime/xgameruntime.dll.so" "$APP_DIR/usr/lib/"
    cp "$SCRIPT_DIR/../xgameruntime/xgameruntime.dll" "$APP_DIR/usr/lib/" 2>/dev/null || true
fi
if [ -f "$SCRIPT_DIR/../xgameruntime/twinapi.appcore.dll.so" ]; then
    cp "$SCRIPT_DIR/../xgameruntime/twinapi.appcore.dll.so" "$APP_DIR/usr/lib/"
    cp "$SCRIPT_DIR/../xgameruntime/twinapi.appcore.dll" "$APP_DIR/usr/lib/" 2>/dev/null || true
fi
if [ -f "$SCRIPT_DIR/../xgameruntime/api-ms-win-core-psm-appnotify-l1-1-0.dll.so" ]; then
    cp "$SCRIPT_DIR/../xgameruntime/api-ms-win-core-psm-appnotify-l1-1-0.dll.so" "$APP_DIR/usr/lib/"
    cp "$SCRIPT_DIR/../xgameruntime/api-ms-win-core-psm-appnotify-l1-1-0.dll" "$APP_DIR/usr/lib/" 2>/dev/null || true
fi
if [ -f "$SCRIPT_DIR/../xgameruntime/windows.ui.core.textinput.dll.so" ]; then
    cp "$SCRIPT_DIR/../xgameruntime/windows.ui.core.textinput.dll.so" "$APP_DIR/usr/lib/"
    cp "$SCRIPT_DIR/../xgameruntime/windows.ui.core.textinput.dll" "$APP_DIR/usr/lib/" 2>/dev/null || true
fi
if [ -f "$SCRIPT_DIR/../xgameruntime/wintypes.dll.so" ]; then
    cp "$SCRIPT_DIR/../xgameruntime/wintypes.dll.so" "$APP_DIR/usr/lib/"
    cp "$SCRIPT_DIR/../xgameruntime/wintypes.dll" "$APP_DIR/usr/lib/" 2>/dev/null || true
fi
if [ -d "$SCRIPT_DIR/../xgameruntime/x86_64-unix" ]; then
    mkdir -p "$APP_DIR/usr/lib/x86_64-unix"
    cp -r "$SCRIPT_DIR/../xgameruntime/x86_64-unix/"* "$APP_DIR/usr/lib/x86_64-unix/" 2>/dev/null || true
fi

cp "$SCRIPT_DIR/crates/xodus-gui/xodus-gui.desktop" "$APP_DIR/"
cp "$SCRIPT_DIR/crates/xodus-gui/xodus-gui.desktop" "$APP_DIR/usr/share/applications/"
cp "$SCRIPT_DIR/crates/xodus-gui/xodus-gui.svg" "$APP_DIR/"
cp "$SCRIPT_DIR/crates/xodus-gui/xodus-gui.svg" "$APP_DIR/usr/share/icons/hicolor/scalable/apps/"

cat << 'EOF' > "$APP_DIR/AppRun"
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
export XODUS_RUNTIME_PATH="${HERE}/usr/lib"
export WINEDLLPATH="${HERE}/usr/lib:${WINEDLLPATH}"

# Ensure WebKitGTK and GDK run reliably across all Wayland compositors (Bazzite, Fedora, SteamOS, GNOME, KDE)
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1

# If invoked without arguments, or explicitly with gui / --gui / -g / ui, launch graphical interface
if [ -z "$1" ] || [ "$1" = "gui" ] || [ "$1" = "--gui" ] || [ "$1" = "-g" ] || [ "$1" = "ui" ]; then
    if [ "$1" = "gui" ] || [ "$1" = "--gui" ] || [ "$1" = "-g" ] || [ "$1" = "ui" ]; then shift; fi
    exec "${HERE}/usr/bin/xodus-gui" "$@"
fi

# Otherwise route all CLI arguments and subcommands (login, download, play, run, status, etc.) to xodus CLI
if [ "$1" = "cli" ]; then shift; fi
exec "${HERE}/usr/bin/xodus" "$@"
EOF

chmod +x "$APP_DIR/AppRun"

echo "=== Packaging Self-Contained AppImage ==="
OUTPUT_APPIMAGE="$SCRIPT_DIR/Xodus-x86_64.AppImage"
ARCH=x86_64 appimagetool "$APP_DIR" "$OUTPUT_APPIMAGE"

mkdir -p "$HOME/Builds"
cp --remove-destination "$OUTPUT_APPIMAGE" "$HOME/Builds/nocts-xodus-gui.AppImage"
chmod +x "$HOME/Builds/nocts-xodus-gui.AppImage"

echo "=== Successfully Generated: $OUTPUT_APPIMAGE ==="
echo "=== Deployed copy to: $HOME/Builds/nocts-xodus-gui.AppImage ==="
