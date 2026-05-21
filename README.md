# Workspace Manager

A cross-platform desktop application to manage `.code-workspace` files with a clean GUI.

<p align="center">
  <img src="logo.png" alt="Workspace Manager Icon" width="128" height="128">
</p>

## Preview

<p align="center">
  <img src="screenshot.png" alt="Workspace Manager Screenshot" width="800">
</p>

## Features

- Create, edit, and delete workspace configurations
- Open workspaces with your preferred editor
- **Built-in Terminal Improvements**:
  - Full mouse-wheel scrolling and keyboard navigation (`PageUp`/`PageDown`) through the scrollback buffer
  - Right-click context menu for Copy/Paste (prevents selection crashes and frees up shortcuts)
  - `Ctrl+C` works as standard terminal interrupt (`SIGINT`)
- **Auto-Updates**: Built-in self-updater checks, downloads, and installs updates seamlessly directly from GitHub Releases (across Linux `.deb`, macOS `.dmg`, and Windows `.zip`)
- **Statically Linked Windows Build**: The Windows `.exe` statically links the VC++ Runtime (CRT), meaning it runs out-of-the-box without requiring Visual C++ Redistributable (`VCRUNTIME140.dll`)
- Quick terminal access from workspace folders
- Search and filter workspaces
- Cross-platform support (Linux, macOS, Windows)

## Downloads

| Platform | Format | Download |
|----------|--------|----------|
| Linux | .deb | [Download](https://github.com/moekyawsoe/workspaces/releases/latest) |
| macOS | .dmg | [Download](https://github.com/moekyawsoe/workspaces/releases/latest) |
| Windows | .exe | [Download](https://github.com/moekyawsoe/workspaces/releases/latest) |

## Installation

### Linux (.deb)

```bash
sudo dpkg -i workspace-manager-*.deb
sudo apt-get install -f  # Install dependencies if needed
```

### macOS (.dmg)

1. Download the `.dmg` file
2. Open it and drag **Workspace Manager** to your Applications folder

### Windows (.exe)

1. Download the `.zip` file
2. Extract and run `workspace-manager.exe`

## Building from Source

```bash
cargo build --release
```

Or use the platform-specific build scripts:

```bash
# Linux
bash build-linux.sh

# macOS
bash build-macos.sh

# Windows
build-windows.bat
```

## About

- **Developer:** Moe Kyaw Soe
- **Website:** [www.moekyawsoe.com](https://www.moekyawsoe.com)

## License

This project is open source.
