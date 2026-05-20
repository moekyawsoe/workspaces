@echo off
echo === Building Workspace Manager for Windows (.exe) ===

call cargo build --release --target x86_64-pc-windows-msvc

if %errorlevel% neq 0 (
    echo Building for native target...
    call cargo build --release
)

echo.
echo Creating installer...

set APP_NAME=workspace-manager
set VERSION=0.1.0
set BUILD_DIR=build\windows

if not exist %BUILD_DIR% mkdir %BUILD_DIR%

if exist target\release\%APP_NAME%.exe (
    copy target\release\%APP_NAME%.exe %BUILD_DIR%\%APP_NAME%.exe
    echo ✓ Built: %BUILD_DIR%\%APP_NAME%.exe
) else if exist target\x86_64-pc-windows-msvc\release\%APP_NAME%.exe (
    copy target\x86_64-pc-windows-msvc\release\%APP_NAME%.exe %BUILD_DIR%\%APP_NAME%.exe
    echo ✓ Built: %BUILD_DIR%\%APP_NAME%.exe
) else (
    echo × Build failed - executable not found
    exit /b 1
)

echo.
echo To create a proper installer, use one of:
echo   - Inno Setup
echo   - NSIS
echo   - WiX Toolset
echo.
echo Or use cargo-wix: cargo install cargo-wix ^&^& cargo wix
