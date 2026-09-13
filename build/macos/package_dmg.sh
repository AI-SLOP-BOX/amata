#!/bin/bash
set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_NAME="IrasuIllustrator"
DMG_NAME="IrasuIllustrator_macOS.dmg"
BUNDLE_DIR="$PROJECT_DIR/target/bundle/osx/$APP_NAME.app"

echo "=== 1. Building Release Binary ==="
cargo build --release --bin irasu-illustrator --manifest-path "$PROJECT_DIR/Cargo.toml"

echo "=== 2. Creating macOS .app Bundle ==="
rm -rf "$PROJECT_DIR/target/bundle/osx"
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

cp "$PROJECT_DIR/target/release/irasu-illustrator" "$BUNDLE_DIR/Contents/MacOS/$APP_NAME"

cat << 'PLIST' > "$BUNDLE_DIR/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>IrasuIllustrator</string>
    <key>CFBundleIdentifier</key>
    <string>com.irasu.illustrator</string>
    <key>CFBundleName</key>
    <string>Irasu Illustrator</string>
    <key>CFBundleDisplayName</key>
    <string>Irasu Illustrator</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

echo "=== 3. Packaging .dmg Installer ==="
DMG_TEMP_DIR="$PROJECT_DIR/target/dmg_temp"
rm -rf "$DMG_TEMP_DIR" "$PROJECT_DIR/$DMG_NAME"
mkdir -p "$DMG_TEMP_DIR"
cp -R "$BUNDLE_DIR" "$DMG_TEMP_DIR/"
ln -s /Applications "$DMG_TEMP_DIR/Applications"

hdiutil create -volname "$APP_NAME" -srcfolder "$DMG_TEMP_DIR" -ov -format UDZO "$PROJECT_DIR/$DMG_NAME"
rm -rf "$DMG_TEMP_DIR"

echo "=== SUCCESS: $PROJECT_DIR/$DMG_NAME created ==="
