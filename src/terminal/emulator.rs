use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use vt100::Parser;

/// A real terminal emulator backed by vt100 parser and a PTY.
pub struct TerminalEmulator {
    parser: Arc<Mutex<Parser>>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    reader_handle: Option<std::thread::JoinHandle<()>>,
    pub rows: u16,
    pub cols: u16,
    pub title: String,
    pub running: bool,
}

impl TerminalEmulator {
    /// Spawn a new terminal with the given command
    pub fn spawn(cmd: &str, args: &[&str], rows: u16, cols: u16) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut command = CommandBuilder::new(cmd);
        for arg in args {
            command.arg(*arg);
        }

        // Set environment for color support
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");

        let _child = pair.slave.spawn_command(command)?;
        drop(pair.slave);

        let writer = pair.master.take_writer()?;
        let mut reader = pair.master.try_clone_reader()?;

        let parser = Arc::new(Mutex::new(Parser::new(rows, cols, 1000)));

        // Background thread to read PTY output and feed the vt100 parser
        let parser_clone = Arc::clone(&parser);
        let reader_handle = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut p) = parser_clone.lock() {
                            p.process(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            parser,
            master: pair.master,
            writer,
            reader_handle: Some(reader_handle),
            rows,
            cols,
            title: cmd.to_string(),
            running: true,
        })
    }

    /// Spawn a local shell
    pub fn spawn_shell(rows: u16, cols: u16) -> anyhow::Result<Self> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        Self::spawn(&shell, &[], rows, cols)
    }

    /// Spawn a WSL distribution shell
    pub fn spawn_wsl(distro: &str, rows: u16, cols: u16) -> anyhow::Result<Self> {
        let mut emu = Self::spawn("wsl", &["-d", distro], rows, cols)?;
        emu.title = format!("WSL: {}", distro);
        Ok(emu)
    }

    /// Spawn an SSH session
    pub fn spawn_ssh(
        host: &str,
        port: u16,
        username: &str,
        rows: u16,
        cols: u16,
    ) -> anyhow::Result<Self> {
        let port_str = port.to_string();
        let user_host = format!("{}@{}", username, host);
        let mut emu = Self::spawn(
            "ssh",
            &["-p", &port_str, "-o", "StrictHostKeyChecking=accept-new", &user_host],
            rows,
            cols,
        )?;
        emu.title = format!("SSH: {}@{}:{}", username, host, port);
        Ok(emu)
    }

    /// Write raw bytes to the terminal (user input)
    pub fn write_input(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Write a single character
    pub fn write_char(&mut self, c: char) -> anyhow::Result<()> {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.write_input(s.as_bytes())
    }

    /// Send a special key sequence
    pub fn send_key(&mut self, key: TerminalKey) -> anyhow::Result<()> {
        let seq = match key {
            TerminalKey::Enter => b"\r".as_slice(),
            TerminalKey::Tab => b"\t".as_slice(),
            TerminalKey::Backspace => b"\x7f".as_slice(),
            TerminalKey::Escape => b"\x1b".as_slice(),
            TerminalKey::Up => b"\x1b[A".as_slice(),
            TerminalKey::Down => b"\x1b[B".as_slice(),
            TerminalKey::Right => b"\x1b[C".as_slice(),
            TerminalKey::Left => b"\x1b[D".as_slice(),
            TerminalKey::Home => b"\x1b[H".as_slice(),
            TerminalKey::End => b"\x1b[F".as_slice(),
            TerminalKey::PageUp => b"\x1b[5~".as_slice(),
            TerminalKey::PageDown => b"\x1b[6~".as_slice(),
            TerminalKey::Delete => b"\x1b[3~".as_slice(),
            TerminalKey::Insert => b"\x1b[2~".as_slice(),
            TerminalKey::F(n) => {
                return self.send_function_key(n);
            }
            TerminalKey::CtrlC => b"\x03".as_slice(),
            TerminalKey::CtrlD => b"\x04".as_slice(),
            TerminalKey::CtrlZ => b"\x1a".as_slice(),
            TerminalKey::CtrlL => b"\x0c".as_slice(),
        };
        self.write_input(seq)
    }

    fn send_function_key(&mut self, n: u8) -> anyhow::Result<()> {
        let seq = match n {
            1 => "\x1bOP",
            2 => "\x1bOQ",
            3 => "\x1bOR",
            4 => "\x1bOS",
            5 => "\x1b[15~",
            6 => "\x1b[17~",
            7 => "\x1b[18~",
            8 => "\x1b[19~",
            9 => "\x1b[20~",
            10 => "\x1b[21~",
            11 => "\x1b[23~",
            12 => "\x1b[24~",
            _ => return Ok(()),
        };
        self.write_input(seq.as_bytes())
    }

    /// Get the current screen content as styled lines for ratatui rendering
    pub fn get_screen_lines(&self) -> Vec<TerminalLine> {
        let parser = match self.parser.lock() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };

        let screen = parser.screen();
        let mut lines = Vec::with_capacity(self.rows as usize);

        for row in 0..self.rows {
            let mut cells = Vec::new();
            for col in 0..self.cols {
                let cell = screen.cell(row, col);
                match cell {
                    Some(cell) => {
                        cells.push(TerminalCell {
                            c: cell.contents().chars().next().unwrap_or(' '),
                            fg: convert_color(cell.fgcolor()),
                            bg: convert_color(cell.bgcolor()),
                            bold: cell.bold(),
                            italic: cell.italic(),
                            underline: cell.underline(),
                            inverse: cell.inverse(),
                        });
                    }
                    None => {
                        cells.push(TerminalCell::default());
                    }
                }
            }
            lines.push(TerminalLine { cells });
        }

        lines
    }

    /// Get cursor position
    pub fn cursor_position(&self) -> (u16, u16) {
        match self.parser.lock() {
            Ok(p) => p.screen().cursor_position(),
            Err(_) => (0, 0),
        }
    }

    /// Resize the terminal
    pub fn resize(&mut self, rows: u16, cols: u16) -> anyhow::Result<()> {
        self.rows = rows;
        self.cols = cols;
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        if let Ok(mut p) = self.parser.lock() {
            p.set_size(rows, cols);
        }
        Ok(())
    }

    /// Check if the terminal process is still running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Drop for TerminalEmulator {
    fn drop(&mut self) {
        // Signal the process to exit
        let _ = self.write_input(b"\x04"); // Ctrl+D
        if let Some(handle) = self.reader_handle.take() {
            let _ = handle.join();
        }
    }
}

pub enum TerminalKey {
    Enter,
    Tab,
    Backspace,
    Escape,
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    F(u8),
    CtrlC,
    CtrlD,
    CtrlZ,
    CtrlL,
}

#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub cells: Vec<TerminalCell>,
}

#[derive(Debug, Clone)]
pub struct TerminalCell {
    pub c: char,
    pub fg: TermColor,
    pub bg: TermColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: TermColor::Default,
            bg: TermColor::Default,
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TermColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

fn convert_color(color: vt100::Color) -> TermColor {
    match color {
        vt100::Color::Default => TermColor::Default,
        vt100::Color::Idx(i) => TermColor::Indexed(i),
        vt100::Color::Rgb(r, g, b) => TermColor::Rgb(r, g, b),
    }
}

/// Convert our TermColor to ratatui Color
pub fn to_ratatui_color(color: TermColor) -> ratatui::style::Color {
    match color {
        TermColor::Default => ratatui::style::Color::Reset,
        TermColor::Indexed(i) => ratatui::style::Color::Indexed(i),
        TermColor::Rgb(r, g, b) => ratatui::style::Color::Rgb(r, g, b),
    }
}
