#!/bin/bash

set -e

echo "=== Building Workspace Manager for macOS (.dmg) ==="

TARGET="aarch64-apple-darwin"

if rustup target list | grep -q "$TARGET (installed)"; then
    echo "Building for $TARGET..."
    cargo build --release --target $TARGET
    TARGET_DIR="target/$TARGET/release"
else
    echo "Building for native target..."
    cargo build --release
    TARGET_DIR="target/release"
fi

APP_NAME="workspace-manager"
VERSION="0.1.0"
BUILD_DIR="build/macos"
APP_BUNDLE="$BUILD_DIR/Workspace Manager.app"

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

cp "$TARGET_DIR/$APP_NAME" "$APP_BUNDLE/Contents/MacOS/"

cat > "$APP_BUNDLE/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Workspace Manager</string>
    <key>CFBundleDisplayName</key>
    <string>Workspace Manager</string>
    <key>CFBundleIdentifier</key>
    <string>com.workspacemanager.app</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "Creating DMG..."

DMG_FILE="$BUILD_DIR/Workspace-Manager-$VERSION.dmg"
rm -f "$DMG_FILE"

hdiutil create -volname "Workspace Manager" \
    -srcfolder "$APP_BUNDLE" \
    -ov -format UDZO \
    "$DMG_FILE"

echo "✓ Built: $DMG_FILE"
