mod core;
mod sftp;
mod ssh;
mod sysinfo_mod;
mod tui;
mod tunnel;
mod wsl;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use tui::app::{App, Tab};

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

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Ctrl+C always quits
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
                {
                    app.quit();
                    continue;
                }

                if app.input_mode {
                    handle_input_mode(app, key.code);
                } else {
                    handle_normal_mode(app, key.code);
                }
            }
        }
    }
    Ok(())
}

fn handle_input_mode(app: &mut App, key: KeyCode) {
    match app.active_tab {
        Tab::Wsl => match key {
            KeyCode::Esc => app.input_mode = false,
            KeyCode::Enter => {
                app.input_mode = false;
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
            KeyCode::Esc => app.input_mode = false,
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
                app.input_mode = false;
                app.status_message = format!(
                    "Connecting to {}@{}:{}...",
                    app.ssh_form.username, app.ssh_form.host, app.ssh_form.port
                );
            }
            _ => {}
        },
        _ => {
            if let KeyCode::Esc = key {
                app.input_mode = false;
            }
        }
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

        // Input mode
        KeyCode::Char('i') => {
            app.input_mode = true;
        }

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
