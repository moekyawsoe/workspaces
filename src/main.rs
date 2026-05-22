mod terminal;
mod update;
mod workspace;

use eframe::egui;
use std::collections::HashMap;
use std::time::SystemTime;
use terminal::*;
use update::*;
use workspace::*;

#[derive(Clone)]
enum ViewportCmd {
    Dock(usize),
    Close(usize),
    Error(String),
}

#[derive(Clone)]
struct FormFolder {
    path: String,
    name: String,
}

#[derive(Clone)]
struct FormUrl {
    label: String,
    url: String,
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
    urls: Vec<FormUrl>,
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
            urls: Vec::new(),
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

        let urls = config.urls.iter()
            .map(|u| FormUrl {
                label: u.label.clone(),
                url: u.url.clone(),
            })
            .collect();

        Self {
            folders,
            settings,
            extensions,
            urls,
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

        let urls: Vec<WorkspaceUrl> = self.urls.iter()
            .filter(|u| !u.url.is_empty())
            .map(|u| WorkspaceUrl::new(u.label.clone(), u.url.clone()))
            .collect();

        WorkspaceConfig {
            folders,
            settings,
            extensions,
            launch: None,
            urls,
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
    TerminalPreferences,
    About,
    ManageUrls(usize),
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
    OpenBuiltinTerminal(usize),
    OpenUrl(usize, usize),
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
    new_url_label: String,
    new_url: String,

    selected_terminal: String,
    terminal_message: Option<String>,

    terminal_tabs: Vec<TerminalTab>,
    floating_tabs: Vec<std::sync::Arc<parking_lot::Mutex<TerminalTab>>>,
    viewport_commands: std::sync::Arc<parking_lot::Mutex<Vec<ViewportCmd>>>,
    active_tab_index: usize,
    terminal_visible: bool,
    terminal_settings: TerminalSettings,
    discovered_fonts: Vec<(String, std::path::PathBuf)>,
    updater: Option<Updater>,
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

        let terminal_settings = load_terminal_settings();

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
            new_url_label: String::new(),
            new_url: String::new(),
            selected_terminal,
            terminal_message: None,
            terminal_tabs: Vec::new(),
            floating_tabs: Vec::new(),
            viewport_commands: std::sync::Arc::new(parking_lot::Mutex::new(Vec::new())),
            active_tab_index: 0,
            terminal_visible: false,
            terminal_settings,
            discovered_fonts: Vec::new(),
            updater: None,
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

    fn open_builtin_terminal(&mut self, name: Option<String>, working_dir: Option<std::path::PathBuf>) {
        if self.terminal_tabs.is_empty() {
            self.add_terminal_tab(name, working_dir);
        } else {
            self.terminal_visible = true;
        }
    }

    fn close_builtin_terminal(&mut self) {
        self.terminal_visible = false;
    }

    fn toggle_builtin_terminal(&mut self, name: Option<String>, working_dir: Option<std::path::PathBuf>) {
        if self.terminal_visible {
            self.close_builtin_terminal();
        } else {
            self.open_builtin_terminal(name, working_dir);
        }
    }

    fn add_terminal_tab(&mut self, name: Option<String>, working_dir: Option<std::path::PathBuf>) {
        let tab_name = name.unwrap_or_else(|| format!("Tab {}", self.terminal_tabs.len() + 1));
        match TerminalTab::new(tab_name, self.terminal_settings.clone(), working_dir) {
            Ok(tab) => {
                self.terminal_tabs.push(tab);
                self.active_tab_index = self.terminal_tabs.len() - 1;
                self.terminal_visible = true;
            }
            Err(e) => {
                self.message = Message::Error(format!("Failed to open terminal: {}", e));
            }
        }
    }

    fn close_tab(&mut self, index: usize) {
        if index < self.terminal_tabs.len() {
            self.terminal_tabs.remove(index);
            if self.terminal_tabs.is_empty() {
                self.terminal_visible = false;
                self.active_tab_index = 0;
            } else if self.active_tab_index >= self.terminal_tabs.len() {
                self.active_tab_index = self.terminal_tabs.len() - 1;
            }
        }
    }

    fn split_active_terminal(&mut self, direction: SplitDirection) {
        if self.terminal_tabs.is_empty() {
            return;
        }
        let tab = &mut self.terminal_tabs[self.active_tab_index];
        if let Some(active_id) = tab.active_terminal_id {
            // Try to get the current working directory from the active terminal
            let working_dir = tab.root.find_terminal_mut(active_id)
                .and_then(|t| t.current_working_dir())
                .or_else(|| dirs::home_dir());
            match Terminal::new(self.terminal_settings.clone(), working_dir) {
                Ok(new_term) => {
                    let new_id = new_term.id;
                    let mut term_opt = Some(new_term);
                    tab.root.split(active_id, direction, &mut term_opt);
                    tab.active_terminal_id = Some(new_id);
                }
                Err(e) => {
                    self.message = Message::Error(format!("Failed to split terminal: {}", e));
                }
            }
        }
    }

    fn duplicate_active_terminal(&mut self) {
        self.split_active_terminal(SplitDirection::Horizontal);
    }

    fn close_active_pane(&mut self) {
        if self.terminal_tabs.is_empty() {
            return;
        }
        let tab = &mut self.terminal_tabs[self.active_tab_index];
        if let Some(active_id) = tab.active_terminal_id {
            let mut removed = false;
            let opt_pane = tab.root.close_pane(active_id, &mut removed);
            if removed {
                if let Some(new_root) = opt_pane {
                    match new_root {
                        TerminalPane::Placeholder => {
                            let index_to_close = self.active_tab_index;
                            self.close_tab(index_to_close);
                        }
                        _ => {
                            tab.root = new_root;
                            tab.active_terminal_id = tab.root.any_terminal_id();
                        }
                    }
                } else {
                    tab.active_terminal_id = tab.root.any_terminal_id();
                }
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

    fn get_updater(&mut self, ctx: &egui::Context) -> &Updater {
        if self.updater.is_none() {
            self.updater = Some(Updater::new(ctx.clone()));
        }
        self.updater.as_ref().unwrap()
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

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.strong("URLs");
            let url_count = self.form.urls.len();
            let mut remove_url = None;
            for i in 0..url_count {
                ui.horizontal(|ui| {
                    ui.label("Label:");
                    ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut self.form.urls[i].label));
                    ui.label("URL:");
                    ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut self.form.urls[i].url));
                    if ui.button("Remove").clicked() {
                        remove_url = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_url {
                self.form.urls.remove(idx);
            }
            ui.horizontal(|ui| {
                ui.label("Label:");
                ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut self.new_url_label));
                ui.label("URL:");
                ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut self.new_url));
                if ui.button("+ Add").clicked() && !self.new_url.is_empty() {
                    self.form.urls.push(FormUrl {
                        label: self.new_url_label.clone(),
                        url: self.new_url.clone(),
                    });
                    self.new_url_label.clear();
                    self.new_url.clear();
                }
            });
        });
    }
}

impl eframe::App for WorkspaceManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Backtick) && (i.modifiers.ctrl || i.modifiers.command)) {
            let working_dir = dirs::home_dir();
            self.toggle_builtin_terminal(None, working_dir);
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            self.terminal_tabs.clear();
            self.floating_tabs.clear();
        }

        let viewport_commands = self.viewport_commands.clone();
        for (idx, tab_arc) in self.floating_tabs.iter().enumerate() {
            let tab_arc_clone = tab_arc.clone();
            let viewport_commands_clone = viewport_commands.clone();
            let terminal_settings_clone = self.terminal_settings.clone();
            let viewport_id = egui::ViewportId::from_hash_of(&(std::sync::Arc::as_ptr(&tab_arc) as usize));
            let tab_name = {
                let lock = tab_arc.lock();
                lock.name.clone()
            };

            ctx.show_viewport_immediate(
                viewport_id,
                egui::ViewportBuilder::default()
                    .with_title(&tab_name)
                    .with_inner_size(egui::vec2(700.0, 450.0)),
                move |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let mut tab_guard = tab_arc_clone.lock();
                        let tab: &mut TerminalTab = &mut *tab_guard;
                        ui.vertical(|ui| {
                            // Toolbar inside the floating window
                            ui.horizontal(|ui| {
                                if ui.small_button("⤵ Dock").on_hover_text("Dock back to bottom panel").clicked() {
                                    viewport_commands_clone.lock().push(ViewportCmd::Dock(idx));
                                }
                                ui.separator();
                                if ui.small_button("x Close Pane").on_hover_text("Close active pane").clicked() {
                                    if let Some(active_id) = tab.active_terminal_id {
                                        let mut removed = false;
                                        let opt_pane = tab.root.close_pane(active_id, &mut removed);
                                        if removed {
                                            if let Some(new_root) = opt_pane {
                                                match new_root {
                                                    TerminalPane::Placeholder => {
                                                        viewport_commands_clone.lock().push(ViewportCmd::Close(idx));
                                                    }
                                                    _ => {
                                                        tab.root = new_root;
                                                        tab.active_terminal_id = tab.root.any_terminal_id();
                                                    }
                                                }
                                            } else {
                                                tab.active_terminal_id = tab.root.any_terminal_id();
                                            }
                                        }
                                    }
                                }
                                if ui.small_button("| Split V").on_hover_text("Split active pane vertically").clicked() {
                                    if let Some(active_id) = tab.active_terminal_id {
                                        let working_dir = tab.root.find_terminal_mut(active_id)
                                            .and_then(|t| t.current_working_dir())
                                            .or_else(|| dirs::home_dir());
                                        match Terminal::new(terminal_settings_clone.clone(), working_dir) {
                                            Ok(new_term) => {
                                                let new_id = new_term.id;
                                                let mut term_opt = Some(new_term);
                                                tab.root.split(active_id, SplitDirection::Vertical, &mut term_opt);
                                                tab.active_terminal_id = Some(new_id);
                                            }
                                            Err(e) => {
                                                viewport_commands_clone.lock().push(ViewportCmd::Error(format!("Failed to split terminal: {}", e)));
                                            }
                                        }
                                    }
                                }
                                if ui.small_button("| Split H").on_hover_text("Split active pane horizontally").clicked() {
                                    if let Some(active_id) = tab.active_terminal_id {
                                        let working_dir = tab.root.find_terminal_mut(active_id)
                                            .and_then(|t| t.current_working_dir())
                                            .or_else(|| dirs::home_dir());
                                        match Terminal::new(terminal_settings_clone.clone(), working_dir) {
                                            Ok(new_term) => {
                                                let new_id = new_term.id;
                                                let mut term_opt = Some(new_term);
                                                tab.root.split(active_id, SplitDirection::Horizontal, &mut term_opt);
                                                tab.active_terminal_id = Some(new_id);
                                            }
                                            Err(e) => {
                                                viewport_commands_clone.lock().push(ViewportCmd::Error(format!("Failed to split terminal: {}", e)));
                                            }
                                        }
                                    }
                                }
                                if ui.small_button("🗐 Duplicate").on_hover_text("Duplicate active pane").clicked() {
                                    if let Some(active_id) = tab.active_terminal_id {
                                        let working_dir = tab.root.find_terminal_mut(active_id)
                                            .and_then(|t| t.current_working_dir())
                                            .or_else(|| dirs::home_dir());
                                        match Terminal::new(terminal_settings_clone.clone(), working_dir) {
                                            Ok(new_term) => {
                                                let new_id = new_term.id;
                                                let mut term_opt = Some(new_term);
                                                tab.root.split(active_id, SplitDirection::Horizontal, &mut term_opt);
                                                tab.active_terminal_id = Some(new_id);
                                            }
                                            Err(e) => {
                                                viewport_commands_clone.lock().push(ViewportCmd::Error(format!("Failed to split terminal: {}", e)));
                                            }
                                        }
                                    }
                                }
                            });
                            ui.separator();

                            let active_id = &mut tab.active_terminal_id;
                            tab.root.render(ui, active_id, ctx);
                        });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        viewport_commands_clone.lock().push(ViewportCmd::Close(idx));
                    }
                }
            );
        }

        // Process viewport commands at the end of the frame
        let mut commands = Vec::new();
        {
            let mut lock = self.viewport_commands.lock();
            if !lock.is_empty() {
                commands = std::mem::take(&mut *lock);
            }
        }

        if !commands.is_empty() {
            let mut removals = Vec::new(); // stores (index, should_dock)
            for cmd in commands {
                match cmd {
                    ViewportCmd::Dock(idx) => removals.push((idx, true)),
                    ViewportCmd::Close(idx) => removals.push((idx, false)),
                    ViewportCmd::Error(e) => {
                        self.message = Message::Error(e);
                    }
                }
            }

            // Sort by index descending to avoid index shifting when removing elements
            removals.sort_by(|a, b| b.0.cmp(&a.0));
            removals.dedup_by(|a, b| a.0 == b.0);

            for (idx, should_dock) in removals {
                if idx < self.floating_tabs.len() {
                    let tab_arc = self.floating_tabs.remove(idx);
                    if should_dock {
                        let tab = std::mem::replace(
                            &mut *tab_arc.lock(),
                            TerminalTab {
                                name: String::new(),
                                root: TerminalPane::Placeholder,
                                active_terminal_id: None,
                            },
                        );
                        self.terminal_tabs.push(tab);
                        self.active_tab_index = self.terminal_tabs.len() - 1;
                        self.terminal_visible = true;
                    }
                }
            }
        }

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
                    ui.separator();
                    let terminal_label = if self.terminal_visible { "Hide Terminal" } else { "Show Terminal" };
                    if ui.button(terminal_label).clicked() {
                        let working_dir = dirs::home_dir();
                        self.toggle_builtin_terminal(None, working_dir);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Terminal Settings").clicked() {
                        self.dialog = DialogState::TerminalSettings;
                        ui.close_menu();
                    }
                    if ui.button("Terminal Preferences").clicked() {
                        self.dialog = DialogState::TerminalPreferences;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Check for Updates").clicked() {
                        self.dialog = DialogState::About;
                        ui.close_menu();
                    }
                    if ui.button("About").clicked() {
                        self.dialog = DialogState::About;
                        ui.close_menu();
                    }
                });
            });
        });

        if self.terminal_visible {
            egui::TopBottomPanel::bottom("terminal_panel")
                .resizable(true)
                .default_height(300.0)
                .min_height(100.0)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            // Tab list
                            let mut tab_to_close = None;
                            let mut tab_to_float = None;
                            for (idx, tab) in self.terminal_tabs.iter().enumerate() {
                                let is_active = idx == self.active_tab_index;
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    let text = format!(" {} ", tab.name);
                                    if ui.selectable_label(is_active, text).clicked() {
                                        self.active_tab_index = idx;
                                    }
                                    if ui.small_button("↗").on_hover_text("Float tab").clicked() {
                                        tab_to_float = Some(idx);
                                    }
                                    if ui.small_button("x").clicked() {
                                        tab_to_close = Some(idx);
                                    }
                                });
                                ui.add_space(8.0);
                            }
                            if let Some(idx) = tab_to_float {
                                let tab = self.terminal_tabs.remove(idx);
                                self.floating_tabs.push(std::sync::Arc::new(parking_lot::Mutex::new(tab)));
                                if self.terminal_tabs.is_empty() {
                                    self.terminal_visible = false;
                                } else if self.active_tab_index >= self.terminal_tabs.len() {
                                    self.active_tab_index = self.terminal_tabs.len() - 1;
                                }
                            }
                            if let Some(idx) = tab_to_close {
                                self.close_tab(idx);
                            }
                            if ui.button("+").on_hover_text("New Tab").clicked() {
                                let working_dir = dirs::home_dir();
                                self.add_terminal_tab(None, working_dir);
                            }
                            
                            // Controls on the right
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("x").on_hover_text("Hide Terminal").clicked() {
                                    self.close_builtin_terminal();
                                }
                                if ui.small_button("⚙").on_hover_text("Preferences").clicked() {
                                    self.dialog = DialogState::TerminalPreferences;
                                }
                                ui.separator();
                                if ui.small_button("x Close Pane").on_hover_text("Close active pane").clicked() {
                                    self.close_active_pane();
                                }
                                if ui.small_button("| Split V").on_hover_text("Split active pane vertically").clicked() {
                                    self.split_active_terminal(SplitDirection::Vertical);
                                }
                                if ui.small_button("| Split H").on_hover_text("Split active pane horizontally").clicked() {
                                    self.split_active_terminal(SplitDirection::Horizontal);
                                }
                                if ui.small_button("🗐 Duplicate").on_hover_text("Duplicate active pane").clicked() {
                                    self.duplicate_active_terminal();
                                }
                                if ui.small_button("↗ Float Tab").on_hover_text("Float current tab").clicked() {
                                    if !self.terminal_tabs.is_empty() {
                                        let idx = self.active_tab_index;
                                        let tab = self.terminal_tabs.remove(idx);
                                        self.floating_tabs.push(std::sync::Arc::new(parking_lot::Mutex::new(tab)));
                                        if self.terminal_tabs.is_empty() {
                                            self.terminal_visible = false;
                                        } else if self.active_tab_index >= self.terminal_tabs.len() {
                                            self.active_tab_index = self.terminal_tabs.len() - 1;
                                        }
                                    }
                                }
                            });
                        });
                        ui.separator();
                        
                        if !self.terminal_tabs.is_empty() {
                            if self.active_tab_index >= self.terminal_tabs.len() {
                                self.active_tab_index = self.terminal_tabs.len() - 1;
                            }
                            let tab = &mut self.terminal_tabs[self.active_tab_index];
                            let active_id = &mut tab.active_terminal_id;
                            tab.root.render(ui, active_id, ctx);
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label("No tabs open. Click '+' to open a terminal.");
                            });
                        }
                    });
                });
        }

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
                            if ui.button("x").clicked() {
                                self.message = Message::None;
                            }
                        });
                    }
                    Message::Error(msg) => {
                        let msg = msg.clone();
                        ui.horizontal(|ui| {
                            ui.label("❌");
                            ui.label(&msg);
                            if ui.button("x").clicked() {
                                self.message = Message::None;
                            }
                        });
                    }
                    Message::Success(msg) => {
                        let msg = msg.clone();
                        ui.horizontal(|ui| {
                            ui.label("✅");
                            ui.label(&msg);
                            if ui.button("x").clicked() {
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
                        if ui.button("x").clicked() {
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
                        let filtered: Vec<(usize, String, u64, SystemTime, Vec<String>, Vec<(String, String)>)> = self
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
                                let urls: Vec<(String, String)> = ws.config.urls.iter()
                                    .map(|u| (u.label.clone(), u.url.clone()))
                                    .collect();
                                (idx, ws.name.clone(), ws.size, ws.modified, folder_paths, urls)
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
                                    for (idx, name, size, modified, folder_paths, urls) in chunk {
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

                                                        if !urls.is_empty() {
                                                            ui.add_space(4.0);
                                                            for (url_label, url_value) in urls {
                                                                ui.horizontal(|ui| {
                                                                    let label_text = if url_label.is_empty() {
                                                                        url_value.clone()
                                                                    } else {
                                                                        url_label.clone()
                                                                    };
                                                                    ui.add(egui::Label::new(
                                                                        egui::RichText::new(label_text)
                                                                            .size(11.0)
                                                                            .color(ui.visuals().hyperlink_color),
                                                                    ).truncate());
                                                                    if ui.small_button("🌐").on_hover_text(format!("Open {}", url_value)).clicked() {
                                                                        self.pending_action = PendingAction::OpenUrl(idx, urls.iter().position(|(_, u)| u == url_value).unwrap_or(0));
                                                                    }
                                                                });
                                                            }
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
                                                            if !urls.is_empty() && ui.small_button("URLs").clicked() {
                                                                self.pending_action = PendingAction::OpenUrl(idx, 0);
                                                            }
                                                            ui.menu_button("⋮", |ui| {
                                                                if ui.button("Open in Built-in Terminal").clicked() {
                                                                    self.pending_action = PendingAction::OpenBuiltinTerminal(idx);
                                                                    ui.close_menu();
                                                                }
                                                                if ui.button("Open in File Manager").clicked() {
                                                                    self.pending_action = PendingAction::OpenFileManager(idx);
                                                                    ui.close_menu();
                                                                }
                                                                if ui.button("Open with Editor...").clicked() {
                                                                    self.pending_action = PendingAction::OpenEditor(idx);
                                                                    ui.close_menu();
                                                                }
                                                                if ui.button("Manage URLs").clicked() {
                                                                    self.dialog = DialogState::ManageUrls(idx);
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
            PendingAction::OpenUrl(idx, url_idx) => {
                if let Some(ws) = self.workspaces.get(idx) {
                    if ws.config.urls.is_empty() {
                        self.dialog = DialogState::ManageUrls(idx);
                    } else if let Some(url_entry) = ws.config.urls.get(url_idx) {
                        open_url(&url_entry.url);
                        self.message = Message::Info(format!("Opening '{}' in browser", url_entry.label));
                    }
                }
            }
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
            PendingAction::OpenBuiltinTerminal(idx) => {
                if let Some(ws) = self.workspaces.get(idx) {
                    let folder_path = ws
                        .config
                        .folders
                        .first()
                        .map(|f| std::path::PathBuf::from(&f.path))
                        .unwrap_or_else(|| ws.path.parent().unwrap_or(&ws.path).to_path_buf());
                    self.add_terminal_tab(Some(ws.name.clone()), Some(folder_path));
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
            DialogState::TerminalPreferences => {
                egui::Window::new("Terminal Preferences")
                    .collapsible(false)
                    .resizable(true)
                    .default_width(450.0)
                    .default_height(400.0)
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.heading("Terminal Preferences");
                            ui.add_space(10.0);

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.group(|ui| {
                                    ui.strong("Theme");
                                    ui.add_space(5.0);
                                    let themes = ["Catppuccin Mocha", "Ubuntu", "WSL"];
                                    let current_theme = match self.terminal_settings.theme.background {
                                        c if c == TerminalTheme::catppuccin_mocha().background => 0,
                                        c if c == TerminalTheme::ubuntu().background => 1,
                                        _ => 2,
                                    };
                                    for (i, theme_name) in themes.iter().enumerate() {
                                        let selected = i == current_theme;
                                        if ui.selectable_value(&mut String::new(), if selected { "selected".to_string() } else { String::new() }, *theme_name).clicked() {
                                            self.terminal_settings.theme = match i {
                                                0 => TerminalTheme::catppuccin_mocha(),
                                                1 => TerminalTheme::ubuntu(),
                                                _ => TerminalTheme::wsl(),
                                            };
                                        }
                                    }
                                });

                                ui.add_space(10.0);

                                ui.group(|ui| {
                                    ui.strong("Font");
                                    ui.add_space(5.0);
                                    ui.horizontal(|ui| {
                                        ui.label("Family:");
                                        let selected_font = self.terminal_settings.font.family.clone();
                                        egui::ComboBox::from_id_source("terminal_font_family_combo")
                                            .selected_text(&selected_font)
                                            .show_ui(ui, |ui| {
                                                let is_default = selected_font == "JetBrains Mono" || selected_font.is_empty();
                                                if ui.selectable_label(is_default, "System Monospace").clicked() {
                                                    self.terminal_settings.font.family = "JetBrains Mono".to_string();
                                                }
                                                for (name, _) in &self.discovered_fonts {
                                                    let is_selected = selected_font == *name;
                                                    if ui.selectable_label(is_selected, name).clicked() {
                                                        self.terminal_settings.font.family = name.clone();
                                                    }
                                                }
                                            });
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Size:");
                                        ui.add(egui::DragValue::new(&mut self.terminal_settings.font.size)
                                            .range(8.0..=32.0)
                                            .speed(0.5));
                                    });
                                });

                                ui.add_space(10.0);

                                ui.group(|ui| {
                                    ui.strong("Shell");
                                    ui.add_space(5.0);
                                    ui.horizontal(|ui| {
                                        ui.label("Path:");
                                        ui.text_edit_singleline(&mut self.terminal_settings.shell);
                                    });
                                    ui.label(egui::RichText::new("Leave empty to use system default").size(11.0).color(ui.visuals().weak_text_color()));
                                });

                                ui.add_space(10.0);

                                ui.group(|ui| {
                                    ui.strong("Scrollback");
                                    ui.add_space(5.0);
                                    ui.horizontal(|ui| {
                                        ui.label("Lines:");
                                        ui.add(egui::DragValue::new(&mut self.terminal_settings.scrollback_lines)
                                            .range(100..=100000)
                                            .speed(100));
                                    });
                                });
                            });

                            ui.add_space(12.0);
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    save_terminal_settings(&self.terminal_settings);
                                    apply_font(ctx, &self.terminal_settings.font.family, &self.discovered_fonts);
                                    for tab in &mut self.terminal_tabs {
                                        tab.root.update_settings(&self.terminal_settings);
                                    }
                                    for tab in &mut self.floating_tabs {
                                        tab.lock().root.update_settings(&self.terminal_settings);
                                    }
                                    self.dialog = DialogState::None;
                                }
                                if ui.button("Reset").clicked() {
                                    self.terminal_settings = TerminalSettings::default();
                                    save_terminal_settings(&self.terminal_settings);
                                    apply_font(ctx, &self.terminal_settings.font.family, &self.discovered_fonts);
                                    for tab in &mut self.terminal_tabs {
                                        tab.root.update_settings(&self.terminal_settings);
                                    }
                                    for tab in &mut self.floating_tabs {
                                        tab.lock().root.update_settings(&self.terminal_settings);
                                    }
                                }
                                if ui.button("Close").clicked() {
                                    self.dialog = DialogState::None;
                                }
                            });
                        });
                    });
            }
            DialogState::ManageUrls(index) => {
                let idx = *index;
                if let Some(ws) = self.workspaces.get(idx) {
                    let name = ws.name.clone();
                    let ws_path = ws.path.clone();
                    let title = format!("Manage URLs: {}", name);
                    egui::Window::new(title)
                        .collapsible(false)
                        .resizable(true)
                        .default_width(550.0)
                        .default_height(400.0)
                        .show(ctx, |ui| {
                            ui.vertical(|ui| {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.group(|ui| {
                                        ui.strong("Saved URLs");
                                        ui.add_space(5.0);

                                        let urls = {
                                            let ws_opt = self.workspaces.get(idx);
                                            ws_opt.map(|w| w.config.urls.clone()).unwrap_or_default()
                                        };
                                        let mut urls_to_remove: Vec<usize> = Vec::new();

                                        for (i, url_entry) in urls.iter().enumerate() {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(if url_entry.label.is_empty() { &url_entry.url } else { &url_entry.label }).strong().size(12.0));
                                                ui.add_space(8.0);
                                                ui.add(egui::Label::new(egui::RichText::new(&url_entry.url).size(11.0).color(ui.visuals().weak_text_color())).truncate());
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.small_button("🌐 Launch").on_hover_text("Open in browser").clicked() {
                                                        open_url(&url_entry.url);
                                                        self.message = Message::Info(format!("Opening '{}' in browser", url_entry.label));
                                                    }
                                                    if ui.small_button("🗑").on_hover_text("Delete URL").clicked() {
                                                        urls_to_remove.push(i);
                                                    }
                                                });
                                            });
                                            ui.add_space(4.0);
                                        }

                                        if !urls_to_remove.is_empty() {
                                            let ws_opt = self.workspaces.get(idx);
                                            if let Some(ws_ref) = ws_opt {
                                                let mut config = ws_ref.config.clone();
                                                for i in urls_to_remove.iter().rev() {
                                                    config.urls.remove(*i);
                                                }
                                                if let Err(e) = update_workspace(&ws_path, &config) {
                                                    self.message = Message::Error(e.to_string());
                                                } else {
                                                    self.load_workspaces();
                                                }
                                            }
                                        }

                                        if urls.is_empty() {
                                            ui.label(egui::RichText::new("No URLs saved yet. Add one below.").size(11.0).color(ui.visuals().weak_text_color()));
                                        }
                                    });

                                    ui.add_space(10.0);

                                    ui.group(|ui| {
                                        ui.strong("Add New URL");
                                        ui.add_space(5.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Label:");
                                            ui.add_sized([130.0, 20.0], egui::TextEdit::singleline(&mut self.new_url_label));
                                            ui.label("URL:");
                                            ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut self.new_url));
                                            if ui.button("+ Add").clicked() && !self.new_url.is_empty() {
                                                let ws_opt = self.workspaces.get(idx);
                                                if let Some(ws_ref) = ws_opt {
                                                    let mut config = ws_ref.config.clone();
                                                    config.urls.push(WorkspaceUrl::new(
                                                        self.new_url_label.clone(),
                                                        self.new_url.clone(),
                                                    ));
                                                    if let Err(e) = update_workspace(&ws_path, &config) {
                                                        self.message = Message::Error(e.to_string());
                                                    } else {
                                                        self.new_url_label.clear();
                                                        self.new_url.clear();
                                                        self.load_workspaces();
                                                    }
                                                }
                                            }
                                        });
                                    });
                                });

                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Close").clicked() {
                                        self.dialog = DialogState::None;
                                    }
                                });
                            });
                        });
                }
            }
            DialogState::About => {
                egui::Window::new("About Workspace Manager")
                    .collapsible(false)
                    .resizable(false)
                    .default_width(380.0)
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

                                    ui.group(|ui| {
                                        ui.strong("Updates");
                                        ui.add_space(6.0);

                                        let updater = self.get_updater(ctx);
                                        let status = updater.get_status();

                                        match &status {
                                            UpdateStatus::Idle => {
                                                if ui.button("Check for Updates").clicked() {
                                                    updater.check_for_updates();
                                                }
                                            }
                                            UpdateStatus::Checking => {
                                                ui.horizontal(|ui| {
                                                    ui.spinner();
                                                    ui.label("Checking for updates...");
                                                });
                                            }
                                            UpdateStatus::UpToDate => {
                                                ui.horizontal(|ui| {
                                                    ui.label("✅");
                                                    ui.label("You are running the latest version.");
                                                });
                                                ui.add_space(4.0);
                                                if ui.small_button("Check Again").clicked() {
                                                    updater.reset();
                                                    updater.check_for_updates();
                                                }
                                            }
                                            UpdateStatus::UpdateAvailable { version, release_notes } => {
                                                ui.label(egui::RichText::new(format!("New version {} available!", version)).strong().color(egui::Color32::GREEN));
                                                ui.add_space(4.0);

                                                if !release_notes.is_empty() {
                                                    egui::ScrollArea::vertical()
                                                        .max_height(80.0)
                                                        .show(ui, |ui| {
                                                            ui.label(egui::RichText::new(release_notes).size(11.0).color(ui.visuals().weak_text_color()));
                                                        });
                                                    ui.add_space(4.0);
                                                }

                                                ui.horizontal(|ui| {
                                                    if ui.button("Download & Install").clicked() {
                                                        updater.install_update();
                                                    }
                                                    if ui.small_button("Dismiss").clicked() {
                                                        updater.reset();
                                                    }
                                                });
                                            }
                                            UpdateStatus::Downloading(progress) => {
                                                ui.label("Downloading update...");
                                                ui.add(egui::ProgressBar::new(*progress)
                                                    .text(format!("{:.0}%", progress * 100.0)));
                                            }
                                            UpdateStatus::Installing => {
                                                ui.horizontal(|ui| {
                                                    ui.spinner();
                                                    ui.label("Installing update...");
                                                });
                                            }
                                            UpdateStatus::Success => {
                                                ui.label("✅ Update installed! Restarting...");
                                            }
                                            UpdateStatus::Error(msg) => {
                                                ui.label(egui::RichText::new(format!("Error: {}", msg)).color(ui.visuals().error_fg_color));
                                                ui.add_space(4.0);
                                                ui.horizontal(|ui| {
                                                    if ui.button("Retry").clicked() {
                                                        updater.reset();
                                                        updater.check_for_updates();
                                                    }
                                                    if ui.small_button("Dismiss").clicked() {
                                                        updater.reset();
                                                    }
                                                });
                                            }
                                        }
                                    });

                                    ui.add_space(12.0);
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

fn discover_fonts() -> Vec<(String, std::path::PathBuf)> {
    let mut fonts = Vec::new();
    let mut search_paths = vec![
        std::path::PathBuf::from("/usr/share/fonts"),
        std::path::PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = dirs::home_dir() {
        search_paths.push(home.join(".local/share/fonts"));
        search_paths.push(home.join(".fonts"));
    }

    let mut stack = search_paths;
    let mut visited = std::collections::HashSet::new();
    while let Some(dir) = stack.pop() {
        if let Ok(canonical) = dir.canonicalize() {
            if !visited.insert(canonical) {
                continue;
            }
        } else {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ext_lower == "ttf" || ext_lower == "otf" {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            fonts.push((stem.to_string(), path));
                        }
                    }
                }
            }
        }
    }

    fonts.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    fonts.dedup_by(|a, b| a.0 == b.0);
    fonts
}

fn apply_font(ctx: &egui::Context, family_name: &str, discovered_fonts: &[(String, std::path::PathBuf)]) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some((_, path)) = discovered_fonts.iter().find(|(name, _)| name == family_name) {
        if let Ok(font_bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                family_name.to_owned(),
                egui::FontData::from_owned(font_bytes),
            );
            fonts.families.entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, family_name.to_owned());
        }
    }

    if let Ok(myanmar_bytes) = std::fs::read("/usr/share/fonts/truetype/noto/NotoSansMyanmar-Regular.ttf") {
        fonts.font_data.insert(
            "NotoSansMyanmar".to_owned(),
            egui::FontData::from_owned(myanmar_bytes),
        );
        
        let proportional_list = fonts.families.entry(egui::FontFamily::Proportional).or_default();
        if !proportional_list.contains(&"NotoSansMyanmar".to_owned()) {
            let idx = proportional_list.len().min(1);
            proportional_list.insert(idx, "NotoSansMyanmar".to_owned());
        }

        let monospace_list = fonts.families.entry(egui::FontFamily::Monospace).or_default();
        if !monospace_list.contains(&"NotoSansMyanmar".to_owned()) {
            let idx = monospace_list.len().min(1);
            monospace_list.insert(idx, "NotoSansMyanmar".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

fn main() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic payload"
        };

        if msg.contains("accesskit") || msg.contains("panic in a destructor") {
            unsafe { libc::_exit(0); }
        }

        let location = info.location()
            .map(|l| format!("at {}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let full_msg = format!("Application crashed due to a panic.\n\nMessage: {}\nLocation: {}", msg, location);

        rfd::MessageDialog::new()
            .set_title("Fatal Error")
            .set_description(&full_msg)
            .set_level(rfd::MessageLevel::Error)
            .show();

        default_hook(info);
    }));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Workspace Manager"),
        ..Default::default()
    };

    let app_creator = Box::new(|cc: &eframe::CreationContext<'_>| {
        let discovered = discover_fonts();
        let settings = load_terminal_settings();

        apply_font(&cc.egui_ctx, &settings.font.family, &discovered);

        let mut app = WorkspaceManagerApp::default();
        app.discovered_fonts = discovered;
        app.terminal_settings = settings;

        Ok(Box::new(app) as Box<dyn eframe::App>)
    });

    let res = eframe::run_native("Workspace Manager", options, app_creator);
    if let Err(e) = res {
        let err_msg = format!(
            "Failed to start the application.\n\nError: {:?}\n\nThis is usually caused by missing or outdated graphics drivers (OpenGL 3.3+ support is required).",
            e
        );
        rfd::MessageDialog::new()
            .set_title("Startup Error")
            .set_description(&err_msg)
            .set_level(rfd::MessageLevel::Error)
            .show();
    }

    unsafe { libc::_exit(0); }
}
