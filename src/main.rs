mod workspace;
mod terminal;

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
                    let json_value: serde_json::Value =
                        serde_json::from_str(value).unwrap_or(serde_json::Value::String(value.clone()));
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

    terminals: Vec<Vec<terminal::TerminalInstance>>,
    active_terminal: usize,
    terminal_open: bool,
    terminal_focused_split: Option<usize>, // which split pane has keyboard focus
    terminal_focus_requested: bool,
    terminal_height: f32,
    selected_shell: String,
    custom_shell: String,
    show_custom_shell: bool,
}

impl Default for WorkspaceManagerApp {
    fn default() -> Self {
        let folder_path = dirs::home_dir()
            .unwrap_or_default()
            .join("workspaces")
            .to_string_lossy()
            .to_string();

        let terminals = vec![vec![terminal::TerminalInstance::new(std::path::PathBuf::from(&folder_path))]];

        #[cfg(target_os = "windows")]
        let selected_shell = "cmd.exe".to_string();
        #[cfg(not(target_os = "windows"))]
        let selected_shell = "/bin/bash".to_string();

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

            terminals,
            active_terminal: 0,
            terminal_open: false,
            terminal_focused_split: None,
            terminal_focus_requested: false,
            terminal_height: 250.0,
            selected_shell,
            custom_shell: String::new(),
            show_custom_shell: false,
        }
    }
}

impl WorkspaceManagerApp {
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
            self.open_terminal_for_path(path, ctx);
        }
        ctx.request_repaint();
    }

    fn get_active_shell_path(&self) -> String {
        if self.show_custom_shell {
            self.custom_shell.clone()
        } else {
            self.selected_shell.clone()
        }
    }

    fn open_terminal_for_path(&mut self, path: std::path::PathBuf, ctx: &egui::Context) {
        let shell = self.get_active_shell_path();
        if self.terminals.is_empty() {
            let mut term = terminal::TerminalInstance::new(path);
            term.start(&shell, ctx.clone());
            self.terminals.push(vec![term]);
            self.active_terminal = 0;
        } else if self.terminals.len() == 1 && self.terminals[0].len() == 1 && self.terminals[0][0].pty_writer.is_none() {
            self.terminals[0][0].change_dir(path, &shell, ctx.clone());
            self.active_terminal = 0;
        } else {
            let mut term = terminal::TerminalInstance::new(path);
            term.start(&shell, ctx.clone());
            self.terminals.push(vec![term]);
            self.active_terminal = self.terminals.len() - 1;
        }
        self.terminal_open = true;
        self.terminal_focus_requested = false;
    }

    fn show_create_dialog(&mut self, ctx: &egui::Context) {
        self.dialog = DialogState::Create;
        self.new_name.clear();
        self.form = WorkspaceForm::default();
        self.new_extension.clear();
        self.new_setting_key.clear();
        self.new_setting_value.clear();
        ctx.request_repaint();
    }

    fn show_edit_dialog(&mut self, index: usize, ctx: &egui::Context) {
        if let Some(ws) = self.workspaces.get(index) {
            self.dialog = DialogState::Edit(index);
            self.form = WorkspaceForm::from_config(&ws.config);
            self.new_extension.clear();
            self.new_setting_key.clear();
            self.new_setting_value.clear();
            ctx.request_repaint();
        }
    }

    fn show_delete_dialog(&mut self, index: usize, ctx: &egui::Context) {
        self.dialog = DialogState::Delete(index);
        ctx.request_repaint();
    }

    fn show_editor_select(&mut self, index: usize, ctx: &egui::Context) {
        self.dialog = DialogState::EditorSelect(index);
        self.show_custom_editor = false;
        ctx.request_repaint();
    }

    fn create_workspace(&mut self, ctx: &egui::Context) {
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
        ctx.request_repaint();
    }

    fn update_workspace(&mut self, index: usize, ctx: &egui::Context) {
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
        ctx.request_repaint();
    }

    fn delete_workspace(&mut self, index: usize, ctx: &egui::Context) {
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
        ctx.request_repaint();
    }

    fn duplicate_workspace(&mut self, index: usize, ctx: &egui::Context) {
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
        ctx.request_repaint();
    }

    fn open_workspace(&mut self, index: usize, action: &str, ctx: &egui::Context) {
        let (folder_path, ws_path, ws_name, config_folders) = match self.workspaces.get(index) {
            Some(ws) => {
                let folder_path = ws.config.folders.first()
                    .map(|f| std::path::PathBuf::from(&f.path))
                    .unwrap_or_else(|| ws.path.parent().unwrap_or(&ws.path).to_path_buf());
                (folder_path, ws.path.clone(), ws.name.clone(), ws.config.folders.clone())
            }
            None => return,
        };

        self.open_terminal_for_path(folder_path, ctx);

        match action {
            "file_manager" => {
                let folder_path = config_folders.first()
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
                self.message =
                    Message::Info(format!("Opening '{}' with {}", ws_name, editor));
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
                if ui.button("+ Add").clicked()
                    && !self.new_setting_key.is_empty()
                {
                    self.form.settings.entries.push((
                        self.new_setting_key.clone(),
                        self.new_setting_value.clone(),
                    ));
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
                        self.show_create_dialog(ctx);
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("Reload").clicked() {
                        self.load_workspaces();
                        ui.close_menu();
                    }
                    if ui.selectable_label(self.terminal_open, "Terminal").clicked() {
                        self.terminal_open = !self.terminal_open;
                        if self.terminal_open {
                            self.terminal_focus_requested = false;
                            if !self.terminals.is_empty() && !self.terminals[self.active_terminal].is_empty() {
                                if self.terminals[self.active_terminal][0].pty_writer.is_none() {
                                    let shell = self.get_active_shell_path();
                                    self.terminals[self.active_terminal][0].start(&shell, ctx.clone());
                                }
                            }
                        }
                        ui.close_menu();
                    }
                });
            });
        });

        // ── PTY Terminal Panel ────────────────────────────────────────────────────
        if self.terminal_open {
            // Auto-start PTY for all splits in active tab if not yet started
            {
                let shell = self.get_active_shell_path();
                let at = self.active_terminal;
                if let Some(splits) = self.terminals.get_mut(at) {
                    for term in splits.iter_mut() {
                        if term.pty_writer.is_none() {
                            term.start(&shell, ctx.clone());
                        }
                    }
                }
            }

            egui::TopBottomPanel::bottom("terminal_panel")
                .resizable(false)
                .height_range(self.terminal_height..=self.terminal_height)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // ── Title bar ────────────────────────────────────────
                        ui.horizontal(|ui| {
                            ui.strong("💻 Terminals: ");

                            let mut tab_to_close = None;
                            for i in 0..self.terminals.len() {
                                if self.terminals[i].is_empty() { continue; }
                                let dir_name = self.terminals[i][0].working_dir.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "workspaces".to_string());
                                let tab_label = format!("{}. {}", i + 1, dir_name);

                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    if ui.selectable_label(self.active_terminal == i, &tab_label).clicked() {
                                        self.active_terminal = i;
                                        self.terminal_focus_requested = false;
                                    }
                                    if self.terminals.len() > 1 {
                                        if ui.small_button("x").clicked() {
                                            tab_to_close = Some(i);
                                        }
                                    }
                                });
                            }

                            if ui.button("+ New Tab").clicked() {
                                let current_dir = if !self.terminals[self.active_terminal].is_empty() {
                                    self.terminals[self.active_terminal][0].working_dir.clone()
                                } else {
                                    std::path::PathBuf::from(&self.folder_path)
                                };
                                let mut new_term = terminal::TerminalInstance::new(current_dir);
                                let shell = self.get_active_shell_path();
                                new_term.start(&shell, ctx.clone());
                                self.terminals.push(vec![new_term]);
                                self.active_terminal = self.terminals.len() - 1;
                                self.terminal_focus_requested = false;
                            }

                            if self.terminals[self.active_terminal].len() < 3 {
                                if ui.button("Split Right").clicked() {
                                    let current_dir = if !self.terminals[self.active_terminal].is_empty() {
                                        self.terminals[self.active_terminal][0].working_dir.clone()
                                    } else {
                                        std::path::PathBuf::from(&self.folder_path)
                                    };
                                    let mut new_term = terminal::TerminalInstance::new(current_dir);
                                    let shell = self.get_active_shell_path();
                                    new_term.start(&shell, ctx.clone());
                                    self.terminals[self.active_terminal].push(new_term);
                                    self.terminal_focus_requested = false;
                                }
                            }

                            if let Some(close_idx) = tab_to_close {
                                self.terminals.remove(close_idx);
                                if self.active_terminal >= self.terminals.len() {
                                    self.active_terminal = self.terminals.len() - 1;
                                }
                                self.terminal_focus_requested = false;
                            }

                            ui.separator();
                            ui.label("Height:");
                            ui.add(egui::Slider::new(&mut self.terminal_height, 120.0..=600.0).show_value(false));

                            if ui.small_button("Min").clicked() {
                                self.terminal_height = 150.0;
                            }
                            if ui.small_button("Max").clicked() {
                                self.terminal_height = 450.0;
                            }

                            ui.separator();
                            ui.label("Shell:");
                            
                            let mut shell_changed = false;
                            egui::ComboBox::from_id_source("shell_selector")
                                .selected_text(if self.show_custom_shell { "Custom" } else { &self.selected_shell })
                                .show_ui(ui, |ui| {
                                    #[cfg(target_os = "windows")]
                                    {
                                        if ui.selectable_value(&mut self.selected_shell, "cmd.exe".to_string(), "cmd.exe").clicked() {
                                            self.show_custom_shell = false;
                                            shell_changed = true;
                                        }
                                        if ui.selectable_value(&mut self.selected_shell, "powershell.exe".to_string(), "powershell.exe").clicked() {
                                            self.show_custom_shell = false;
                                            shell_changed = true;
                                        }
                                    }
                                    #[cfg(not(target_os = "windows"))]
                                    {
                                        if ui.selectable_value(&mut self.selected_shell, "/bin/bash".to_string(), "bash (/bin/bash)").clicked() {
                                            self.show_custom_shell = false;
                                            shell_changed = true;
                                        }
                                        if ui.selectable_value(&mut self.selected_shell, "/bin/zsh".to_string(), "zsh (/bin/zsh)").clicked() {
                                            self.show_custom_shell = false;
                                            shell_changed = true;
                                        }
                                        if ui.selectable_value(&mut self.selected_shell, "/bin/sh".to_string(), "sh (/bin/sh)").clicked() {
                                            self.show_custom_shell = false;
                                            shell_changed = true;
                                        }
                                        if ui.selectable_value(&mut self.selected_shell, "/bin/fish".to_string(), "fish (/bin/fish)").clicked() {
                                            self.show_custom_shell = false;
                                            shell_changed = true;
                                        }
                                    }
                                    if ui.selectable_label(self.show_custom_shell, "Custom...").clicked() {
                                        self.show_custom_shell = true;
                                        shell_changed = true;
                                    }
                                });

                            if self.show_custom_shell {
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut self.custom_shell)
                                        .hint_text("Shell path (e.g. /bin/zsh)")
                                        .desired_width(120.0)
                                );
                                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    shell_changed = true;
                                }
                            }

                            if shell_changed {
                                let shell = self.get_active_shell_path();
                                for term in &mut self.terminals[self.active_terminal] {
                                    term.start(&shell, ctx.clone());
                                }
                                self.terminal_focus_requested = false;
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Clear").clicked() {
                                    let at = self.active_terminal;
                                    for term in &mut self.terminals[at] {
                                        term.write_str("clear\n");
                                    }
                                }
                                if ui.button("Restart").clicked() {
                                    let shell = self.get_active_shell_path();
                                    for term in &mut self.terminals[self.active_terminal] {
                                        term.start(&shell, ctx.clone());
                                    }
                                    self.terminal_focus_requested = false;
                                }
                                if ui.button("×").clicked() {
                                    self.terminal_open = false;
                                }
                            });
                        });
                        ui.separator();

                        // ── PTY Cell Grid Canvas ──────────────────────────────
                        let mut split_to_close: Option<usize> = None;

                        let canvas_rect = ui.available_rect_before_wrap();
                        ui.painter().rect_filled(canvas_rect, 4.0, egui::Color32::from_rgb(10, 10, 10));

                        let num_splits = self.terminals[self.active_terminal].len();
                        let spacing = 6.0;
                        let split_width = (canvas_rect.width() - spacing * (num_splits - 1) as f32) / num_splits as f32;

                        // Capture keyboard events for the focused split
                        let focused_s = self.terminal_focused_split.unwrap_or(0);
                        let at = self.active_terminal;

                        // Use an invisible sense to track if the terminal canvas area has focus
                        let canvas_id = ui.id().with("terminal_canvas_focus");
                        let canvas_sense = ui.interact(canvas_rect, canvas_id, egui::Sense::click());
                        if canvas_sense.clicked() {
                            ui.ctx().memory_mut(|m| m.request_focus(canvas_id));
                        }

                        let canvas_has_focus = ui.ctx().memory(|m| m.has_focus(canvas_id));

                        // Collect key events to forward to PTY — only when canvas has focus
                        if canvas_has_focus {
                            let raw_input: Option<Vec<u8>> = ui.input(|i| {
                                let mut bytes: Vec<u8> = Vec::new();

                                for event in &i.events {
                                    match event {
                                        egui::Event::Text(s) => {
                                            bytes.extend_from_slice(s.as_bytes());
                                        }
                                        egui::Event::Key { key, pressed: true, modifiers, .. } => {
                                            use egui::Key::*;
                                            match key {
                                                Enter     => bytes.push(b'\r'),
                                                Backspace => bytes.push(0x7f),
                                                Tab       => bytes.push(b'\t'),
                                                Escape    => bytes.push(0x1b),
                                                ArrowUp   => bytes.extend_from_slice(b"\x1b[A"),
                                                ArrowDown => bytes.extend_from_slice(b"\x1b[B"),
                                                ArrowRight => bytes.extend_from_slice(b"\x1b[C"),
                                                ArrowLeft  => bytes.extend_from_slice(b"\x1b[D"),
                                                Home  => bytes.extend_from_slice(b"\x1b[H"),
                                                End   => bytes.extend_from_slice(b"\x1b[F"),
                                                Delete => bytes.extend_from_slice(b"\x1b[3~"),
                                                PageUp   => bytes.extend_from_slice(b"\x1b[5~"),
                                                PageDown => bytes.extend_from_slice(b"\x1b[6~"),
                                                A if modifiers.ctrl => bytes.push(0x01),
                                                C if modifiers.ctrl => bytes.push(0x03),
                                                D if modifiers.ctrl => bytes.push(0x04),
                                                E if modifiers.ctrl => bytes.push(0x05),
                                                K if modifiers.ctrl => bytes.push(0x0b),
                                                L if modifiers.ctrl => bytes.push(0x0c),
                                                U if modifiers.ctrl => bytes.push(0x15),
                                                W if modifiers.ctrl => bytes.push(0x17),
                                                Z if modifiers.ctrl => bytes.push(0x1a),
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                if bytes.is_empty() { None } else { Some(bytes) }
                            });

                            if let Some(bytes) = raw_input {
                                let num_s = self.terminals.get(at).map(|s| s.len()).unwrap_or(0);
                                if num_s > 0 {
                                    let split_idx = focused_s.min(num_s - 1);
                                    if let Some(splits) = self.terminals.get_mut(at) {
                                        if let Some(term) = splits.get_mut(split_idx) {
                                            term.write_input(&bytes);
                                        }
                                    }
                                }
                            }
                        }

                        // ── Render each split pane ────────────────────────────
                        for s_idx in 0..num_splits {
                            let left = canvas_rect.min.x + s_idx as f32 * (split_width + spacing);
                            let split_rect = egui::Rect::from_min_size(
                                egui::pos2(left, canvas_rect.min.y),
                                egui::vec2(split_width, canvas_rect.height()),
                            );

                            // Vertical divider
                            if s_idx > 0 {
                                let dx = left - spacing / 2.0;
                                ui.painter().line_segment(
                                    [egui::pos2(dx, canvas_rect.min.y), egui::pos2(dx, canvas_rect.max.y)],
                                    egui::Stroke::new(1.0, egui::Color32::from_gray(40)),
                                );
                            }

                            // Click on split → mark it focused + give canvas keyboard focus
                            let click_sense = ui.interact(split_rect, ui.id().with(("pty_split", s_idx)), egui::Sense::click());
                            if click_sense.clicked() {
                                self.terminal_focused_split = Some(s_idx);
                                ui.ctx().memory_mut(|m| m.request_focus(canvas_id));
                            }

                            let is_focused = self.terminal_focused_split.unwrap_or(0) == s_idx;

                            let mut child_ui = ui.child_ui(split_rect, egui::Layout::top_down(egui::Align::Min), None);
                            let ui = &mut child_ui;

                            // Mini header
                            let folder_name = self.terminals[at][s_idx].working_dir.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "terminal".to_string());

                            let header_color = if is_focused {
                                egui::Color32::from_rgb(0, 200, 120)
                            } else {
                                egui::Color32::from_rgb(120, 120, 120)
                            };

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("💻 {}", folder_name))
                                        .small()
                                        .color(header_color),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if num_splits > 1 && ui.small_button("×").clicked() {
                                        split_to_close = Some(s_idx);
                                    }
                                });
                            });

                            ui.painter().line_segment(
                                [egui::pos2(split_rect.min.x, ui.cursor().min.y),
                                 egui::pos2(split_rect.max.x, ui.cursor().min.y)],
                                egui::Stroke::new(1.0, egui::Color32::from_gray(35)),
                            );

                            // ── Cell grid render ───────────────────────────────
                            let font_id = egui::FontId::monospace(13.0);
                            let char_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
                            let line_height = 15.0;

                            let state_lock = self.terminals[at][s_idx].state.lock();
                            if let Ok(st) = state_lock {
                                let all_lines = st.performer.grid.all_lines();
                                let cursor_row_abs = st.performer.grid.scrollback.len() + st.performer.grid.cursor_row;
                                let cursor_col = st.performer.grid.cursor_col;

                                let available_h = ui.available_height();

                                egui::ScrollArea::vertical()
                                    .id_source(format!("pty_scroll_{}_{}", at, s_idx))
                                    .stick_to_bottom(true)
                                    .max_height(available_h)
                                    .show(ui, |ui| {
                                        for (row_idx, row) in all_lines.iter().enumerate() {
                                            let row_top = ui.cursor().min.y;

                                            // Draw background spans for colored bg cells
                                            for (col_idx, cell) in row.iter().enumerate() {
                                                if cell.bg != egui::Color32::TRANSPARENT {
                                                    let x = ui.cursor().min.x + col_idx as f32 * char_width;
                                                    let bg_rect = egui::Rect::from_min_size(
                                                        egui::pos2(x, row_top),
                                                        egui::vec2(char_width, line_height),
                                                    );
                                                    ui.painter().rect_filled(bg_rect, 0.0, cell.bg);
                                                }
                                            }

                                            let mut text_buf = String::new();

                                            for cell in row.iter() {
                                                text_buf.push(if cell.ch == '\0' { ' ' } else { cell.ch });
                                            }
                                            // Trim trailing spaces for rendering
                                            let trimmed = text_buf.trim_end_matches(' ');

                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 0.0;
                                                ui.label(
                                                    egui::RichText::new(trimmed)
                                                        .font(font_id.clone())
                                                        .color(egui::Color32::from_rgb(220, 220, 220))
                                                );
                                            });

                                            // Draw cursor block
                                            if is_focused && row_idx == cursor_row_abs {
                                                let cx = ui.min_rect().min.x + cursor_col as f32 * char_width;
                                                let cursor_rect = egui::Rect::from_min_size(
                                                    egui::pos2(cx, row_top),
                                                    egui::vec2(char_width.max(2.0), line_height),
                                                );
                                                let blink = (ctx.input(|i| i.time) * 2.0).floor() as i64 % 2 == 0;
                                                let cursor_color = if blink {
                                                    egui::Color32::from_rgba_unmultiplied(200, 200, 200, 180)
                                                } else {
                                                    egui::Color32::TRANSPARENT
                                                };
                                                ui.painter().rect_filled(cursor_rect, 0.0, cursor_color);
                                                ctx.request_repaint();
                                            }
                                        }
                                    });  // ScrollArea
                            }
                        }

                        if let Some(close_idx) = split_to_close {
                            self.terminals[self.active_terminal].remove(close_idx);
                            if let Some(f) = self.terminal_focused_split {
                                if f >= self.terminals[self.active_terminal].len() {
                                    self.terminal_focused_split = Some(
                                        self.terminals[self.active_terminal].len().saturating_sub(1)
                                    );
                                }
                            }
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
                        self.show_create_dialog(ctx);
                    }
                    ui.separator();
                    let terminal_btn_text = if self.terminal_open { "Hide Terminal" } else { "Show Terminal" };
                    if ui.selectable_label(self.terminal_open, format!("💻 {}", terminal_btn_text)).clicked() {
                        self.terminal_open = !self.terminal_open;
                        if self.terminal_open {
                            self.terminal_focus_requested = false;
                            if !self.terminals.is_empty() && !self.terminals[self.active_terminal].is_empty() {
                                if self.terminals[self.active_terminal][0].pty_writer.is_none() {
                                    let shell = self.get_active_shell_path();
                                    self.terminals[self.active_terminal][0].start(&shell, ctx.clone());
                                }
                            }
                        }
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
            PendingAction::OpenDefault(idx) => self.open_workspace(idx, "default", ctx),
            PendingAction::OpenFileManager(idx) => self.open_workspace(idx, "file_manager", ctx),
            PendingAction::OpenEditor(idx) => self.show_editor_select(idx, ctx),
            PendingAction::OpenTerminal(idx) => {
                if let Some(ws) = self.workspaces.get(idx) {
                    let folder_path = ws.config.folders.first()
                        .map(|f| std::path::PathBuf::from(&f.path))
                        .unwrap_or_else(|| ws.path.parent().unwrap_or(&ws.path).to_path_buf());
                    self.open_terminal_for_path(folder_path, ctx);
                }
            }
            PendingAction::Edit(idx) => self.show_edit_dialog(idx, ctx),
            PendingAction::Duplicate(idx) => self.duplicate_workspace(idx, ctx),
            PendingAction::Delete(idx) => self.show_delete_dialog(idx, ctx),
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
                                ui.text_edit_singleline(&mut self.new_name)
                                    .request_focus();
                            });

                            ui.add_space(10.0);
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                self.render_form(ui);
                            });

                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.button("Create").clicked() {
                                    self.create_workspace(ctx);
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
                                        self.update_workspace(idx, ctx);
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
                                ui.label(format!(
                                    "Are you sure you want to delete '{}'?",
                                    name
                                ));
                                ui.label(format!("Path: {}", path));

                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Delete").clicked() {
                                        self.delete_workspace(idx, ctx);
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
                                    .selectable_value(&mut self.selected_editor, editor.to_string(), editor)
                                    .clicked()
                                {
                                    self.open_workspace(idx, "editor", ctx);
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
                                        self.open_workspace(idx, "editor", ctx);
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

fn autocomplete_path(input: &mut String, working_dir: &std::path::Path) {
    if let Some(last_word_idx) = input.rfind(|c: char| c.is_whitespace()) {
        let (prefix, last_part) = input.split_at(last_word_idx + 1);
        if let Some(completed) = match_path_completion(last_part, working_dir) {
            *input = format!("{}{}", prefix, completed);
        }
    } else {
        if let Some(completed) = match_path_completion(input, working_dir) {
            *input = completed;
        }
    }
}

fn match_path_completion(part: &str, working_dir: &std::path::Path) -> Option<String> {
    if part.is_empty() {
        return None;
    }
    
    let mut search_dir = working_dir.to_path_buf();
    let mut search_prefix = part;
    
    if let Some(last_slash) = part.rfind('/') {
        let (dir_part, file_part) = part.split_at(last_slash + 1);
        search_dir = search_dir.join(dir_part);
        search_prefix = file_part;
    }
    
    if let Ok(entries) = std::fs::read_dir(search_dir) {
        let mut matches = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(search_prefix) {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let suffix = if is_dir { "/" } else { " " };
                matches.push(format!("{}{}", name, suffix));
            }
        }
        
        if matches.len() == 1 {
            if let Some(last_slash) = part.rfind('/') {
                let dir_part = &part[..=last_slash];
                return Some(format!("{}{}", dir_part, matches[0]));
            } else {
                return Some(matches[0].clone());
            }
        }
    }
    None
}
