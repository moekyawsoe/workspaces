@echo off
echo === Building Workspace Manager for Windows (.exe) ===

call cargo build --release --target x86_64-pc-windows-msvc

if %errorlevel% neq 0 (
    echo Building for native target...
    call cargo build --release
)

echo.
echo Preparing build directory...

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
echo === Packaging Windows Installer (.msi) ===

set WIX_DIR=C:\Program Files (x86)\WiX Toolset v3.11\bin
if exist "%WIX_DIR%\candle.exe" (
    echo WiX Toolset found. Compiling installer...
    "%WIX_DIR%\candle.exe" -out "%BUILD_DIR%\workspace-manager.wixobj" wix\workspace-manager.wxs
    if %errorlevel% equ 0 (
        echo Linking installer...
        "%WIX_DIR%\light.exe" -sval -out "%BUILD_DIR%\workspace-manager-%VERSION%-windows.msi" "%BUILD_DIR%\workspace-manager.wixobj"
        if %errorlevel% equ 0 (
            echo ✓ MSI Installer created: %BUILD_DIR%\workspace-manager-%VERSION%-windows.msi
        ) else (
            echo × WiX Linker (light.exe) failed.
        )
    ) else (
        echo × WiX Compiler (candle.exe) failed.
    )
) else (
    echo WiX Toolset v3.11 was not found at default path: %WIX_DIR%
    echo Please install WiX Toolset v3.11 to package into .msi automatically.
)
