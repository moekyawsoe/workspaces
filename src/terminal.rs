use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::thread;
use egui::Context;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use vte::{Params, Parser, Perform};

// ── Cell & Grid ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: egui::Color32,
    pub bg: egui::Color32,
    pub bold: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            fg: egui::Color32::from_rgb(220, 220, 220),
            bg: egui::Color32::TRANSPARENT,
            bold: false,
        }
    }
}

#[derive(Clone)]
pub struct TerminalGrid {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scrollback: Vec<Vec<Cell>>,  // lines that scrolled off the top
}

impl TerminalGrid {
    pub fn new(rows: usize, cols: usize) -> Self {
        TerminalGrid {
            rows,
            cols,
            cells: vec![vec![Cell::default(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            scrollback: Vec::new(),
        }
    }

    fn ensure_row(&mut self, row: usize) {
        while self.cells.len() <= row {
            self.cells.push(vec![Cell::default(); self.cols]);
        }
    }

    fn ensure_col(&mut self, row: usize, col: usize) {
        self.ensure_row(row);
        while self.cells[row].len() <= col {
            self.cells[row].push(Cell::default());
        }
    }

    pub fn all_lines(&self) -> Vec<Vec<Cell>> {
        let mut all = self.scrollback.clone();
        all.extend(self.cells.iter().cloned());
        all
    }
}

// ── VTE Performer ────────────────────────────────────────────────────────────

pub struct TermPerformer {
    pub grid: TerminalGrid,
    fg: egui::Color32,
    bg: egui::Color32,
    bold: bool,
    saved_cursor: (usize, usize),
}

impl TermPerformer {
    pub fn new(rows: usize, cols: usize) -> Self {
        TermPerformer {
            grid: TerminalGrid::new(rows, cols),
            fg: egui::Color32::from_rgb(220, 220, 220),
            bg: egui::Color32::TRANSPARENT,
            bold: false,
            saved_cursor: (0, 0),
        }
    }

    fn put_char(&mut self, ch: char) {
        let row = self.grid.cursor_row;
        let col = self.grid.cursor_col;
        self.grid.ensure_col(row, col);
        self.grid.cells[row][col] = Cell {
            ch,
            fg: self.fg,
            bg: self.bg,
            bold: self.bold,
        };
        self.grid.cursor_col += 1;
    }

    fn scroll_up(&mut self) {
        if !self.grid.cells.is_empty() {
            let old_row = self.grid.cells.remove(0);
            self.grid.scrollback.push(old_row);
            // Keep scrollback to 5000 lines
            if self.grid.scrollback.len() > 5000 {
                self.grid.scrollback.remove(0);
            }
            self.grid.cells.push(vec![Cell::default(); self.grid.cols]);
        }
    }

    fn newline(&mut self) {
        self.grid.cursor_row += 1;
        if self.grid.cursor_row >= self.grid.rows {
            self.scroll_up();
            self.grid.cursor_row = self.grid.rows - 1;
        }
        self.grid.ensure_row(self.grid.cursor_row);
    }

    fn ansi_color(n: u16, bold: bool) -> egui::Color32 {
        match n {
            0 => if bold { egui::Color32::from_rgb(85, 85, 85) } else { egui::Color32::from_rgb(0, 0, 0) },
            1 => if bold { egui::Color32::from_rgb(255, 85, 85) } else { egui::Color32::from_rgb(170, 0, 0) },
            2 => if bold { egui::Color32::from_rgb(85, 255, 85) } else { egui::Color32::from_rgb(0, 170, 0) },
            3 => if bold { egui::Color32::from_rgb(255, 255, 85) } else { egui::Color32::from_rgb(170, 85, 0) },
            4 => if bold { egui::Color32::from_rgb(85, 85, 255) } else { egui::Color32::from_rgb(0, 0, 170) },
            5 => if bold { egui::Color32::from_rgb(255, 85, 255) } else { egui::Color32::from_rgb(170, 0, 170) },
            6 => if bold { egui::Color32::from_rgb(85, 255, 255) } else { egui::Color32::from_rgb(0, 170, 170) },
            7 => if bold { egui::Color32::WHITE } else { egui::Color32::from_rgb(170, 170, 170) },
            _ => egui::Color32::from_rgb(220, 220, 220),
        }
    }

    fn apply_sgr(&mut self, params: &[u16]) {
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => {
                    self.fg = egui::Color32::from_rgb(220, 220, 220);
                    self.bg = egui::Color32::TRANSPARENT;
                    self.bold = false;
                }
                1 => { self.bold = true; }
                2 | 22 => { self.bold = false; }
                30..=37 => { self.fg = Self::ansi_color(params[i] - 30, self.bold); }
                38 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        self.fg = Self::ansi_256(params[i + 2]);
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        self.fg = egui::Color32::from_rgb(params[i+2] as u8, params[i+3] as u8, params[i+4] as u8);
                        i += 4;
                    }
                }
                39 => { self.fg = egui::Color32::from_rgb(220, 220, 220); }
                40..=47 => { self.bg = Self::ansi_color(params[i] - 40, false); }
                48 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        self.bg = Self::ansi_256(params[i + 2]);
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        self.bg = egui::Color32::from_rgb(params[i+2] as u8, params[i+3] as u8, params[i+4] as u8);
                        i += 4;
                    }
                }
                49 => { self.bg = egui::Color32::TRANSPARENT; }
                90..=97 => { self.fg = Self::ansi_color(params[i] - 90, true); }
                100..=107 => { self.bg = Self::ansi_color(params[i] - 100, true); }
                _ => {}
            }
            i += 1;
        }
    }

    fn ansi_256(n: u16) -> egui::Color32 {
        if n < 16 {
            return Self::ansi_color(n, n >= 8);
        }
        if n >= 232 {
            let v = (n - 232) * 10 + 8;
            return egui::Color32::from_rgb(v as u8, v as u8, v as u8);
        }
        let n = n - 16;
        let b = (n % 6) * 51;
        let g = ((n / 6) % 6) * 51;
        let r = ((n / 36) % 6) * 51;
        egui::Color32::from_rgb(r as u8, g as u8, b as u8)
    }
}

impl Perform for TermPerformer {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\r' => { self.grid.cursor_col = 0; }
            b'\n' => { self.newline(); }
            b'\x08' => {
                if self.grid.cursor_col > 0 {
                    self.grid.cursor_col -= 1;
                }
            }
            b'\x07' => {} // bell - ignore
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        let ps: Vec<u16> = params.iter()
            .map(|p| p.first().copied().unwrap_or(0))
            .collect();

        match action {
            'm' => { self.apply_sgr(&ps); }
            'A' => { // cursor up
                let n = ps.first().copied().unwrap_or(1).max(1) as usize;
                self.grid.cursor_row = self.grid.cursor_row.saturating_sub(n);
            }
            'B' => { // cursor down
                let n = ps.first().copied().unwrap_or(1).max(1) as usize;
                self.grid.cursor_row = (self.grid.cursor_row + n).min(self.grid.rows - 1);
            }
            'C' => { // cursor forward
                let n = ps.first().copied().unwrap_or(1).max(1) as usize;
                self.grid.cursor_col += n;
            }
            'D' => { // cursor back
                let n = ps.first().copied().unwrap_or(1).max(1) as usize;
                self.grid.cursor_col = self.grid.cursor_col.saturating_sub(n);
            }
            'H' | 'f' => { // cursor position
                let row = ps.first().copied().unwrap_or(1).max(1) as usize - 1;
                let col = ps.get(1).copied().unwrap_or(1).max(1) as usize - 1;
                self.grid.cursor_row = row.min(self.grid.rows - 1);
                self.grid.cursor_col = col;
                self.grid.ensure_row(self.grid.cursor_row);
            }
            'J' => { // erase display
                match ps.first().copied().unwrap_or(0) {
                    0 => {
                        let row = self.grid.cursor_row;
                        let col = self.grid.cursor_col;
                        if let Some(r) = self.grid.cells.get_mut(row) {
                            for c in r.iter_mut().skip(col) { *c = Cell::default(); }
                        }
                        for r in self.grid.cells.iter_mut().skip(row + 1) {
                            for c in r.iter_mut() { *c = Cell::default(); }
                        }
                    }
                    1 => {
                        let row = self.grid.cursor_row;
                        let col = self.grid.cursor_col;
                        for r in self.grid.cells.iter_mut().take(row) {
                            for c in r.iter_mut() { *c = Cell::default(); }
                        }
                        if let Some(r) = self.grid.cells.get_mut(row) {
                            for c in r.iter_mut().take(col + 1) { *c = Cell::default(); }
                        }
                    }
                    2 | 3 => {
                        // clear entire screen - move to scrollback if 2
                        if ps.first().copied().unwrap_or(0) == 2 {
                            let drained: Vec<_> = self.grid.cells.drain(..).collect();
                            self.grid.scrollback.extend(drained);
                        }
                        self.grid.cells = vec![vec![Cell::default(); self.grid.cols]; self.grid.rows];
                        self.grid.cursor_row = 0;
                        self.grid.cursor_col = 0;
                    }
                    _ => {}
                }
            }
            'K' => { // erase line
                let row = self.grid.cursor_row;
                let col = self.grid.cursor_col;
                match ps.first().copied().unwrap_or(0) {
                    0 => {
                        if let Some(r) = self.grid.cells.get_mut(row) {
                            for c in r.iter_mut().skip(col) { *c = Cell::default(); }
                        }
                    }
                    1 => {
                        if let Some(r) = self.grid.cells.get_mut(row) {
                            for c in r.iter_mut().take(col + 1) { *c = Cell::default(); }
                        }
                    }
                    2 => {
                        if let Some(r) = self.grid.cells.get_mut(row) {
                            for c in r.iter_mut() { *c = Cell::default(); }
                        }
                    }
                    _ => {}
                }
            }
            'P' => { // delete chars
                let n = ps.first().copied().unwrap_or(1).max(1) as usize;
                let row = self.grid.cursor_row;
                let col = self.grid.cursor_col;
                if let Some(r) = self.grid.cells.get_mut(row) {
                    let len = r.len();
                    if col < len {
                        let end = (col + n).min(len);
                        r.drain(col..end);
                        while r.len() < len { r.push(Cell::default()); }
                    }
                }
            }
            's' => { self.saved_cursor = (self.grid.cursor_row, self.grid.cursor_col); }
            'u' => {
                self.grid.cursor_row = self.saved_cursor.0;
                self.grid.cursor_col = self.saved_cursor.1;
            }
            'l' | 'h' => {} // mode set/reset — ignore
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => { self.saved_cursor = (self.grid.cursor_row, self.grid.cursor_col); }
            b'8' => {
                self.grid.cursor_row = self.saved_cursor.0;
                self.grid.cursor_col = self.saved_cursor.1;
            }
            b'M' => { // reverse index
                if self.grid.cursor_row > 0 {
                    self.grid.cursor_row -= 1;
                }
            }
            _ => {}
        }
    }
}

// ── Shared state between PTY thread and egui ─────────────────────────────────

pub struct TerminalState {
    pub performer: TermPerformer,
    pub parser: Parser,
}

impl TerminalState {
    pub fn new(rows: usize, cols: usize) -> Self {
        TerminalState {
            performer: TermPerformer::new(rows, cols),
            parser: Parser::new(),
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.parser.advance(&mut self.performer, b);
        }
    }
}

// ── TerminalInstance ──────────────────────────────────────────────────────────

pub struct TerminalInstance {
    pub state: Arc<Mutex<TerminalState>>,
    pub pty_writer: Option<Box<dyn std::io::Write + Send>>,
    pub working_dir: PathBuf,
    pub rows: usize,
    pub cols: usize,
}

impl TerminalInstance {
    pub fn new(working_dir: PathBuf) -> Self {
        let rows = 24usize;
        let cols = 200usize;
        TerminalInstance {
            state: Arc::new(Mutex::new(TerminalState::new(rows, cols))),
            pty_writer: None,
            working_dir,
            rows,
            cols,
        }
    }

    pub fn start(&mut self, shell_path: &str, ctx: Context) {
        self.stop();

        let shell = if shell_path.is_empty() {
            if cfg!(target_os = "windows") { "cmd.exe" } else { "/bin/bash" }
        } else {
            shell_path
        };

        let pty_system = native_pty_system();
        let size = PtySize {
            rows: self.rows as u16,
            cols: self.cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        };

        match pty_system.openpty(size) {
            Ok(pair) => {
                let mut cmd = CommandBuilder::new(shell);
                cmd.cwd(&self.working_dir);

                // Set TERM so the shell knows it has a real terminal
                cmd.env("TERM", "xterm-256color");
                cmd.env("COLORTERM", "truecolor");

                match pair.slave.spawn_command(cmd) {
                    Ok(_child) => {
                        // Writer (master) — for sending keystrokes
                        let writer = pair.master.take_writer().expect("PTY writer");
                        self.pty_writer = Some(writer);

                        // Reader thread — reads PTY output and feeds into VTE
                        let state = Arc::clone(&self.state);
                        let mut reader = pair.master.try_clone_reader().expect("PTY reader");
                        thread::spawn(move || {
                            let mut buf = [0u8; 4096];
                            loop {
                                match reader.read(&mut buf) {
                                    Ok(0) | Err(_) => break,
                                    Ok(n) => {
                                        if let Ok(mut st) = state.lock() {
                                            st.process(&buf[..n]);
                                        }
                                        ctx.request_repaint();
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        if let Ok(mut st) = self.state.lock() {
                            st.performer.put_char('E');
                            st.performer.put_char('r');
                            st.performer.put_char('r');
                            let _ = e; // suppress unused warning
                        }
                    }
                }
            }
            Err(_e) => {}
        }
    }

    pub fn write_input(&mut self, data: &[u8]) {
        if let Some(w) = &mut self.pty_writer {
            let _ = w.write_all(data);
        }
    }

    pub fn write_str(&mut self, s: &str) {
        self.write_input(s.as_bytes());
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.rows = rows;
        self.cols = cols;
        // Resize VTE grid
        if let Ok(mut st) = self.state.lock() {
            st.performer.grid.rows = rows;
            st.performer.grid.cols = cols;
            // Trim rows
            while st.performer.grid.cells.len() > rows {
                let row = st.performer.grid.cells.remove(0);
                st.performer.grid.scrollback.push(row);
            }
            while st.performer.grid.cells.len() < rows {
                st.performer.grid.cells.push(vec![Cell::default(); cols]);
            }
            // Clip cursor
            st.performer.grid.cursor_row = st.performer.grid.cursor_row.min(rows - 1);
        }
    }

    pub fn change_dir(&mut self, path: PathBuf, shell_path: &str, ctx: Context) {
        self.working_dir = path;
        self.start(shell_path, ctx);
    }

    pub fn stop(&mut self) {
        self.pty_writer = None;
    }
}

impl Drop for TerminalInstance {
    fn drop(&mut self) {
        self.stop();
    }
}

// Need Read for reader thread
use std::io::Read;
use std::io::Write;
