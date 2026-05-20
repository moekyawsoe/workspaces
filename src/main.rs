mod workspace;

use eframe::egui;
use std::collections::HashMap;
use std::time::SystemTime;
use workspace::*;

#[derive(Clone)]
struct FormFolder {
    path: String,
    name: String,
}

#[derive(Clone)]
struct FormSettings {
    entries: Vec<(String, String)>,
}

#[derive(Clone)]
struct WorkspaceForm {
    folders: Vec<FormFolder>,
    settings: FormSettings,
    extensions: Vec<String>,
}

impl Default for WorkspaceForm {
    fn default() -> Self {
        Self {
            folders: vec![FormFolder {
                path: String::new(),
                name: String::new(),
            }],
            settings: FormSettings {
                entries: Vec::new(),
            },
            extensions: Vec::new(),
        }
    }
}

impl WorkspaceForm {
    fn from_config(config: &WorkspaceConfig) -> Self {
        let folders = if config.folders.is_empty() {
            vec![FormFolder {
                path: String::new(),
                name: String::new(),
            }]
        } else {
            config
                .folders
                .iter()
                .map(|f| FormFolder {
                    path: f.path.clone(),
                    name: f.name.clone().unwrap_or_default(),
                })
                .collect()
        };

        let settings = if let Some(map) = &config.settings {
            FormSettings {
                entries: map
                    .iter()
                    .map(|(k, v)| {
                        let val = match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Number(n) => n.to_string(),
                            _ => serde_json::to_string(v).unwrap_or_default(),
                        };
                        (k.clone(), val)
                    })
                    .collect(),
            }
        } else {
            FormSettings {
                entries: Vec::new(),
            }
        };

        let extensions = config.extensions.clone().unwrap_or_default();

        Self {
            folders,
            settings,
            extensions,
        }
    }

    fn to_config(&self) -> WorkspaceConfig {
        let folders: Vec<WorkspaceFolder> = self
            .folders
            .iter()
            .filter(|f| !f.path.is_empty())
            .map(|f| WorkspaceFolder {
                path: f.path.clone(),
                name: if f.name.is_empty() {
                    None
                } else {
                    Some(f.name.clone())
                },
            })
            .collect();

        let settings = if self.settings.entries.is_empty() {
            None
        } else {
            let mut map = HashMap::new();
            for (key, value) in &self.settings.entries {
                if !key.is_empty() {
                    let json_value: serde_json::Value = serde_json::from_str(value)
                        .unwrap_or(serde_json::Value::String(value.clone()));
                    map.insert(key.clone(), json_value);
                }
            }
            Some(map)
        };

        let extensions = if self.extensions.is_empty() {
            None
        } else {
            Some(self.extensions.clone())
        };

        WorkspaceConfig {
            folders,
            settings,
            extensions,
            launch: None,
        }
    }
}

enum DialogState {
    None,
    Create,
    Edit(usize),
    Delete(usize),
    EditorSelect(usize),
    TerminalSettings,
    About,
}

#[derive(Clone)]
enum Message {
    None,
    Info(String),
    Error(String),
    Success(String),
}

#[derive(Clone)]
enum PendingAction {
    None,
    OpenDefault(usize),
    OpenFileManager(usize),
    OpenEditor(usize),
    OpenTerminal(usize),
    Edit(usize),
    Duplicate(usize),
    Delete(usize),
}

struct WorkspaceManagerApp {
    folder_path: String,
    workspaces: Vec<WorkspaceFile>,
    dialog: DialogState,
    message: Message,
    pending_action: PendingAction,

    new_name: String,
    form: WorkspaceForm,
    selected_editor: String,
    search_query: String,
    custom_editor: String,
    show_custom_editor: bool,

    new_extension: String,
    new_setting_key: String,
    new_setting_value: String,

    selected_terminal: String,
    terminal_message: Option<String>,
}

impl Default for WorkspaceManagerApp {
    fn default() -> Self {
        let folder_path = dirs::home_dir()
            .unwrap_or_default()
            .join("workspaces")
            .to_string_lossy()
            .to_string();

        let available = list_available_terminals();
        let selected_terminal = available
            .first()
            .map(|(cmd, _)| cmd.clone())
            .unwrap_or_default();

        let (saved_terminal, _known_terminals) = Self::load_settings();
        let selected_terminal = if saved_terminal.is_empty() {
            selected_terminal
        } else {
            saved_terminal
        };

        Self {
            folder_path,
            workspaces: Vec::new(),
            dialog: DialogState::None,
            message: Message::None,
            pending_action: PendingAction::None,
            new_name: String::new(),
            form: WorkspaceForm::default(),
            selected_editor: "VS Code".to_string(),
            search_query: String::new(),
            custom_editor: String::new(),
            show_custom_editor: false,
            new_extension: String::new(),
            new_setting_key: String::new(),
            new_setting_value: String::new(),
            selected_terminal,
            terminal_message: None,
        }
    }
}

impl WorkspaceManagerApp {
    fn load_settings() -> (String, Vec<String>) {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("workspace-manager");

        let config_file = config_dir.join("settings.json");
        if config_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_file) {
                if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                    let terminal = settings
                        .get("terminal")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    return (terminal, Vec::new());
                }
            }
        }
        (String::new(), Vec::new())
    }

    fn save_settings(&self) {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("workspace-manager");

        let _ = std::fs::create_dir_all(&config_dir);
        let config_file = config_dir.join("settings.json");

        let settings = serde_json::json!({
            "terminal": self.selected_terminal
        });

        let _ = std::fs::write(
            &config_file,
            serde_json::to_string_pretty(&settings).unwrap_or_default(),
        );
    }

    fn load_workspaces(&mut self) {
        let path = std::path::Path::new(&self.folder_path);
        match list_workspaces(path) {
            Ok(ws) => {
                self.workspaces = ws;
                self.message = Message::None;
            }
            Err(e) => {
                self.workspaces.clear();
                self.message = Message::Error(e.to_string());
            }
        }
    }

    fn select_folder(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Workspace Folder")
            .pick_folder()
        {
            self.folder_path = path.to_string_lossy().to_string();
            self.load_workspaces();
        }
        ctx.request_repaint();
    }

    fn open_system_terminal(&mut self, path: std::path::PathBuf) {
        match open_system_terminal(&path, Some(&self.selected_terminal)) {
            Ok(()) => {
                self.terminal_message = Some(format!("Opened terminal in '{}'", path.display()));
            }
            Err(e) => {
                self.message = Message::Error(e);
            }
        }
    }

    fn show_create_dialog(&mut self) {
        self.dialog = DialogState::Create;
        self.new_name.clear();
        self.form = WorkspaceForm::default();
        self.new_extension.clear();
        self.new_setting_key.clear();
        self.new_setting_value.clear();
    }

    fn show_edit_dialog(&mut self, index: usize) {
        if let Some(ws) = self.workspaces.get(index) {
            self.dialog = DialogState::Edit(index);
            self.form = WorkspaceForm::from_config(&ws.config);
            self.new_extension.clear();
            self.new_setting_key.clear();
            self.new_setting_value.clear();
        }
    }

    fn show_delete_dialog(&mut self, index: usize) {
        self.dialog = DialogState::Delete(index);
    }

    fn show_editor_select(&mut self, index: usize) {
        self.dialog = DialogState::EditorSelect(index);
        self.show_custom_editor = false;
    }

    fn create_workspace(&mut self) {
        if self.new_name.is_empty() {
            self.message = Message::Error("Name cannot be empty".to_string());
            return;
        }

        let path = std::path::Path::new(&self.folder_path);
        let config = self.form.to_config();
        match create_workspace(path, &self.new_name, &config) {
            Ok(_) => {
                self.message = Message::Success(format!("Created '{}'", self.new_name));
                self.dialog = DialogState::None;
                self.load_workspaces();
            }
            Err(e) => {
                self.message = Message::Error(e.to_string());
            }
        }
    }

    fn update_workspace(&mut self, index: usize) {
        if let Some(ws) = self.workspaces.get(index) {
            let path = ws.path.clone();
            let config = self.form.to_config();
            match update_workspace(&path, &config) {
                Ok(_) => {
                    self.message = Message::Success(format!("Updated '{}'", ws.name));
                    self.dialog = DialogState::None;
                    self.load_workspaces();
                }
                Err(e) => {
                    self.message = Message::Error(e.to_string());
                }
            }
        }
    }

    fn delete_workspace(&mut self, index: usize) {
        if let Some(ws) = self.workspaces.get(index) {
            let path = ws.path.clone();
            let name = ws.name.clone();
            match delete_workspace(&path) {
                Ok(_) => {
                    self.message = Message::Success(format!("Deleted '{}'", name));
                    self.dialog = DialogState::None;
                    self.load_workspaces();
                }
                Err(e) => {
                    self.message = Message::Error(e.to_string());
                }
            }
        }
    }

    fn duplicate_workspace(&mut self, index: usize) {
        if let Some(ws) = self.workspaces.get(index) {
            let path = ws.path.clone();
            match duplicate_workspace(&path) {
                Ok(new_path) => {
                    self.message = Message::Success(format!(
                        "Duplicated to '{}'",
                        new_path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                    self.load_workspaces();
                }
                Err(e) => {
                    self.message = Message::Error(e.to_string());
                }
            }
        }
    }

    fn open_workspace(&mut self, index: usize, action: &str) {
        let (folder_path, ws_path, ws_name, config_folders) = match self.workspaces.get(index) {
            Some(ws) => {
                let folder_path = ws
                    .config
                    .folders
                    .first()
                    .map(|f| std::path::PathBuf::from(&f.path))
                    .unwrap_or_else(|| ws.path.parent().unwrap_or(&ws.path).to_path_buf());
                (
                    folder_path,
                    ws.path.clone(),
                    ws.name.clone(),
                    ws.config.folders.clone(),
                )
            }
            None => return,
        };

        match action {
            "terminal" => {
                self.open_system_terminal(folder_path);
            }
            "file_manager" => {
                let folder_path = config_folders
                    .first()
                    .map(|f| std::path::PathBuf::from(&f.path))
                    .unwrap_or_else(|| ws_path.clone());
                open_in_file_manager(&folder_path);
                self.message = Message::Info(format!("Opening file manager for '{}'", ws_name));
            }
            "editor" => {
                let editor = if self.show_custom_editor {
                    &self.custom_editor
                } else {
                    &self.selected_editor
                };

                if editor.is_empty() {
                    self.message = Message::Error("Please select or enter an editor".to_string());
                    return;
                }

                open_with_editor(&ws_path, editor);
                self.message = Message::Info(format!("Opening '{}' with {}", ws_name, editor));
            }
            "default" => {
                let _ = open::that(&ws_path);
                self.message = Message::Info(format!("Opening '{}' with default app", ws_name));
            }
            _ => {}
        }
    }

    fn render_form(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.strong("Folders");
            let folder_count = self.form.folders.len();
            let mut remove_folder = None;
            for i in 0..folder_count {
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", i + 1));
                    ui.text_edit_singleline(&mut self.form.folders[i].path);
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Select Folder")
                            .pick_folder()
                        {
                            self.form.folders[i].path = path.to_string_lossy().to_string();
                        }
                    }
                    if ui.button("Remove").clicked() && self.form.folders.len() > 1 {
                        remove_folder = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_folder {
                self.form.folders.remove(idx);
            }
            if ui.button("+ Add Folder").clicked() {
                self.form.folders.push(FormFolder {
                    path: String::new(),
                    name: String::new(),
                });
            }
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.strong("Settings");
            let settings_count = self.form.settings.entries.len();
            let mut remove_setting = None;
            for i in 0..settings_count {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.form.settings.entries[i].0);
                    ui.text_edit_singleline(&mut self.form.settings.entries[i].1);
                    if ui.button("Remove").clicked() {
                        remove_setting = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_setting {
                self.form.settings.entries.remove(idx);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_setting_key);
                ui.text_edit_singleline(&mut self.new_setting_value);
                if ui.button("+ Add").clicked() && !self.new_setting_key.is_empty() {
                    self.form
                        .settings
                        .entries
                        .push((self.new_setting_key.clone(), self.new_setting_value.clone()));
                    self.new_setting_key.clear();
                    self.new_setting_value.clear();
                }
            });
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.strong("Extensions");
            let ext_count = self.form.extensions.len();
            let mut remove_ext = None;
            for i in 0..ext_count {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.form.extensions[i]);
                    if ui.button("Remove").clicked() {
                        remove_ext = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_ext {
                self.form.extensions.remove(idx);
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_extension);
                if ui.button("+ Add").clicked() && !self.new_extension.is_empty() {
                    self.form.extensions.push(self.new_extension.clone());
                    self.new_extension.clear();
                }
            });
        });
    }
}

impl eframe::App for WorkspaceManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Select Folder").clicked() {
                        self.select_folder(ctx);
                        ui.close_menu();
                    }
                    if ui.button("Refresh").clicked() {
                        self.load_workspaces();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("New Workspace").clicked() {
                        self.show_create_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("Reload").clicked() {
                        self.load_workspaces();
                        ui.close_menu();
                    }
                    if ui.button("Terminal Settings").clicked() {
                        self.dialog = DialogState::TerminalSettings;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.dialog = DialogState::About;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("📁 Folder:");
                    ui.text_edit_singleline(&mut self.folder_path);
                    if ui.button("Browse").clicked() {
                        self.select_folder(ctx);
                    }
                    if ui.button("Refresh").clicked() {
                        self.load_workspaces();
                    }
                    ui.separator();
                    if ui.button("+ New Workspace").clicked() {
                        self.show_create_dialog();
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.text_edit_singleline(&mut self.search_query);
                    if !self.search_query.is_empty() {
                        if ui.button("Clear").clicked() {
                            self.search_query.clear();
                        }
                    }
                });

                ui.separator();

                let message = self.message.clone();
                match &message {
                    Message::Info(msg) => {
                        let msg = msg.clone();
                        ui.horizontal(|ui| {
                            ui.label("ℹ️");
                            ui.label(&msg);
                            if ui.button("×").clicked() {
                                self.message = Message::None;
                            }
                        });
                    }
                    Message::Error(msg) => {
                        let msg = msg.clone();
                        ui.horizontal(|ui| {
                            ui.label("❌");
                            ui.label(&msg);
                            if ui.button("×").clicked() {
                                self.message = Message::None;
                            }
                        });
                    }
                    Message::Success(msg) => {
                        let msg = msg.clone();
                        ui.horizontal(|ui| {
                            ui.label("✅");
                            ui.label(&msg);
                            if ui.button("×").clicked() {
                                self.message = Message::None;
                            }
                        });
                    }
                    Message::None => {}
                }

                if let Some(msg) = &self.terminal_message {
                    let msg = msg.clone();
                    ui.horizontal(|ui| {
                        ui.label("💻");
                        ui.label(&msg);
                        if ui.button("×").clicked() {
                            self.terminal_message = None;
                        }
                    });
                }

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.workspaces.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.vertical(|ui| {
                                ui.label("No workspace files found");
                                ui.label("Select a folder containing .code-workspace files");
                                ui.add_space(10.0);
                                if ui.button("Select Folder").clicked() {
                                    self.select_folder(ctx);
                                }
                            });
                        });
                    } else {
                        let filtered: Vec<(usize, String, u64, SystemTime, Vec<String>)> = self
                            .workspaces
                            .iter()
                            .enumerate()
                            .filter(|(_, ws)| {
                                self.search_query.is_empty()
                                    || ws
                                        .name
                                        .to_lowercase()
                                        .contains(&self.search_query.to_lowercase())
                            })
                            .map(|(idx, ws)| {
                                let folder_paths: Vec<String> = ws.config.folders.iter()
                                    .map(|f| f.path.clone())
                                    .collect();
                                (idx, ws.name.clone(), ws.size, ws.modified, folder_paths)
                            })
                            .collect();

                        if filtered.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label("No workspaces match your search");
                            });
                        } else {
                            let available_width = ui.available_width();
                            let card_min = 220.0;
                            let gap = 12.0;
                            let cols = ((available_width + gap) / (card_min + gap)).floor().max(1.0) as usize;
                            let card_width = (available_width + gap) / cols as f32 - gap;

                            for chunk in filtered.chunks(cols) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
                                    for (idx, name, size, modified, folder_paths) in chunk {
                                        let idx = *idx;
                                        let size = *size;
                                        let modified = *modified;
                                        ui.vertical(|ui| {
                                            ui.set_max_width(card_width);
                                            ui.set_min_width(card_width);

                                            egui::Frame::group(ui.style())
                                                .inner_margin(8.0)
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.add(egui::Label::new(
                                                            egui::RichText::new(name).strong(),
                                                        ).truncate());
                                                        ui.add_space(4.0);

                                                        if !folder_paths.is_empty() {
                                                            for fp in folder_paths {
                                                                ui.add(egui::Label::new(
                                                                    egui::RichText::new(fp)
                                                                        .size(11.0)
                                                                        .color(ui.visuals().weak_text_color()),
                                                                ).truncate());
                                                            }
                                                        } else {
                                                            ui.add(egui::Label::new(
                                                                egui::RichText::new("No folders")
                                                                    .size(11.0)
                                                                    .color(ui.visuals().weak_text_color()),
                                                            ).truncate());
                                                        }

                                                        ui.add_space(4.0);
                                                        ui.separator();
                                                        ui.horizontal(|ui| {
                                                            ui.label(egui::RichText::new(format_size(size)).size(10.0));
                                                            ui.label("|");
                                                            ui.label(egui::RichText::new(format_time(&modified)).size(10.0));
                                                        });

                                                        ui.add_space(6.0);
                                                        ui.horizontal(|ui| {
                                                            if ui.small_button("Open").clicked() {
                                                                self.pending_action = PendingAction::OpenDefault(idx);
                                                            }
                                                            if ui.small_button("Terminal").clicked() {
                                                                self.pending_action = PendingAction::OpenTerminal(idx);
                                                            }
                                                            if ui.small_button("Edit").clicked() {
                                                                self.pending_action = PendingAction::Edit(idx);
                                                            }
                                                            ui.menu_button("⋮", |ui| {
                                                                if ui.button("Open in File Manager").clicked() {
                                                                    self.pending_action = PendingAction::OpenFileManager(idx);
                                                                    ui.close_menu();
                                                                }
                                                                if ui.button("Open with Editor...").clicked() {
                                                                    self.pending_action = PendingAction::OpenEditor(idx);
                                                                    ui.close_menu();
                                                                }
                                                                ui.separator();
                                                                if ui.button("Duplicate").clicked() {
                                                                    self.pending_action = PendingAction::Duplicate(idx);
                                                                    ui.close_menu();
                                                                }
                                                                if ui.button("Delete").clicked() {
                                                                    self.pending_action = PendingAction::Delete(idx);
                                                                    ui.close_menu();
                                                                }
                                                            });
                                                        });
                                                    });
                                                });
                                        });
                                    }
                                });
                                ui.add_space(gap);
                            }
                        }
                    }
                });
            });
        });

        match std::mem::replace(&mut self.pending_action, PendingAction::None) {
            PendingAction::OpenDefault(idx) => self.open_workspace(idx, "default"),
            PendingAction::OpenFileManager(idx) => self.open_workspace(idx, "file_manager"),
            PendingAction::OpenEditor(idx) => self.show_editor_select(idx),
            PendingAction::OpenTerminal(idx) => {
                if let Some(ws) = self.workspaces.get(idx) {
                    let folder_path = ws
                        .config
                        .folders
                        .first()
                        .map(|f| std::path::PathBuf::from(&f.path))
                        .unwrap_or_else(|| ws.path.parent().unwrap_or(&ws.path).to_path_buf());
                    self.open_system_terminal(folder_path);
                }
            }
            PendingAction::Edit(idx) => self.show_edit_dialog(idx),
            PendingAction::Duplicate(idx) => self.duplicate_workspace(idx),
            PendingAction::Delete(idx) => self.show_delete_dialog(idx),
            PendingAction::None => {}
        }

        match &self.dialog {
            DialogState::Create => {
                egui::Window::new("New Workspace")
                    .collapsible(false)
                    .resizable(true)
                    .default_width(600.0)
                    .default_height(500.0)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.new_name).request_focus();
                            });

                            ui.add_space(10.0);
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                self.render_form(ui);
                            });

                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.button("Create").clicked() {
                                    self.create_workspace();
                                }
                                if ui.button("Cancel").clicked() {
                                    self.dialog = DialogState::None;
                                }
                            });
                        });
                    });
            }
            DialogState::Edit(index) => {
                let idx = *index;
                if let Some(ws) = self.workspaces.get(idx) {
                    let name = ws.name.clone();
                    let title = format!("Edit: {}", name);
                    egui::Window::new(title)
                        .collapsible(false)
                        .resizable(true)
                        .default_width(600.0)
                        .default_height(500.0)
                        .show(ctx, |ui| {
                            ui.vertical(|ui| {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    self.render_form(ui);
                                });

                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Save").clicked() {
                                        self.update_workspace(idx);
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.dialog = DialogState::None;
                                    }
                                });
                            });
                        });
                }
            }
            DialogState::Delete(index) => {
                let idx = *index;
                if let Some(ws) = self.workspaces.get(idx) {
                    let name = ws.name.clone();
                    let path = ws.path.display().to_string();
                    let title = format!("Delete: {}", name);
                    egui::Window::new(title)
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.vertical(|ui| {
                                ui.label(format!("Are you sure you want to delete '{}'?", name));
                                ui.label(format!("Path: {}", path));

                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Delete").clicked() {
                                        self.delete_workspace(idx);
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.dialog = DialogState::None;
                                    }
                                });
                            });
                        });
                }
            }
            DialogState::EditorSelect(index) => {
                let idx = *index;
                egui::Window::new("Select Editor")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.label("Choose an editor to open with:");

                            ui.add_space(10.0);
                            for editor in get_default_editors() {
                                if ui
                                    .selectable_value(
                                        &mut self.selected_editor,
                                        editor.to_string(),
                                        editor,
                                    )
                                    .clicked()
                                {
                                    self.open_workspace(idx, "editor");
                                    self.dialog = DialogState::None;
                                }
                            }

                            ui.separator();

                            ui.checkbox(&mut self.show_custom_editor, "Custom editor:");
                            if self.show_custom_editor {
                                ui.horizontal(|ui| {
                                    ui.label("Command:");
                                    ui.text_edit_singleline(&mut self.custom_editor);
                                });
                            }

                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if self.show_custom_editor && !self.custom_editor.is_empty() {
                                    if ui.button("Open").clicked() {
                                        self.open_workspace(idx, "editor");
                                        self.dialog = DialogState::None;
                                    }
                                }
                                if ui.button("Cancel").clicked() {
                                    self.dialog = DialogState::None;
                                }
                            });
                        });
                    });
            }
            DialogState::TerminalSettings => {
                egui::Window::new("Terminal Settings")
                    .collapsible(false)
                    .resizable(false)
                    .default_width(400.0)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.label("Select terminal emulator to use:");
                            ui.add_space(8.0);

                            let available = list_available_terminals();
                            if available.is_empty() {
                                ui.label("⚠️ No terminal emulators detected.");
                                ui.label("Install one of: gnome-terminal, konsole, alacritty, kitty, xterm");
                            } else {
                                for (cmd, name) in &available {
                                    let selected = cmd == &self.selected_terminal;
                                    if ui.selectable_label(selected, name).clicked() {
                                        self.selected_terminal = cmd.clone();
                                        self.save_settings();
                                    }
                                }
                            }

                            ui.add_space(12.0);
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Test Terminal").clicked() {
                                    self.save_settings();
                                    let home = dirs::home_dir().unwrap_or_default();
                                    self.open_system_terminal(home);
                                }
                                if ui.button("Close").clicked() {
                                    self.save_settings();
                                    self.dialog = DialogState::None;
                                }
                            });
                        });
                    });
            }
            DialogState::About => {
                egui::Window::new("About Workspace Manager")
                    .collapsible(false)
                    .resizable(false)
                    .default_width(350.0)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.centered_and_justified(|ui| {
                                ui.vertical(|ui| {
                                    ui.heading("Workspace Manager");
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(14.0));
                                    ui.add_space(12.0);
                                    ui.separator();
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new("Developer").strong());
                                    ui.label("Moe Kyaw Soe");
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new("Website").strong());
                                    if ui.link("www.moekyawsoe.com").clicked() {
                                        let _ = open::that("https://www.moekyawsoe.com");
                                    }
                                    ui.add_space(12.0);
                                    ui.separator();
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(env!("CARGO_PKG_DESCRIPTION")).size(12.0).color(ui.visuals().weak_text_color()));
                                    ui.add_space(16.0);
                                    if ui.button("Close").clicked() {
                                        self.dialog = DialogState::None;
                                    }
                                });
                            });
                        });
                    });
            }
            DialogState::None => {}
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Workspace Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "Workspace Manager",
        options,
        Box::new(|_cc| Ok(Box::new(WorkspaceManagerApp::default()))),
    )
}
