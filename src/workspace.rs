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
