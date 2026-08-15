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
fi
if [ -f "$SCRIPT_DIR/../xgameruntime/twinapi.appcore.dll.so" ]; then
    cp "$SCRIPT_DIR/../xgameruntime/twinapi.appcore.dll.so" "$APP_DIR/usr/lib/"
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

# If invoked with CLI subcommands, route to xodus CLI
if [ "$1" = "cli" ] || [ "$1" = "run" ] || [ "$1" = "auth" ] || [ "$1" = "save" ] || [ "$1" = "license" ] || [ "$1" = "status" ]; then
    if [ "$1" = "cli" ]; then shift; fi
    exec "${HERE}/usr/bin/xodus" "$@"
fi

exec "${HERE}/usr/bin/xodus-gui" "$@"
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
