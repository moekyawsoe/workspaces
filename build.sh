#!/bin/bash

set -e

echo "╔══════════════════════════════════════════╗"
echo "║   Workspace Manager - Build Script       ║"
echo "╚══════════════════════════════════════════╝"
echo ""

OS=$(uname -s)

case "$OS" in
    Linux*)
        bash build-linux.sh
        ;;
    Darwin*)
        bash build-macos.sh
        ;;
    MINGW*|MSYS*|CYGWIN*)
        build-windows.bat
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo ""
echo "Build complete! Check the build/ directory for packages."
