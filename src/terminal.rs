use parking_lot::Mutex;
use portable_pty::{Child, CommandBuilder, NativePtySystem, PtySize, PtySystem, MasterPty};
use std::io::{Read, Write};
use std::sync::Arc;
use vte::{Params, Perform};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TerminalTheme {
    pub background: egui::Color32,
    pub foreground: egui::Color32,
    pub cursor: egui::Color32,
    pub cursor_text: egui::Color32,
    pub selection: egui::Color32,
    pub dim_foreground: egui::Color32,
    pub bright_foreground: egui::Color32,
    pub colors: [egui::Color32; 16],
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self {
            background: egui::Color32::from_rgb(30, 30, 46),
            foreground: egui::Color32::from_rgb(205, 214, 244),
            cursor: egui::Color32::from_rgb(245, 194, 23),
            cursor_text: egui::Color32::from_rgb(30, 30, 46),
            selection: egui::Color32::from_rgba_unmultiplied(137, 180, 250, 64),
            dim_foreground: egui::Color32::from_rgb(166, 173, 200),
            bright_foreground: egui::Color32::from_rgb(205, 214, 244),
            colors: [
                egui::Color32::from_rgb(69, 71, 90),
                egui::Color32::from_rgb(243, 139, 168),
                egui::Color32::from_rgb(166, 227, 161),
                egui::Color32::from_rgb(249, 226, 175),
                egui::Color32::from_rgb(137, 180, 250),
                egui::Color32::from_rgb(203, 166, 247),
                egui::Color32::from_rgb(148, 226, 213),
                egui::Color32::from_rgb(186, 194, 222),
                egui::Color32::from_rgb(88, 91, 112),
                egui::Color32::from_rgb(243, 139, 168),
                egui::Color32::from_rgb(166, 227, 161),
                egui::Color32::from_rgb(249, 226, 175),
                egui::Color32::from_rgb(137, 180, 250),
                egui::Color32::from_rgb(203, 166, 247),
                egui::Color32::from_rgb(148, 226, 213),
                egui::Color32::from_rgb(186, 194, 222),
            ],
        }
    }
}

impl TerminalTheme {
    pub fn catppuccin_mocha() -> Self {
        Self::default()
    }

    pub fn ubuntu() -> Self {
        Self {
            background: egui::Color32::from_rgb(48, 10, 36),
            foreground: egui::Color32::from_rgb(255, 255, 255),
            cursor: egui::Color32::from_rgb(255, 255, 255),
            cursor_text: egui::Color32::from_rgb(48, 10, 36),
            selection: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 64),
            dim_foreground: egui::Color32::from_rgb(200, 200, 200),
            bright_foreground: egui::Color32::from_rgb(255, 255, 255),
            colors: [
                egui::Color32::from_rgb(0, 0, 0),
                egui::Color32::from_rgb(204, 0, 0),
                egui::Color32::from_rgb(78, 154, 6),
                egui::Color32::from_rgb(196, 160, 0),
                egui::Color32::from_rgb(52, 101, 164),
                egui::Color32::from_rgb(117, 80, 123),
                egui::Color32::from_rgb(6, 152, 154),
                egui::Color32::from_rgb(211, 215, 207),
                egui::Color32::from_rgb(85, 87, 83),
                egui::Color32::from_rgb(239, 41, 41),
                egui::Color32::from_rgb(138, 226, 52),
                egui::Color32::from_rgb(252, 233, 79),
                egui::Color32::from_rgb(114, 159, 207),
                egui::Color32::from_rgb(173, 127, 168),
                egui::Color32::from_rgb(52, 226, 226),
                egui::Color32::from_rgb(238, 238, 236),
            ],
        }
    }

    pub fn wsl() -> Self {
        Self {
            background: egui::Color32::from_rgb(12, 12, 12),
            foreground: egui::Color32::from_rgb(204, 204, 204),
            cursor: egui::Color32::from_rgb(204, 204, 204),
            cursor_text: egui::Color32::from_rgb(12, 12, 12),
            selection: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 48),
            dim_foreground: egui::Color32::from_rgb(160, 160, 160),
            bright_foreground: egui::Color32::from_rgb(255, 255, 255),
            colors: [
                egui::Color32::from_rgb(0, 0, 0),
                egui::Color32::from_rgb(197, 15, 31),
                egui::Color32::from_rgb(19, 161, 14),
                egui::Color32::from_rgb(193, 156, 0),
                egui::Color32::from_rgb(0, 55, 218),
                egui::Color32::from_rgb(136, 23, 152),
                egui::Color32::from_rgb(58, 150, 221),
                egui::Color32::from_rgb(204, 204, 204),
                egui::Color32::from_rgb(118, 118, 118),
                egui::Color32::from_rgb(231, 72, 86),
                egui::Color32::from_rgb(22, 198, 12),
                egui::Color32::from_rgb(249, 241, 165),
                egui::Color32::from_rgb(59, 120, 255),
                egui::Color32::from_rgb(180, 0, 158),
                egui::Color32::from_rgb(97, 214, 214),
                egui::Color32::from_rgb(242, 242, 242),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalFontSettings {
    pub family: String,
    pub size: f32,
}

impl Default for TerminalFontSettings {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono".to_string(),
            size: 14.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalSettings {
    pub theme: TerminalTheme,
    pub font: TerminalFontSettings,
    pub shell: String,
    pub scrollback_lines: usize,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            theme: TerminalTheme::catppuccin_mocha(),
            font: TerminalFontSettings::default(),
            shell: String::new(),
            scrollback_lines: 10000,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Cell {
    pub ch: String,
    pub fg: egui::Color32,
    pub bg: egui::Color32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub inverse: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: " ".to_string(),
            fg: egui::Color32::TRANSPARENT,
            bg: egui::Color32::TRANSPARENT,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            inverse: false,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TerminalState {
    pub grid: Vec<Vec<Cell>>,
    pub scrollback: Vec<Vec<Cell>>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub cols: usize,
    pub rows: usize,
    pub scroll_offset: i32,
    pub cursor_visible: bool,
    pub ctx: Option<egui::Context>,
}

struct VteHandler {
    state: Arc<Mutex<TerminalState>>,
    theme: TerminalTheme,
    current_fg: egui::Color32,
    current_bg: egui::Color32,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
    inverse: bool,
    cursor_x: usize,
    cursor_y: usize,
    cols: usize,
    rows: usize,
    scroll_top: usize,
    scroll_bottom: usize,
    pending_char: Option<char>,
}

impl VteHandler {
    fn new(state: Arc<Mutex<TerminalState>>, theme: TerminalTheme, cols: usize, rows: usize) -> Self {
        let current_fg = theme.foreground;
        let current_bg = theme.background;
        Self {
            state,
            theme,
            current_fg,
            current_bg,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            inverse: false,
            cursor_x: 0,
            cursor_y: 0,
            cols,
            rows,
            scroll_top: 0,
            scroll_bottom: rows,
            pending_char: None,
        }
    }

    fn sync_dimensions(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.scroll_top = self.scroll_top.min(self.rows);
        self.scroll_bottom = self.scroll_bottom.clamp(self.scroll_top, self.rows);
        self.cursor_x = self.cursor_x.min(self.cols);
        self.cursor_y = self.cursor_y.min(self.rows.saturating_sub(1));
    }

    fn advance_cursor(&mut self) {
        if self.cursor_x < self.cols {
            self.cursor_x += 1;
        }
    }

    fn scroll_up(&mut self) {
        let state_arc = self.state.clone();
        let mut state = state_arc.lock();
        self.sync_dimensions(state.cols, state.rows);
        if self.scroll_top >= self.scroll_bottom || self.scroll_bottom > state.grid.len() {
            return;
        }
        let scroll_region = state.grid[self.scroll_top..self.scroll_bottom].to_vec();
        state.scrollback.push(scroll_region[0].clone());
        if state.scrollback.len() > 10000 {
            state.scrollback.remove(0);
        }
        for i in self.scroll_top..self.scroll_bottom - 1 {
            state.grid[i] = state.grid[i + 1].clone();
        }
        state.grid[self.scroll_bottom - 1] = self.make_empty_row();
    }

    fn make_empty_row(&self) -> Vec<Cell> {
        let mut row = Vec::with_capacity(self.cols);
        for _ in 0..self.cols {
            let mut cell = Cell::default();
            cell.fg = self.theme.foreground;
            cell.bg = self.theme.background;
            row.push(cell);
        }
        row
    }
fn is_myanmar_combining(ch: char) -> bool {
    let u = ch as u32;
    if u >= 0x1000 && u <= 0x109F {
        matches!(
            u,
            0x102D..=0x1030
                | 0x1032..=0x1037
                | 0x1039..=0x103A
                | 0x103D..=0x103E
                | 0x1058..=0x1059
                | 0x105E..=0x1060
                | 0x1071..=0x1074
                | 0x1082
                | 0x1085..=0x1086
                | 0x108D
                | 0x109D
        )
    } else {
        false
    }
}

    fn put_char(&mut self, ch: char) {
        if self.pending_char.is_some() {
            self.pending_char = None;
        }

        let is_combining = !ch.is_control() && (unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) == 0 || Self::is_myanmar_combining(ch));

        if is_combining {
            let (target_x, target_y) = if self.cursor_x > 0 {
                (self.cursor_x - 1, self.cursor_y)
            } else if self.cursor_y > 0 {
                (self.cols - 1, self.cursor_y - 1)
            } else {
                (0, 0)
            };

            let state_arc = self.state.clone();
            let mut state = state_arc.lock();
            self.sync_dimensions(state.cols, state.rows);
            if target_y < state.grid.len() && target_x < state.grid[target_y].len() {
                let cell_ch = &mut state.grid[target_y][target_x].ch;
                if cell_ch == " " {
                    cell_ch.clear();
                }
                cell_ch.push(ch);
            }
            return;
        }

        if self.cursor_x >= self.cols {
            self.cursor_x = 0;
            self.cursor_y += 1;
            if self.cursor_y >= self.rows {
                self.scroll_up();
                self.cursor_y = self.rows.saturating_sub(1);
            }
        }

        {
            let state_arc = self.state.clone();
            let mut state = state_arc.lock();
            self.sync_dimensions(state.cols, state.rows);
            if self.cursor_y < state.grid.len() && self.cursor_x < state.grid[self.cursor_y].len() {
                state.grid[self.cursor_y][self.cursor_x] = Cell {
                    ch: ch.to_string(),
                    fg: self.current_fg,
                    bg: self.current_bg,
                    bold: self.bold,
                    italic: self.italic,
                    underline: self.underline,
                    dim: self.dim,
                    inverse: self.inverse,
                };
            }
        }
        self.advance_cursor();
    }

    fn handle_print(&mut self, ch: char) {
        if ch == '\n' || ch == '\r' {
            if ch == '\r' {
                self.cursor_x = 0;
            }
            if ch == '\n' {
                self.cursor_y += 1;
                if self.cursor_y >= self.rows {
                    self.scroll_up();
                    self.cursor_y = self.rows - 1;
                }
            }
            return;
        }

        if ch == '\t' {
            let next_tab = (self.cursor_x / 8 + 1) * 8;
            self.cursor_x = next_tab.min(self.cols - 1);
            return;
        }

        if ch == '\x08' || ch == '\x7f' {
            if self.cursor_x > 0 {
                self.cursor_x -= 1;
            }
            return;
        }

        self.put_char(ch);
    }

    fn handle_csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        match action {
            'm' => self.select_graphic_rendition(params),
            'H' | 'f' => self.cursor_position(params),
            'A' => self.cursor_up(params),
            'B' => self.cursor_down(params),
            'C' => self.cursor_forward(params),
            'D' => self.cursor_back(params),
            'E' => self.cursor_next_line(params),
            'F' => self.cursor_prev_line(params),
            'G' => self.cursor_character_absolute(params),
            'J' => self.erase_in_display(params),
            'K' => self.erase_in_line(params),
            'L' => self.insert_lines(params),
            'M' => self.delete_lines(params),
            'P' => self.delete_characters(params),
            'X' => self.erase_characters(params),
            'S' => self.scroll_up_csi(params),
            'T' => self.scroll_down_csi(params),
            'r' => self.set_scrolling_region(params),
            'h' | 'l' => {}
            'n' => {}
            's' => {}
            'u' => {}
            _ => {}
        }
    }

    fn select_graphic_rendition(&mut self, params: &Params) {
        if params.is_empty() {
            self.reset_attributes();
            return;
        }

        for param in params.iter() {
            let code: u16 = param.first().copied().unwrap_or(0);
            match code {
                0 => self.reset_attributes(),
                1 => self.bold = true,
                2 => self.dim = true,
                3 => self.italic = true,
                4 => self.underline = true,
                7 => self.inverse = true,
                22 => { self.bold = false; self.dim = false; }
                23 => self.italic = false,
                24 => self.underline = false,
                27 => self.inverse = false,
                30..=37 => self.current_fg = self.theme.colors[(code.saturating_sub(30)) as usize % 16],
                38 => {
                    if let Some(color) = self.get_color_param(params) {
                        self.current_fg = color;
                    }
                }
                39 => self.current_fg = self.theme.foreground,
                40..=47 => self.current_bg = self.theme.colors[(code.saturating_sub(40)) as usize % 16],
                48 => {
                    if let Some(color) = self.get_color_param(params) {
                        self.current_bg = color;
                    }
                }
                49 => self.current_bg = self.theme.background,
                90..=97 => self.current_fg = self.theme.colors[(code.saturating_sub(90) + 8) as usize % 16],
                100..=107 => self.current_bg = self.theme.colors[(code.saturating_sub(100) + 8) as usize % 16],
                _ => {}
            }
        }
    }

    fn get_color_param(&self, params: &Params) -> Option<egui::Color32> {
        let mut iter = params.iter();
        if let Some(p) = iter.next() {
            let sub: u16 = p.first().copied().unwrap_or(0);
            if sub == 5 {
                if let Some(p2) = iter.next() {
                    let idx: u16 = p2.first().copied().unwrap_or(0);
                    if idx < 16 {
                        return Some(self.theme.colors[idx as usize]);
                    } else if idx < 232 {
                        let idx_u32: u32 = idx as u32;
                        let r: u8 = (((idx_u32.saturating_sub(16)) / 36) * 51) as u8;
                        let g: u8 = ((((idx_u32.saturating_sub(16)) % 36) / 6) * 51) as u8;
                        let b: u8 = (((idx_u32.saturating_sub(16)) % 6) * 51) as u8;
                        return Some(egui::Color32::from_rgb(r, g, b));
                    } else {
                        let v: u8 = (8 + idx.saturating_sub(232) * 10) as u8;
                        return Some(egui::Color32::from_gray(v));
                    }
                }
            } else if sub == 2 {
                let vals: Vec<u8> = params.iter().skip(1).filter_map(|p| {
                    let v: u16 = p.first().copied().unwrap_or(0);
                    Some(v.min(255) as u8)
                }).collect();
                if vals.len() >= 3 {
                    return Some(egui::Color32::from_rgb(vals[0], vals[1], vals[2]));
                }
            }
        }
        None
    }

    fn reset_attributes(&mut self) {
        self.current_fg = self.theme.foreground;
        self.current_bg = self.theme.background;
        self.bold = false;
        self.italic = false;
        self.underline = false;
        self.dim = false;
        self.inverse = false;
    }

    fn cursor_position(&mut self, params: &Params) {
        let mut row: usize = 1;
        let mut col: usize = 1;
        let mut iter = params.iter();
        if let Some(p) = iter.next() {
            if let Some(&v) = p.first() {
                row = v as usize;
            }
        }
        if let Some(p) = iter.next() {
            if let Some(&v) = p.first() {
                col = v as usize;
            }
        }
        self.cursor_y = row.saturating_sub(1).min(self.rows.saturating_sub(1));
        self.cursor_x = col.saturating_sub(1).min(self.cols.saturating_sub(1));
    }

    fn cursor_up(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        self.cursor_y = self.cursor_y.saturating_sub(n);
    }

    fn cursor_down(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        self.cursor_y = (self.cursor_y + n).min(self.rows.saturating_sub(1));
    }

    fn cursor_forward(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        self.cursor_x = (self.cursor_x + n).min(self.cols.saturating_sub(1));
    }

    fn cursor_back(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        self.cursor_x = self.cursor_x.saturating_sub(n);
    }

    fn cursor_next_line(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        self.cursor_y = (self.cursor_y + n).min(self.rows.saturating_sub(1));
        self.cursor_x = 0;
    }

    fn cursor_prev_line(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        self.cursor_y = self.cursor_y.saturating_sub(n);
        self.cursor_x = 0;
    }

    fn cursor_character_absolute(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        self.cursor_x = n.saturating_sub(1).min(self.cols.saturating_sub(1));
    }

    fn erase_in_display(&mut self, params: &Params) {
        let mode = Self::param_or_zero(params);
        let state_arc = self.state.clone();
        let mut state = state_arc.lock();
        self.sync_dimensions(state.cols, state.rows);
        match mode {
            0 => {
                for y in self.cursor_y..self.rows {
                    for x in 0..self.cols {
                        if y == self.cursor_y && x < self.cursor_x {
                            continue;
                        }
                        if y < state.grid.len() && x < state.grid[y].len() {
                            state.grid[y][x] = self.make_cell();
                        }
                    }
                }
            }
            1 => {
                for y in 0..=self.cursor_y {
                    for x in 0..self.cols {
                        if y == self.cursor_y && x > self.cursor_x {
                            continue;
                        }
                        if y < state.grid.len() && x < state.grid[y].len() {
                            state.grid[y][x] = self.make_cell();
                        }
                    }
                }
            }
            2 | 3 => {
                for y in 0..self.rows {
                    for x in 0..self.cols {
                        if y < state.grid.len() && x < state.grid[y].len() {
                            state.grid[y][x] = self.make_cell();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn erase_in_line(&mut self, params: &Params) {
        let mode = Self::param_or_zero(params);
        let state_arc = self.state.clone();
        let mut state = state_arc.lock();
        self.sync_dimensions(state.cols, state.rows);
        if self.cursor_y >= state.grid.len() {
            return;
        }
        match mode {
            0 => {
                for x in self.cursor_x..self.cols {
                    if x < state.grid[self.cursor_y].len() {
                        state.grid[self.cursor_y][x] = self.make_cell();
                    }
                }
            }
            1 => {
                for x in 0..=self.cursor_x {
                    if x < state.grid[self.cursor_y].len() {
                        state.grid[self.cursor_y][x] = self.make_cell();
                    }
                }
            }
            2 => {
                for x in 0..self.cols {
                    if x < state.grid[self.cursor_y].len() {
                        state.grid[self.cursor_y][x] = self.make_cell();
                    }
                }
            }
            _ => {}
        }
    }

    fn make_cell(&self) -> Cell {
        let mut cell = Cell::default();
        cell.fg = self.theme.foreground;
        cell.bg = self.theme.background;
        cell
    }

    fn insert_lines(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        let state_arc = self.state.clone();
        let mut state = state_arc.lock();
        self.sync_dimensions(state.cols, state.rows);
        if self.scroll_top >= self.scroll_bottom || self.scroll_bottom > state.grid.len() {
            return;
        }
        for _ in 0..n {
            for i in (self.scroll_top + 1..self.scroll_bottom).rev() {
                state.grid[i] = state.grid[i - 1].clone();
            }
            state.grid[self.scroll_top] = self.make_empty_row();
        }
    }

    fn delete_lines(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        let state_arc = self.state.clone();
        let mut state = state_arc.lock();
        self.sync_dimensions(state.cols, state.rows);
        if self.scroll_top >= self.scroll_bottom || self.scroll_bottom > state.grid.len() {
            return;
        }
        for _ in 0..n {
            for i in self.scroll_top..self.scroll_bottom - 1 {
                state.grid[i] = state.grid[i + 1].clone();
            }
            state.grid[self.scroll_bottom - 1] = self.make_empty_row();
        }
    }

    fn delete_characters(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        let state_arc = self.state.clone();
        let mut state = state_arc.lock();
        self.sync_dimensions(state.cols, state.rows);
        if self.cursor_y < state.grid.len() {
            let row = &mut state.grid[self.cursor_y];
            for i in self.cursor_x..self.cols.saturating_sub(n) {
                if i + n < row.len() {
                    row[i] = row[i + n].clone();
                }
            }
            for i in self.cols.saturating_sub(n)..self.cols.min(row.len()) {
                row[i] = self.make_cell();
            }
        }
    }

    fn erase_characters(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        let state_arc = self.state.clone();
        let mut state = state_arc.lock();
        self.sync_dimensions(state.cols, state.rows);
        if self.cursor_y < state.grid.len() {
            let row = &mut state.grid[self.cursor_y];
            for i in self.cursor_x..(self.cursor_x + n).min(self.cols).min(row.len()) {
                row[i] = self.make_cell();
            }
        }
    }

    fn scroll_up_csi(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        for _ in 0..n {
            self.scroll_up();
        }
    }

    fn scroll_down_csi(&mut self, params: &Params) {
        let n = Self::param_or_one(params);
        let state_arc = self.state.clone();
        let mut state = state_arc.lock();
        self.sync_dimensions(state.cols, state.rows);
        if self.scroll_top >= self.scroll_bottom || self.scroll_bottom > state.grid.len() {
            return;
        }
        for _ in 0..n {
            for i in (self.scroll_top + 1..self.scroll_bottom).rev() {
                state.grid[i] = state.grid[i - 1].clone();
            }
            state.grid[self.scroll_top] = self.make_empty_row();
        }
    }

    fn set_scrolling_region(&mut self, params: &Params) {
        let mut top: usize = 1;
        let mut bottom: usize = self.rows;
        let mut iter = params.iter();
        if let Some(p) = iter.next() {
            if let Some(&v) = p.first() {
                top = v as usize;
            }
        }
        if let Some(p) = iter.next() {
            if let Some(&v) = p.first() {
                bottom = v as usize;
            }
        }
        self.scroll_top = top.saturating_sub(1).min(self.rows.saturating_sub(1));
        self.scroll_bottom = bottom.clamp(self.scroll_top, self.rows);
    }

    fn param_or_zero(params: &Params) -> usize {
        params.iter().next().and_then(|p| p.first().copied()?.try_into().ok()).unwrap_or(0usize)
    }

    fn param_or_one(params: &Params) -> usize {
        params.iter().next().and_then(|p| p.first().copied()?.try_into().ok()).unwrap_or(1usize)
    }

    fn sync_state(&mut self) {
        let mut state = self.state.lock();
        state.cursor_x = self.cursor_x.min(state.cols.saturating_sub(1));
        state.cursor_y = self.cursor_y.min(state.rows.saturating_sub(1));
        if let Some(ctx) = &state.ctx {
            ctx.request_repaint();
        }
    }
}

impl Perform for VteHandler {
    fn print(&mut self, c: char) {
        self.handle_print(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | b'\r' => self.handle_print(byte as char),
            b'\x08' | b'\x7f' => self.handle_print(byte as char),
            b'\t' => self.handle_print(byte as char),
            b'\x07' => {}
            b'\x0e' => {}
            b'\x0f' => {}
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        self.handle_csi_dispatch(params, intermediates, ignore, action);
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

impl Drop for VteHandler {
    fn drop(&mut self) {
        self.sync_state();
    }
}

pub struct Terminal {
    pub cols: usize,
    pub rows: usize,
    pub settings: TerminalSettings,
    pub scroll_offset: i32,
    pub cursor_visible: bool,

    state: Arc<Mutex<TerminalState>>,
    pty_master: Option<Box<dyn MasterPty + Send>>,
    pty_writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    read_thread: Option<std::thread::JoinHandle<()>>,
    running: Arc<Mutex<bool>>,
    _child: Option<Box<dyn Child + Send + Sync>>,
    pub id: usize,
}

static NEXT_TERMINAL_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

impl Terminal {
    pub fn new(
        settings: TerminalSettings,
        working_dir: Option<std::path::PathBuf>,
    ) -> Result<Self, String> {
        let id = NEXT_TERMINAL_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pty_system = NativePtySystem::default();

        let shell = if settings.shell.is_empty() {
            default_shell()
        } else {
            settings.shell.clone()
        };

        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(working_dir.unwrap_or_else(|| dirs::home_dir().unwrap_or_default()));

        // Inherit parent environment variables
        for (key, val) in std::env::vars() {
            cmd.env(key, val);
        }
        cmd.env("TERM", "xterm-256color");
        let lang = std::env::var("LANG").unwrap_or_default();
        let target_lang = if lang.to_lowercase().contains("utf-8") || lang.to_lowercase().contains("utf8") {
            lang
        } else {
            "C.UTF-8".to_string()
        };
        cmd.env("LANG", &target_lang);
        cmd.env("LC_CTYPE", &target_lang);
        cmd.env("LC_ALL", &target_lang);

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to create reader: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to get writer: {}", e))?;

        let cols = 80;
        let rows = 24;
        let theme = settings.theme.clone();

        let mut grid = vec![vec![Cell::default(); cols]; rows];
        for row in &mut grid {
            for cell in row {
                cell.fg = theme.foreground;
                cell.bg = theme.background;
            }
        }

        let state = Arc::new(Mutex::new(TerminalState {
            grid,
            scrollback: Vec::new(),
            cursor_x: 0,
            cursor_y: 0,
            cols,
            rows,
            scroll_offset: 0,
            cursor_visible: true,
            ctx: None,
        }));

        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();
        let state_clone = state.clone();
        let theme_clone = theme.clone();

        let read_thread = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut parser = vte::Parser::new();
            let mut handler = VteHandler::new(state_clone, theme_clone, cols, rows);

            loop {
                if !*running_clone.lock() {
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        for &byte in &buf[..n] {
                            parser.advance(&mut handler, byte);
                        }
                        handler.sync_state();
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            cols,
            rows,
            settings,
            scroll_offset: 0,
            cursor_visible: true,
            state,
            pty_master: Some(pair.master),
            pty_writer: Some(Arc::new(Mutex::new(writer))),
            read_thread: Some(read_thread),
            running,
            _child: Some(child),
            id,
        })
    }

    pub fn set_context(&mut self, ctx: egui::Context) {
        let mut state = self.state.lock();
        state.ctx = Some(ctx);
    }

    pub fn input(&self, bytes: &[u8]) {
        if let Some(writer) = &self.pty_writer {
            let mut w = writer.lock();
            let _ = w.write_all(bytes);
            let _ = w.flush();
        }
    }

    pub fn paste(&self, text: &str) {
        self.input(text.as_bytes());
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        if cols == 0 || rows == 0 || (cols == self.cols && rows == self.rows) {
            return;
        }

        if let Some(master) = &self.pty_master {
            let _ = master.resize(portable_pty::PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            });
        }

        let mut state = self.state.lock();
        let mut new_grid = vec![vec![Cell::default(); cols]; rows];
        let theme = &self.settings.theme;

        for row in &mut new_grid {
            for cell in row {
                cell.fg = theme.foreground;
                cell.bg = theme.background;
            }
        }

        let old_rows = state.grid.len();
        for y in 0..old_rows.min(rows) {
            let old_cols = state.grid[y].len();
            for x in 0..old_cols.min(cols) {
                new_grid[y][x] = state.grid[y][x].clone();
            }
        }

        state.grid = new_grid;
        state.cols = cols;
        state.rows = rows;
        state.cursor_x = state.cursor_x.min(cols.saturating_sub(1));
        state.cursor_y = state.cursor_y.min(rows.saturating_sub(1));

        self.cols = cols;
        self.rows = rows;
    }

    pub fn process_events(&mut self) {}

    pub fn scroll(&mut self, lines: i32) {
        let state = self.state.lock();
        self.scroll_offset = (self.scroll_offset + lines).clamp(
            -(state.scrollback.len() as i32),
            0,
        );
    }

    pub fn get_state(&self) -> TerminalState {
        self.state.lock().clone()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        *self.running.lock() = false;
        self.input(b"\x03");
        if let Some(handle) = self.read_thread.take() {
            let _ = handle.join();
        }
    }
}

pub struct TerminalWidget<'a> {
    terminal: &'a mut Terminal,
}

impl<'a> TerminalWidget<'a> {
    pub fn new(terminal: &'a mut Terminal) -> Self {
        Self { terminal }
    }
}

impl egui::Widget for TerminalWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());

        let font_size = self.terminal.settings.font.size;
        let char_width = font_size * 0.6;
        let line_height = font_size * 1.4;

        let available_width = response.rect.width();
        let available_height = response.rect.height();

        let cols = (available_width / char_width).floor().max(2.0) as usize;
        let rows = (available_height / line_height).floor().max(1.0) as usize;

        if cols != self.terminal.cols || rows != self.terminal.rows {
            self.terminal.resize(cols, rows);
        }

        self.terminal.process_events();

        let state = self.terminal.get_state();
        let theme = &self.terminal.settings.theme;

        painter.rect_filled(response.rect, 0.0, theme.background);

        let scroll_offset = self.terminal.scroll_offset;
        let display_start = if scroll_offset < 0 {
            (-scroll_offset) as usize
        } else {
            0
        };

        // Pass 1: Draw cell backgrounds
        for row in 0..self.terminal.rows {
            let grid_row = row + display_start;
            if grid_row >= state.grid.len() {
                break;
            }
            for col in 0..self.terminal.cols {
                if col >= state.grid[grid_row].len() {
                    break;
                }

                let cell = &state.grid[grid_row][col];
                if cell.bg != theme.background && cell.bg != egui::Color32::TRANSPARENT {
                    let x = response.rect.min.x + col as f32 * char_width;
                    let y = response.rect.min.y + row as f32 * line_height;
                    let cell_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(x, y),
                        egui::Vec2::new(char_width, line_height),
                    );
                    painter.rect_filled(cell_rect, 0.0, cell.bg);
                }
            }
        }

        // Pass 2: Draw text runs (grouping contiguous styled characters)
        for row in 0..self.terminal.rows {
            let grid_row = row + display_start;
            if grid_row >= state.grid.len() {
                break;
            }

            let row_cells = &state.grid[grid_row];
            let cols_limit = row_cells.len().min(self.terminal.cols);

            // Find the last column index that contains a non-empty, non-space character
            let mut last_non_empty_col = None;
            for col in (0..cols_limit).rev() {
                let cell = &row_cells[col];
                if !cell.ch.is_empty() && cell.ch != " " && cell.ch != "\0" && cell.ch != "\x00" {
                    last_non_empty_col = Some(col);
                    break;
                }
            }

            if let Some(limit_col) = last_non_empty_col {
                let mut current_run_start = 0;
                let mut current_run_text = String::new();
                let mut current_fg = row_cells[0].fg;
                let mut current_bold = row_cells[0].bold;
                let mut current_italic = row_cells[0].italic;
                let mut current_dim = row_cells[0].dim;

                for col in 0..=limit_col {
                    let cell = &row_cells[col];
                    let cell_ch = if cell.ch.is_empty() || cell.ch == "\0" || cell.ch == "\x00" {
                        " "
                    } else {
                        &cell.ch
                    };

                    let style_match = cell.fg == current_fg
                        && cell.bold == current_bold
                        && cell.italic == current_italic
                        && cell.dim == current_dim;

                    if style_match {
                        current_run_text.push_str(cell_ch);
                    } else {
                        // Emit the finished run
                        if !current_run_text.is_empty() {
                            let mut text_color = current_fg;
                            if current_dim {
                                text_color = text_color.linear_multiply(0.6);
                            }
                            let x = response.rect.min.x + current_run_start as f32 * char_width;
                            let y = response.rect.min.y + row as f32 * line_height;

                            painter.text(
                                egui::Pos2::new(x, y + line_height * 0.5),
                                egui::Align2::LEFT_CENTER,
                                current_run_text.clone(),
                                egui::FontId::monospace(font_size),
                                text_color,
                            );
                        }

                        // Start new run
                        current_run_start = col;
                        current_run_text = cell_ch.to_string();
                        current_fg = cell.fg;
                        current_bold = cell.bold;
                        current_italic = cell.italic;
                        current_dim = cell.dim;
                    }
                }

                // Emit final run
                if !current_run_text.is_empty() {
                    let mut text_color = current_fg;
                    if current_dim {
                        text_color = text_color.linear_multiply(0.6);
                    }
                    let x = response.rect.min.x + current_run_start as f32 * char_width;
                    let y = response.rect.min.y + row as f32 * line_height;

                    painter.text(
                        egui::Pos2::new(x, y + line_height * 0.5),
                        egui::Align2::LEFT_CENTER,
                        current_run_text,
                        egui::FontId::monospace(font_size),
                        text_color,
                    );
                }
            }
        }

        if self.terminal.cursor_visible {
            let cursor_y = state.cursor_y.saturating_sub(display_start);
            if cursor_y < self.terminal.rows {
                let cursor_x = state.cursor_x;
                if cursor_x < self.terminal.cols {
                    let cursor_x_pos = response.rect.min.x + cursor_x as f32 * char_width;
                    let cursor_y_pos = response.rect.min.y + cursor_y as f32 * line_height;
                    let cursor_rect = egui::Rect::from_min_size(
                        egui::Pos2::new(cursor_x_pos, cursor_y_pos),
                        egui::Vec2::new(char_width, line_height),
                    );

                    let has_focus = response.has_focus();
                    if has_focus {
                        painter.rect_filled(cursor_rect, 1.0, theme.cursor);

                        // Draw character under cursor in cursor_text color
                        let grid_row = cursor_y + display_start;
                        if grid_row < state.grid.len() && cursor_x < state.grid[grid_row].len() {
                            let cell = &state.grid[grid_row][cursor_x];
                            let cell_ch = if cell.ch.is_empty() || cell.ch == "\0" || cell.ch == "\x00" {
                                ""
                            } else {
                                &cell.ch
                            };
                            if !cell_ch.is_empty() {
                                painter.text(
                                    egui::Pos2::new(cursor_x_pos, cursor_y_pos + line_height * 0.5),
                                    egui::Align2::LEFT_CENTER,
                                    cell_ch,
                                    egui::FontId::monospace(font_size),
                                    theme.cursor_text,
                                );
                            }
                        }
                    } else {
                        // Hollow outline cursor when unfocused
                        painter.rect_stroke(cursor_rect, 1.0, egui::Stroke::new(1.0, theme.cursor));
                    }
                }
            }
        }

        if response.clicked() {
            response.request_focus();
        }

        if response.has_focus() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::IMEAllowed(true));
            let cursor_y = state.cursor_y.saturating_sub(display_start);
            let cursor_x_pos = response.rect.min.x + state.cursor_x as f32 * char_width;
            let cursor_y_pos = response.rect.min.y + cursor_y as f32 * line_height;
            let ime_rect = egui::Rect::from_min_size(
                egui::Pos2::new(cursor_x_pos, cursor_y_pos),
                egui::Vec2::new(char_width, line_height),
            );
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::IMERect(ime_rect));

            // Consume Tab and Shift+Tab key events to prevent egui focus navigation from intercepting them
            let tab_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Tab);
            let shift_tab_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::Tab);
            if ui.input_mut(|i| i.consume_shortcut(&tab_shortcut)) {
                handle_key_event(self.terminal, egui::Key::Tab, &egui::Modifiers::NONE);
            } else if ui.input_mut(|i| i.consume_shortcut(&shift_tab_shortcut)) {
                handle_key_event(self.terminal, egui::Key::Tab, &egui::Modifiers::SHIFT);
            }

            ui.input(|i| {
                if !i.events.is_empty() {
                    for event in &i.events {
                        match event {
                            egui::Event::Text(text) => {
                                for ch in text.chars() {
                                    let mut buf = [0u8; 4];
                                    let len = ch.encode_utf8(&mut buf).len();
                                    self.terminal.input(&buf[..len]);
                                }
                            }
                            egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                                for ch in text.chars() {
                                    let mut buf = [0u8; 4];
                                    let len = ch.encode_utf8(&mut buf).len();
                                    self.terminal.input(&buf[..len]);
                                }
                            }
                            egui::Event::Key {
                                key,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                // Skip Tab key since we handle and consume it above
                                if *key != egui::Key::Tab {
                                    handle_key_event(self.terminal, *key, modifiers);
                                }
                            }
                            egui::Event::Paste(text) => {
                                self.terminal.paste(text);
                            }
                            _ => {}
                        }
                    }
                }
            });
        }

        response
    }
}

fn handle_key_event(terminal: &mut Terminal, key: egui::Key, modifiers: &egui::Modifiers) {
    let ctrl = modifiers.ctrl || modifiers.command;
    let shift = modifiers.shift;

    let escape_seq = match (key, ctrl, shift) {
        (egui::Key::Enter, _, _) => b"\r".to_vec(),
        (egui::Key::Backspace, _, _) => b"\x7f".to_vec(),
        (egui::Key::Tab, _, _) => b"\t".to_vec(),
        (egui::Key::Escape, _, _) => b"\x1b".to_vec(),
        (egui::Key::ArrowUp, false, false) => b"\x1b[A".to_vec(),
        (egui::Key::ArrowUp, false, true) => b"\x1b[1;2A".to_vec(),
        (egui::Key::ArrowUp, true, _) => b"\x1b[1;5A".to_vec(),
        (egui::Key::ArrowDown, false, false) => b"\x1b[B".to_vec(),
        (egui::Key::ArrowDown, false, true) => b"\x1b[1;2B".to_vec(),
        (egui::Key::ArrowDown, true, _) => b"\x1b[1;5B".to_vec(),
        (egui::Key::ArrowRight, false, false) => b"\x1b[C".to_vec(),
        (egui::Key::ArrowRight, false, true) => b"\x1b[1;2C".to_vec(),
        (egui::Key::ArrowRight, true, _) => b"\x1b[1;5C".to_vec(),
        (egui::Key::ArrowLeft, false, false) => b"\x1b[D".to_vec(),
        (egui::Key::ArrowLeft, false, true) => b"\x1b[1;2D".to_vec(),
        (egui::Key::ArrowLeft, true, _) => b"\x1b[1;5D".to_vec(),
        (egui::Key::Home, _, _) => b"\x1b[H".to_vec(),
        (egui::Key::End, _, _) => b"\x1b[F".to_vec(),
        (egui::Key::PageUp, false, false) => b"\x1b[5~".to_vec(),
        (egui::Key::PageUp, true, _) => {
            terminal.scroll(10);
            return;
        }
        (egui::Key::PageDown, false, false) => b"\x1b[6~".to_vec(),
        (egui::Key::PageDown, true, _) => {
            terminal.scroll(-10);
            return;
        }
        (egui::Key::Delete, _, _) => b"\x1b[3~".to_vec(),
        (egui::Key::Insert, _, _) => b"\x1b[2~".to_vec(),
        (egui::Key::F1, _, _) => b"\x1bOP".to_vec(),
        (egui::Key::F2, _, _) => b"\x1bOQ".to_vec(),
        (egui::Key::F3, _, _) => b"\x1bOR".to_vec(),
        (egui::Key::F4, _, _) => b"\x1bOS".to_vec(),
        (egui::Key::F5, _, _) => b"\x1b[15~".to_vec(),
        (egui::Key::F6, _, _) => b"\x1b[17~".to_vec(),
        (egui::Key::F7, _, _) => b"\x1b[18~".to_vec(),
        (egui::Key::F8, _, _) => b"\x1b[19~".to_vec(),
        (egui::Key::F9, _, _) => b"\x1b[20~".to_vec(),
        (egui::Key::F10, _, _) => b"\x1b[21~".to_vec(),
        (egui::Key::F11, _, _) => b"\x1b[23~".to_vec(),
        (egui::Key::F12, _, _) => b"\x1b[24~".to_vec(),
        (egui::Key::A, true, _) => b"\x01".to_vec(),
        (egui::Key::B, true, _) => b"\x02".to_vec(),
        (egui::Key::C, true, _) => b"\x03".to_vec(),
        (egui::Key::D, true, _) => b"\x04".to_vec(),
        (egui::Key::E, true, _) => b"\x05".to_vec(),
        (egui::Key::F, true, _) => b"\x06".to_vec(),
        (egui::Key::G, true, _) => b"\x07".to_vec(),
        (egui::Key::H, true, _) => b"\x08".to_vec(),
        (egui::Key::I, true, _) => b"\x09".to_vec(),
        (egui::Key::J, true, _) => b"\x0a".to_vec(),
        (egui::Key::K, true, _) => b"\x0b".to_vec(),
        (egui::Key::L, true, _) => b"\x0c".to_vec(),
        (egui::Key::M, true, _) => b"\x0d".to_vec(),
        (egui::Key::N, true, _) => b"\x0e".to_vec(),
        (egui::Key::O, true, _) => b"\x0f".to_vec(),
        (egui::Key::P, true, _) => b"\x10".to_vec(),
        (egui::Key::Q, true, _) => b"\x11".to_vec(),
        (egui::Key::R, true, _) => b"\x12".to_vec(),
        (egui::Key::S, true, _) => b"\x13".to_vec(),
        (egui::Key::T, true, _) => b"\x14".to_vec(),
        (egui::Key::U, true, _) => b"\x15".to_vec(),
        (egui::Key::V, true, _) => b"\x16".to_vec(),
        (egui::Key::W, true, _) => b"\x17".to_vec(),
        (egui::Key::X, true, _) => b"\x18".to_vec(),
        (egui::Key::Y, true, _) => b"\x19".to_vec(),
        (egui::Key::Z, true, _) => b"\x1a".to_vec(),
        (egui::Key::Space, true, _) => b"\x00".to_vec(),
        _ => return,
    };

    terminal.input(&escape_seq);
}

fn default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
    #[cfg(windows)]
    {
        "powershell".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

pub enum TerminalPane {
    Placeholder,
    Single(Terminal),
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<TerminalPane>,
        second: Box<TerminalPane>,
    },
}

#[allow(dead_code)]
impl TerminalPane {
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        active_id: &mut Option<usize>,
        ctx: &egui::Context,
    ) {
        match self {
            TerminalPane::Placeholder => {}
            TerminalPane::Single(term) => {
                term.set_context(ctx.clone());
                let widget = TerminalWidget::new(term);
                let response = ui.add(widget);
                if response.clicked() || response.has_focus() {
                    *active_id = Some(term.id);
                }
            }
            TerminalPane::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let rect = ui.available_rect_before_wrap();
                let width = rect.width();
                let height = rect.height();

                match direction {
                    SplitDirection::Horizontal => {
                        ui.horizontal(|ui| {
                            let spacing = ui.spacing().item_spacing.x;
                            let available_w = (width - spacing).max(0.0);
                            let w1 = available_w * *ratio;
                            let w2 = available_w * (1.0 - *ratio);

                            ui.allocate_ui_with_layout(
                                egui::vec2(w1, height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    first.render(ui, active_id, ctx);
                                },
                            );
                            ui.allocate_ui_with_layout(
                                egui::vec2(w2, height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    second.render(ui, active_id, ctx);
                                },
                            );
                        });
                    }
                    SplitDirection::Vertical => {
                        ui.vertical(|ui| {
                            let spacing = ui.spacing().item_spacing.y;
                            let available_h = (height - spacing).max(0.0);
                            let h1 = available_h * *ratio;
                            let h2 = available_h * (1.0 - *ratio);

                            ui.allocate_ui_with_layout(
                                egui::vec2(width, h1),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    first.render(ui, active_id, ctx);
                                },
                            );
                            ui.allocate_ui_with_layout(
                                egui::vec2(width, h2),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    second.render(ui, active_id, ctx);
                                },
                            );
                        });
                    }
                }
            }
        }
    }

    pub fn process_events(&mut self) {
        match self {
            TerminalPane::Single(term) => {
                term.process_events();
            }
            TerminalPane::Split { first, second, .. } => {
                first.process_events();
                second.process_events();
            }
            TerminalPane::Placeholder => {}
        }
    }

    pub fn split(&mut self, target_id: usize, direction: SplitDirection, new_term: &mut Option<Terminal>) -> bool {
        let mut should_replace = false;
        match self {
            TerminalPane::Single(term) => {
                if term.id == target_id {
                    should_replace = true;
                }
            }
            TerminalPane::Split { first, second, .. } => {
                if first.split(target_id, direction, new_term) {
                    return true;
                }
                return second.split(target_id, direction, new_term);
            }
            TerminalPane::Placeholder => {}
        }

        if should_replace {
            if let Some(term) = new_term.take() {
                if let TerminalPane::Single(old_term) = std::mem::replace(self, TerminalPane::Placeholder) {
                    *self = TerminalPane::Split {
                        direction,
                        ratio: 0.5,
                        first: Box::new(TerminalPane::Single(old_term)),
                        second: Box::new(TerminalPane::Single(term)),
                    };
                    return true;
                }
            }
        }
        false
    }

    pub fn close_pane(&mut self, target_id: usize, removed: &mut bool) -> Option<TerminalPane> {
        match self {
            TerminalPane::Placeholder => None,
            TerminalPane::Single(t) => {
                if t.id == target_id {
                    *removed = true;
                    Some(TerminalPane::Placeholder)
                } else {
                    None
                }
            }
            TerminalPane::Split { first, second, .. } => {
                let first_res = first.close_pane(target_id, removed);
                if *removed {
                    if let Some(new_first) = first_res {
                        match new_first {
                            TerminalPane::Placeholder => {
                                let second_node = std::mem::replace(&mut **second, TerminalPane::Placeholder);
                                return Some(second_node);
                            }
                            _ => {
                                *first = Box::new(new_first);
                            }
                        }
                    }
                    return None;
                }

                let second_res = second.close_pane(target_id, removed);
                if *removed {
                    if let Some(new_second) = second_res {
                        match new_second {
                            TerminalPane::Placeholder => {
                                let first_node = std::mem::replace(&mut **first, TerminalPane::Placeholder);
                                return Some(first_node);
                            }
                            _ => {
                                *second = Box::new(new_second);
                            }
                        }
                    }
                    return None;
                }

                None
            }
        }
    }

    pub fn any_terminal_id(&self) -> Option<usize> {
        match self {
            TerminalPane::Single(term) => Some(term.id),
            TerminalPane::Split { first, .. } => first.any_terminal_id(),
            _ => None,
        }
    }

    pub fn find_terminal_mut(&mut self, id: usize) -> Option<&mut Terminal> {
        match self {
            TerminalPane::Single(term) => {
                if term.id == id {
                    Some(term)
                } else {
                    None
                }
            }
            TerminalPane::Split { first, second, .. } => {
                if let Some(t) = first.find_terminal_mut(id) {
                    Some(t)
                } else {
                    second.find_terminal_mut(id)
                }
            }
            TerminalPane::Placeholder => None,
        }
    }

    pub fn update_settings(&mut self, settings: &TerminalSettings) {
        match self {
            TerminalPane::Single(term) => {
                term.settings = settings.clone();
            }
            TerminalPane::Split { first, second, .. } => {
                first.update_settings(settings);
                second.update_settings(settings);
            }
            TerminalPane::Placeholder => {}
        }
    }
}

pub struct TerminalTab {
    pub name: String,
    pub root: TerminalPane,
    pub active_terminal_id: Option<usize>,
}

impl TerminalTab {
    pub fn new(
        name: String,
        settings: TerminalSettings,
        working_dir: Option<std::path::PathBuf>,
    ) -> Result<Self, String> {
        let term = Terminal::new(settings, working_dir)?;
        let active_terminal_id = Some(term.id);
        Ok(Self {
            name,
            root: TerminalPane::Single(term),
            active_terminal_id,
        })
    }
}

#[allow(dead_code)]
pub fn render_terminal_panel(
    ui: &mut egui::Ui,
    terminal: Option<&mut Terminal>,
    _ctx: &egui::Context,
) {
    match terminal {
        Some(term) => {
            let widget = TerminalWidget::new(term);
            ui.add(widget);
        }
        None => {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label("No terminal open");
                    ui.add_space(10.0);
                    ui.label("Press Ctrl+` or use View > Terminal to open");
                });
            });
        }
    }
}

pub fn save_terminal_settings(settings: &TerminalSettings) {
    if let Some(config_dir) = dirs::config_dir() {
        let dir = config_dir.join("workspace-manager");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("terminal_settings.json");
        let json = serde_json::json!({
            "theme": "catppuccin_mocha",
            "font_size": settings.font.size,
            "font_family": settings.font.family,
            "shell": settings.shell,
            "scrollback_lines": settings.scrollback_lines,
        });
        let _ = std::fs::write(&file, serde_json::to_string_pretty(&json).unwrap_or_default());
    }
}

pub fn load_terminal_settings() -> TerminalSettings {
    if let Some(config_dir) = dirs::config_dir() {
        let file = config_dir.join("workspace-manager").join("terminal_settings.json");
        if file.exists() {
            if let Ok(content) = std::fs::read_to_string(&file) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let mut settings = TerminalSettings::default();
                    if let Some(size) = json.get("font_size").and_then(|v| v.as_f64()) {
                        settings.font.size = size as f32;
                    }
                    if let Some(family) = json.get("font_family").and_then(|v| v.as_str()) {
                        settings.font.family = family.to_string();
                    }
                    if let Some(shell) = json.get("shell").and_then(|v| v.as_str()) {
                        settings.shell = shell.to_string();
                    }
                    if let Some(lines) = json.get("scrollback_lines").and_then(|v| v.as_u64()) {
                        settings.scrollback_lines = lines as usize;
                    }
                    return settings;
                }
            }
        }
    }
    TerminalSettings::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vte_handler_no_recursion_crash() {
        let state = Arc::new(Mutex::new(TerminalState {
            grid: vec![vec![Cell::default(); 80]; 24],
            scrollback: Vec::new(),
            cursor_x: 0,
            cursor_y: 0,
            cols: 80,
            rows: 24,
            scroll_offset: 0,
            cursor_visible: true,
            ctx: None,
        }));
        let mut handler = VteHandler::new(state, TerminalTheme::default(), 80, 24);
        let mut parser = vte::Parser::new();

        // Feed standard text input
        for byte in b"hello world\r\n" {
            parser.advance(&mut handler, *byte);
        }

        // Feed CSI sequence (cursor position, graphic rendition, etc.)
        for byte in b"\x1b[31mred text\x1b[0m" {
            parser.advance(&mut handler, *byte);
        }
    }

    #[test]
    fn test_myanmar_combining_characters() {
        let state = Arc::new(Mutex::new(TerminalState {
            grid: vec![vec![Cell::default(); 80]; 24],
            scrollback: Vec::new(),
            cursor_x: 0,
            cursor_y: 0,
            cols: 80,
            rows: 24,
            scroll_offset: 0,
            cursor_visible: true,
            ctx: None,
        }));
        let mut handler = VteHandler::new(state.clone(), TerminalTheme::default(), 80, 24);
        let mut parser = vte::Parser::new();

        // Feed "မြေ" -> မ (U+1019) + ြ (U+103C) + ေ (U+1031)
        // In UTF-8:
        // မ = \xe1\x80\x99
        // ြ = \xe1\x80\xbc
        // ေ = \xe1\x80\xb1
        let input_bytes = b"\xe1\x80\x99\xe1\x80\xbc\xe1\x80\xb1";
        for byte in input_bytes {
            parser.advance(&mut handler, *byte);
        }
        handler.sync_state();

        let s = state.lock();
        // The characters are stored in separate cells because they have wcwidth == 1
        assert_eq!(s.grid[0][0].ch, "မ");
        assert_eq!(s.grid[0][1].ch, "ြ");
        assert_eq!(s.grid[0][2].ch, "ေ");
        // And the cursor should have advanced by exactly 3 columns
        assert_eq!(s.cursor_x, 3);
    }

    #[test]
    fn test_zsh_myanmar_echo() {
        let pty_system = NativePtySystem::default();
        let shell = default_shell();
        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("LANG", "C.UTF-8");
        cmd.env("LC_CTYPE", "C.UTF-8");
        cmd.env("LC_ALL", "C.UTF-8");

        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }).unwrap();

        let _child = pair.slave.spawn_command(cmd).unwrap();
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut writer = pair.master.take_writer().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(500));
        let mut initial_buf = [0u8; 8192];
        if let Ok(n) = reader.read(&mut initial_buf) {
            println!("Initial Prompt Output: {:?}", String::from_utf8_lossy(&initial_buf[..n]));
        }

        let input_str = "နေကောင်းပါသလား";
        writer.write_all(input_str.as_bytes()).unwrap();
        writer.flush().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(500));
        let mut response_buf = [0u8; 8192];
        if let Ok(n) = reader.read(&mut response_buf) {
            let response = &response_buf[..n];
            println!("Zsh Echo Raw Bytes: {:?}", response);
            println!("Zsh Echo Lossy String: {:?}", String::from_utf8_lossy(response));
        }
    }

    #[test]
    fn test_terminal_grid_zsh_myanmar() {
        let state = Arc::new(Mutex::new(TerminalState {
            grid: vec![vec![Cell::default(); 80]; 24],
            scrollback: Vec::new(),
            cursor_x: 0,
            cursor_y: 0,
            cols: 80,
            rows: 24,
            scroll_offset: 0,
            cursor_visible: true,
            ctx: None,
        }));
        let mut handler = VteHandler::new(state.clone(), TerminalTheme::default(), 80, 24);
        let mut parser = vte::Parser::new();

        // Raw bytes from Zsh echo
        let zsh_bytes: &[u8] = &[
            225, 128, 148, 8, 225, 128, 148, 225, 128, 177, 225, 128, 128, 225, 128, 177,
            225, 128, 172, 225, 128, 132, 8, 225, 128, 132, 225, 128, 186, 225, 128, 184,
            225, 128, 149, 225, 128, 171, 225, 128, 158, 225, 128, 156, 225, 128, 172,
            225, 128, 184
        ];

        for &byte in zsh_bytes {
            parser.advance(&mut handler, byte);
        }
        handler.sync_state();

        let s = state.lock();
        for col in 0..15 {
            println!("Col {}: {:?}", col, s.grid[0][col].ch);
        }
    }
}

