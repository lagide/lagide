mod core;
mod sftp;
mod ssh;
mod sysinfo_mod;
mod terminal;
mod tui;
mod tunnel;
mod wsl;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use terminal::emulator::TerminalKey;
use tui::app::{App, InputMode, Tab};

fn main() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    println!("Goodbye from Lagide!");
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    while app.running {
        terminal.draw(|frame| tui::ui::draw(frame, app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Terminal => handle_terminal_mode(app, key),
                    InputMode::Editing => handle_editing_mode(app, key.code),
                    InputMode::TunnelForm => handle_tunnel_form_mode(app, key.code),
                    InputMode::Normal => {
                        // Ctrl+C quits from normal mode
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            app.quit();
                        } else {
                            handle_normal_mode(app, key.code);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Handle keys when a real terminal emulator is active.
/// All keys are passed through to the PTY except Ctrl+\ (detach).
fn handle_terminal_mode(app: &mut App, key: KeyEvent) {
    let idx = match app.active_terminal {
        Some(i) => i,
        None => {
            app.input_mode = InputMode::Normal;
            return;
        }
    };

    if idx >= app.terminal_tabs.len() {
        app.input_mode = InputMode::Normal;
        app.active_terminal = None;
        return;
    }

    let emu = &mut app.terminal_tabs[idx].emulator;

    // Ctrl+\ = detach from terminal (go back to normal mode)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('\\') {
        app.input_mode = InputMode::Normal;
        app.status_message = format!(
            "Detached from terminal. {} terminals open. Press Enter on WSL/SSH to reattach.",
            app.terminal_tabs.len()
        );
        return;
    }

    // Pass all other keys to the terminal emulator
    let result = if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => emu.send_key(TerminalKey::CtrlC),
            KeyCode::Char('d') => emu.send_key(TerminalKey::CtrlD),
            KeyCode::Char('z') => emu.send_key(TerminalKey::CtrlZ),
            KeyCode::Char('l') => emu.send_key(TerminalKey::CtrlL),
            KeyCode::Char(c) => {
                // Generic ctrl+key: send as control character
                let ctrl_byte = (c as u8).wrapping_sub(b'a').wrapping_add(1);
                emu.write_input(&[ctrl_byte])
            }
            _ => Ok(()),
        }
    } else {
        match key.code {
            KeyCode::Char(c) => emu.write_char(c),
            KeyCode::Enter => emu.send_key(TerminalKey::Enter),
            KeyCode::Backspace => emu.send_key(TerminalKey::Backspace),
            KeyCode::Tab => emu.send_key(TerminalKey::Tab),
            KeyCode::Esc => emu.send_key(TerminalKey::Escape),
            KeyCode::Up => emu.send_key(TerminalKey::Up),
            KeyCode::Down => emu.send_key(TerminalKey::Down),
            KeyCode::Left => emu.send_key(TerminalKey::Left),
            KeyCode::Right => emu.send_key(TerminalKey::Right),
            KeyCode::Home => emu.send_key(TerminalKey::Home),
            KeyCode::End => emu.send_key(TerminalKey::End),
            KeyCode::PageUp => emu.send_key(TerminalKey::PageUp),
            KeyCode::PageDown => emu.send_key(TerminalKey::PageDown),
            KeyCode::Delete => emu.send_key(TerminalKey::Delete),
            KeyCode::Insert => emu.send_key(TerminalKey::Insert),
            KeyCode::F(n) => emu.send_key(TerminalKey::F(n)),
            _ => Ok(()),
        }
    };

    if let Err(e) = result {
        app.status_message = format!("Terminal I/O error: {}", e);
        app.close_active_terminal();
    }
}

/// Handle keys in editing mode (SSH form, WSL command input)
fn handle_editing_mode(app: &mut App, key: KeyCode) {
    match app.active_tab {
        Tab::Wsl => match key {
            KeyCode::Esc => app.input_mode = InputMode::Normal,
            KeyCode::Enter => {
                app.input_mode = InputMode::Normal;
                app.exec_wsl_command();
            }
            KeyCode::Backspace => {
                app.command_input.pop();
            }
            KeyCode::Char(c) => {
                app.command_input.push(c);
            }
            _ => {}
        },
        Tab::Ssh => match key {
            KeyCode::Esc => app.input_mode = InputMode::Normal,
            KeyCode::Tab => {
                app.ssh_form.focused_field = (app.ssh_form.focused_field + 1) % 4;
            }
            KeyCode::BackTab => {
                app.ssh_form.focused_field = if app.ssh_form.focused_field == 0 {
                    3
                } else {
                    app.ssh_form.focused_field - 1
                };
            }
            KeyCode::Backspace => {
                let field = get_ssh_field_mut(app);
                field.pop();
            }
            KeyCode::Char(c) => {
                let field = get_ssh_field_mut(app);
                field.push(c);
            }
            KeyCode::Enter => {
                // Open a real SSH terminal
                app.input_mode = InputMode::Normal;
                let host = app.ssh_form.host.clone();
                let port: u16 = app.ssh_form.port.parse().unwrap_or(22);
                let username = app.ssh_form.username.clone();
                if !host.is_empty() && !username.is_empty() {
                    app.open_ssh_terminal(&host, port, &username, 24, 80);
                } else {
                    app.status_message =
                        "Please fill in Host and Username fields".to_string();
                }
            }
            _ => {}
        },
        _ => {
            if let KeyCode::Esc = key {
                app.input_mode = InputMode::Normal;
            }
        }
    }
}

/// Handle keys in tunnel form mode
fn handle_tunnel_form_mode(app: &mut App, key: KeyCode) {
    let num_fields: usize = 8;

    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.tunnel_form.active = false;
            app.status_message = "Tunnel form cancelled".to_string();
        }
        KeyCode::Tab => {
            app.tunnel_form.focused_field = (app.tunnel_form.focused_field + 1) % num_fields;
        }
        KeyCode::BackTab => {
            app.tunnel_form.focused_field = if app.tunnel_form.focused_field == 0 {
                num_fields - 1
            } else {
                app.tunnel_form.focused_field - 1
            };
        }
        KeyCode::Char(' ') if app.tunnel_form.focused_field == 1 => {
            // Cycle tunnel type
            app.tunnel_form.tunnel_type = app.tunnel_form.tunnel_type.next();
        }
        KeyCode::Enter => {
            // Create tunnel
            app.create_tunnel_from_form();
            app.input_mode = InputMode::Normal;
            app.tunnel_form.active = false;
        }
        KeyCode::Backspace => {
            if app.tunnel_form.focused_field != 1 {
                let field = get_tunnel_field_mut(app);
                field.pop();
            }
        }
        KeyCode::Char(c) => {
            if app.tunnel_form.focused_field != 1 {
                let field = get_tunnel_field_mut(app);
                field.push(c);
            }
        }
        _ => {}
    }
}

fn handle_normal_mode(app: &mut App, key: KeyCode) {
    match key {
        // Quit
        KeyCode::Char('q') => app.quit(),

        // Help
        KeyCode::Char('?') => app.show_help = !app.show_help,

        // Tab navigation
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
            if !app.show_help {
                app.next_tab();
            }
        }
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
            if !app.show_help {
                app.prev_tab();
            }
        }

        // Direct tab jump
        KeyCode::Char('1') => app.active_tab = Tab::Dashboard,
        KeyCode::Char('2') => app.active_tab = Tab::Wsl,
        KeyCode::Char('3') => app.active_tab = Tab::Ssh,
        KeyCode::Char('4') => app.active_tab = Tab::Sftp,
        KeyCode::Char('5') => app.active_tab = Tab::Tunnels,
        KeyCode::Char('6') => app.active_tab = Tab::Sessions,
        KeyCode::Char('7') => app.active_tab = Tab::SysInfo,

        // Input mode (editing forms)
        KeyCode::Char('i') => {
            app.input_mode = InputMode::Editing;
        }

        // Enter: context-dependent action
        KeyCode::Enter => match app.active_tab {
            Tab::Wsl => {
                // Open a real WSL terminal
                let distros = app.wsl_manager.list_distributions().unwrap_or_default();
                if let Some(d) = distros.get(app.wsl_selected) {
                    let name = d.name.clone();
                    app.open_wsl_terminal(&name, 24, 80);
                }
            }
            Tab::Ssh => {
                // Switch to editing mode for SSH form
                app.input_mode = InputMode::Editing;
            }
            Tab::Dashboard => {
                // Open a local shell
                app.open_local_terminal(24, 80);
            }
            _ => {}
        },

        // Vertical navigation
        KeyCode::Down | KeyCode::Char('j') => match app.active_tab {
            Tab::Wsl => {
                let count = app
                    .wsl_manager
                    .list_distributions()
                    .map(|d| d.len())
                    .unwrap_or(0);
                if count > 0 {
                    app.wsl_selected = (app.wsl_selected + 1) % count;
                }
            }
            Tab::Sessions => {
                let count = app.session_manager.sessions.len();
                if count > 0 {
                    app.session_selected = (app.session_selected + 1) % count;
                }
            }
            Tab::Tunnels => {
                let count = app.tunnel_manager.list_tunnels().len();
                if count > 0 {
                    app.tunnel_selected = (app.tunnel_selected + 1) % count;
                }
            }
            _ => {}
        },
        KeyCode::Up | KeyCode::Char('k') => match app.active_tab {
            Tab::Wsl => {
                let count = app
                    .wsl_manager
                    .list_distributions()
                    .map(|d| d.len())
                    .unwrap_or(0);
                if count > 0 {
                    app.wsl_selected = if app.wsl_selected == 0 {
                        count - 1
                    } else {
                        app.wsl_selected - 1
                    };
                }
            }
            Tab::Sessions => {
                let count = app.session_manager.sessions.len();
                if count > 0 {
                    app.session_selected = if app.session_selected == 0 {
                        count - 1
                    } else {
                        app.session_selected - 1
                    };
                }
            }
            Tab::Tunnels => {
                let count = app.tunnel_manager.list_tunnels().len();
                if count > 0 {
                    app.tunnel_selected = if app.tunnel_selected == 0 {
                        count - 1
                    } else {
                        app.tunnel_selected - 1
                    };
                }
            }
            _ => {}
        },

        // WSL specific
        KeyCode::Char('s') if app.active_tab == Tab::Wsl => {
            let distros = app.wsl_manager.list_distributions().unwrap_or_default();
            if let Some(d) = distros.get(app.wsl_selected) {
                match app.wsl_manager.start_distribution(&d.name) {
                    Ok(()) => {
                        app.status_message = format!("Started {}", d.name);
                    }
                    Err(e) => app.status_message = format!("Error: {}", e),
                }
            }
        }
        KeyCode::Char('x') if app.active_tab == Tab::Wsl => {
            let distros = app.wsl_manager.list_distributions().unwrap_or_default();
            if let Some(d) = distros.get(app.wsl_selected) {
                match app.wsl_manager.stop_distribution(&d.name) {
                    Ok(()) => {
                        app.status_message = format!("Stopped {}", d.name);
                    }
                    Err(e) => app.status_message = format!("Error: {}", e),
                }
            }
        }

        // Tunnel: 'n' to open creation form
        KeyCode::Char('n') if app.active_tab == Tab::Tunnels => {
            app.tunnel_form.active = true;
            app.input_mode = InputMode::TunnelForm;
            app.tunnel_form.focused_field = 0;
            app.status_message = "Fill in tunnel details. Tab=next, Space=type, Enter=create".to_string();
        }

        // Tunnel: 'd' to stop selected
        KeyCode::Char('d') if app.active_tab == Tab::Tunnels => {
            app.stop_selected_tunnel();
        }

        // Delete session
        KeyCode::Char('d') if app.active_tab == Tab::Sessions => {
            let sessions = &app.session_manager.sessions;
            if let Some(s) = sessions.get(app.session_selected) {
                let id = s.id.clone();
                app.session_manager.remove_session(&id);
                let _ = app.session_manager.save();
                app.status_message = "Session deleted".to_string();
                if app.session_selected > 0 {
                    app.session_selected -= 1;
                }
            }
        }

        _ => {}
    }
}

fn get_ssh_field_mut(app: &mut App) -> &mut String {
    match app.ssh_form.focused_field {
        0 => &mut app.ssh_form.host,
        1 => &mut app.ssh_form.port,
        2 => &mut app.ssh_form.username,
        3 => &mut app.ssh_form.password,
        _ => &mut app.ssh_form.host,
    }
}

fn get_tunnel_field_mut(app: &mut App) -> &mut String {
    match app.tunnel_form.focused_field {
        0 => &mut app.tunnel_form.name,
        // 1 = type (handled with Space, not text input)
        2 => &mut app.tunnel_form.local_port,
        3 => &mut app.tunnel_form.remote_host,
        4 => &mut app.tunnel_form.remote_port,
        5 => &mut app.tunnel_form.ssh_host,
        6 => &mut app.tunnel_form.ssh_port,
        7 => &mut app.tunnel_form.ssh_user,
        _ => &mut app.tunnel_form.name,
    }
}
