use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceLaunch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub folders: Vec<WorkspaceFolder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch: Option<WorkspaceLaunch>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub content: String,
    pub config: WorkspaceConfig,
}

#[derive(Debug)]
pub enum WorkspaceError {
    IoError(std::io::Error),
    ParseError(String),
    NotFound(String),
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceError::IoError(e) => write!(f, "IO error: {}", e),
            WorkspaceError::ParseError(e) => write!(f, "Parse error: {}", e),
            WorkspaceError::NotFound(e) => write!(f, "Not found: {}", e),
        }
    }
}

impl From<std::io::Error> for WorkspaceError {
    fn from(error: std::io::Error) -> Self {
        WorkspaceError::IoError(error)
    }
}

pub fn parse_workspace_json(content: &str) -> Result<WorkspaceConfig, WorkspaceError> {
    serde_json::from_str(content).map_err(|e| WorkspaceError::ParseError(e.to_string()))
}

pub fn workspace_config_to_json(config: &WorkspaceConfig) -> Result<String, WorkspaceError> {
    serde_json::to_string_pretty(config).map_err(|e| WorkspaceError::ParseError(e.to_string()))
}

pub fn list_workspaces(folder: &Path) -> Result<Vec<WorkspaceFile>, WorkspaceError> {
    if !folder.exists() {
        return Err(WorkspaceError::NotFound(format!(
            "Folder not found: {}",
            folder.display()
        )));
    }

    let mut workspaces = Vec::new();

    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(ext) = path.extension() {
            if ext == "code-workspace" || ext == "workspace" {
                let content = fs::read_to_string(&path)?;
                let metadata = entry.metadata()?;
                let config = parse_workspace_json(&content).unwrap_or_default();

                workspaces.push(WorkspaceFile {
                    name: path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    path: path.clone(),
                    size: metadata.len(),
                    modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    content,
                    config,
                });
            }
        }
    }

    workspaces.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(workspaces)
}

pub fn create_workspace(
    folder: &Path,
    name: &str,
    config: &WorkspaceConfig,
) -> Result<PathBuf, WorkspaceError> {
    let mut path = folder.to_path_buf();
    path.push(format!("{}.code-workspace", name));

    if path.exists() {
        return Err(WorkspaceError::ParseError(format!(
            "Workspace '{}' already exists",
            name
        )));
    }

    let content = workspace_config_to_json(config)?;
    fs::write(&path, content)?;
    Ok(path)
}

pub fn update_workspace(path: &Path, config: &WorkspaceConfig) -> Result<(), WorkspaceError> {
    if !path.exists() {
        return Err(WorkspaceError::NotFound(format!(
            "File not found: {}",
            path.display()
        )));
    }

    let content = workspace_config_to_json(config)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn delete_workspace(path: &Path) -> Result<(), WorkspaceError> {
    if !path.exists() {
        return Err(WorkspaceError::NotFound(format!(
            "File not found: {}",
            path.display()
        )));
    }

    fs::remove_file(path)?;
    Ok(())
}

pub fn duplicate_workspace(source: &Path) -> Result<PathBuf, WorkspaceError> {
    if !source.exists() {
        return Err(WorkspaceError::NotFound(format!(
            "File not found: {}",
            source.display()
        )));
    }

    let stem = source
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let parent = source.parent().unwrap_or(Path::new("."));

    let mut new_path = parent.to_path_buf();
    new_path.push(format!("{} copy.code-workspace", stem));

    let mut counter = 1;
    while new_path.exists() {
        new_path = parent.to_path_buf();
        new_path.push(format!("{} copy {}.code-workspace", stem, counter));
        counter += 1;
    }

    fs::copy(source, &new_path)?;
    Ok(new_path)
}

pub fn open_in_file_manager(path: &Path) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
}

pub fn open_with_editor(path: &Path, editor: &str) {
    let editor_lower = editor.to_lowercase();

    match editor_lower.as_str() {
        "vscode" | "code" => {
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("code").arg(path).spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("code")
                    .arg(path)
                    .spawn()
                    .or_else(|_| {
                        std::process::Command::new("open")
                            .arg("-a")
                            .arg("Visual Studio Code")
                            .arg(path)
                            .spawn()
                    });
            }
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("code").arg(path).spawn();
            }
        }
        "cursor" => {
            let _ = std::process::Command::new("cursor").arg(path).spawn();
        }
        "vscodium" | "codium" => {
            let _ = std::process::Command::new("codium").arg(path).spawn();
        }
        "sublime" | "subl" => {
            let _ = std::process::Command::new("subl").arg(path).spawn();
        }
        "vim" | "nvim" | "neovim" => {
            let _ = open_terminal_with_editor(path, editor);
        }
        _ => {
            let _ = std::process::Command::new(editor).arg(path).spawn();
        }
    }
}

#[cfg(unix)]
fn open_terminal_with_editor(path: &Path, editor: &str) -> std::io::Result<()> {
    use std::process::Command;

    let editor_cmd = if editor == "nvim" || editor == "neovim" {
        "nvim"
    } else {
        "vim"
    };

    if Command::new("which").arg("gnome-terminal").output().is_ok() {
        Command::new("gnome-terminal")
            .arg("--")
            .arg(editor_cmd)
            .arg(path)
            .spawn()?;
    } else if Command::new("which").arg("konsole").output().is_ok() {
        Command::new("konsole")
            .arg("-e")
            .arg(editor_cmd)
            .arg(path)
            .spawn()?;
    } else {
        Command::new("x-terminal-emulator")
            .arg("-e")
            .arg(editor_cmd)
            .arg(path)
            .spawn()?;
    }
    Ok(())
}

#[cfg(windows)]
fn open_terminal_with_editor(path: &Path, editor: &str) -> std::io::Result<()> {
    std::process::Command::new("cmd")
        .arg("/c")
        .arg("start")
        .arg(editor)
        .arg(path)
        .spawn()
}

pub fn get_default_editors() -> Vec<&'static str> {
    vec![
        "VS Code",
        "Cursor",
        "VSCodium",
        "Sublime Text",
        "Neovim",
        "Vim",
    ]
}

/// Detects available system terminal emulators and returns the best one.
/// Returns (command, args_to_pass_to_command)
pub fn detect_system_terminal() -> Option<(String, Vec<String>)> {
    #[cfg(target_os = "linux")]
    {
        let terminals: [(&str, &[&str]); 9] = [
            ("gnome-terminal", &["--"]),
            ("konsole", &["-e"]),
            ("xfce4-terminal", &["-e"]),
            ("alacritty", &["-e"]),
            ("kitty", &[]),
            ("wezterm", &["start", "--"]),
            ("foot", &[]),
            ("xterm", &["-e"]),
            ("st", &["-e"]),
        ];

        for (cmd, args) in terminals {
            if command_exists(cmd) {
                return Some((
                    cmd.to_string(),
                    args.iter().map(|s| s.to_string()).collect(),
                ));
            }
        }

        // Fallback to x-terminal-emulator (Debian/Ubuntu alternative system)
        if command_exists("x-terminal-emulator") {
            return Some(("x-terminal-emulator".to_string(), vec!["-e".to_string()]));
        }

        None
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: use osascript to open Terminal.app
        Some((
            "osascript".to_string(),
            vec![
                "-e".to_string(),
                r#"tell application "Terminal" to do script ""#.to_string(),
            ],
        ))
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: try Windows Terminal, then fall back to cmd
        if command_exists("wt") {
            Some(("wt".to_string(), vec![]))
        } else {
            Some((
                "cmd".to_string(),
                vec!["/c".to_string(), "start".to_string()],
            ))
        }
    }
}

/// Opens the system terminal emulator in the given working directory.
/// If `terminal` is provided, uses that terminal; otherwise auto-detects.
pub fn open_system_terminal(
    working_dir: &std::path::Path,
    terminal: Option<&str>,
) -> Result<(), String> {
    let terminal = match terminal {
        Some(t) => t.to_string(),
        None => detect_system_terminal()
            .ok_or_else(|| "No system terminal emulator found. Please install one (gnome-terminal, konsole, alacritty, etc.)".to_string())?
            .0,
    };

    #[cfg(target_os = "linux")]
    {
        let mut cmd = std::process::Command::new(&terminal);

        match terminal.as_str() {
            "gnome-terminal" => {
                cmd.arg(format!("--working-directory={}", working_dir.display()));
                cmd.arg("--").arg("bash");
            }
            "konsole" => {
                cmd.arg("--workdir").arg(working_dir);
            }
            "xfce4-terminal" => {
                cmd.arg(format!("--working-directory={}", working_dir.display()));
            }
            "alacritty" => {
                cmd.arg("--working-directory").arg(working_dir);
            }
            "kitty" => {
                cmd.arg("--directory").arg(working_dir);
            }
            "foot" => {
                cmd.arg(format!("--working-directory={}", working_dir.display()));
            }
            "wezterm" => {
                cmd.arg("start").arg("--cwd").arg(working_dir);
            }
            "xterm" | "st" => {
                cmd.arg("-e")
                    .arg(format!("cd {} && exec $SHELL", working_dir.display()));
            }
            _ => {
                cmd.current_dir(working_dir);
            }
        }

        cmd.spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        let dir = working_dir.to_string_lossy();
        let script = format!(r#"tell application "Terminal" to do script "cd '{}'"#, dir);
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn()
            .map_err(|e| format!("Failed to open Terminal.app: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        let dir = working_dir.to_string_lossy();
        match terminal.as_str() {
            "wt" => {
                std::process::Command::new("wt")
                    .arg("-d")
                    .arg(&dir)
                    .spawn()
                    .map_err(|e| format!("Failed to open Windows Terminal: {}", e))?;
            }
            "cmd" => {
                std::process::Command::new("cmd")
                    .arg("/c")
                    .arg("start")
                    .arg("cmd")
                    .arg("/k")
                    .arg("cd")
                    .arg(&dir)
                    .spawn()
                    .map_err(|e| format!("Failed to open cmd: {}", e))?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Returns a list of detected terminal emulators with their names.
pub fn list_available_terminals() -> Vec<(String, String)> {
    #[cfg(target_os = "linux")]
    {
        let terminals = [
            ("gnome-terminal", "GNOME Terminal"),
            ("konsole", "KDE Konsole"),
            ("xfce4-terminal", "XFCE Terminal"),
            ("alacritty", "Alacritty"),
            ("kitty", "Kitty"),
            ("wezterm", "WezTerm"),
            ("foot", "Foot"),
            ("xterm", "Xterm"),
            ("st", "Simple Terminal (st)"),
        ];

        terminals
            .iter()
            .filter(|(cmd, _)| command_exists(cmd))
            .map(|(cmd, name)| (cmd.to_string(), name.to_string()))
            .collect()
    }

    #[cfg(target_os = "macos")]
    {
        vec![("Terminal.app".to_string(), "Terminal".to_string())]
    }

    #[cfg(target_os = "windows")]
    {
        let mut result = Vec::new();
        if command_exists("wt") {
            result.push(("wt".to_string(), "Windows Terminal".to_string()));
        }
        result.push(("cmd".to_string(), "Command Prompt".to_string()));
        result
    }
}

#[cfg(unix)]
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("where")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    }
}

pub fn format_time(time: &SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = (*time).into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}
