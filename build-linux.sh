#!/bin/bash

set -e

echo "=== Building Workspace Manager for Linux (.deb) ==="

cargo build --release

echo "Creating .deb package..."

APP_NAME="workspace-manager"
VERSION="0.1.0"
BUILD_DIR="build/linux"
DEB_DIR="$BUILD_DIR/$APP_NAME-$VERSION"

rm -rf "$DEB_DIR"
mkdir -p "$DEB_DIR/DEBIAN"
mkdir -p "$DEB_DIR/usr/bin"
mkdir -p "$DEB_DIR/usr/share/applications"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"

cp target/release/$APP_NAME "$DEB_DIR/usr/bin/"
cp logo.png "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/$APP_NAME.png"

cat > "$DEB_DIR/DEBIAN/control" << EOF
Package: $APP_NAME
Version: $VERSION
Section: utility
Priority: optional
Architecture: amd64
Depends: libgtk-3-0, libxcb-render0, libxcb-shape0, libxcb-xfixes0
Maintainer: Developer
Description: Cross-platform workspace file manager
 A desktop application to manage .code-workspace files with GUI.
EOF

cat > "$DEB_DIR/usr/share/applications/$APP_NAME.desktop" << EOF
[Desktop Entry]
Name=Workspace Manager
Comment=Manage your workspace files
Exec=/usr/bin/$APP_NAME
Icon=$APP_NAME
Terminal=false
Type=Application
Categories=Utility;Development;
EOF

dpkg-deb --build "$DEB_DIR" "$BUILD_DIR/$APP_NAME-$VERSION-amd64.deb"

echo "✓ Built: $BUILD_DIR/$APP_NAME-$VERSION-amd64.deb"
