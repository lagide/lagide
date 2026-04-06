use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

use super::app::{App, Tab};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(10),  // Content
            Constraint::Length(3), // Status bar
        ])
        .split(frame.area());

    draw_tabs(frame, app, chunks[0]);
    draw_content(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);

    if app.show_help {
        draw_help_popup(frame, app);
    }
}

fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == app.active_tab {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(format!(" {} ", t.title()), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Lagide - Terminal Hub ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        )
        .select(app.active_tab.index())
        .highlight_style(Style::default().fg(Color::Cyan));

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        Tab::Dashboard => draw_dashboard(frame, app, area),
        Tab::Wsl => draw_wsl(frame, app, area),
        Tab::Ssh => draw_ssh(frame, app, area),
        Tab::Sftp => draw_sftp(frame, area),
        Tab::Tunnels => draw_tunnels(frame, app, area),
        Tab::Sessions => draw_sessions(frame, app, area),
        Tab::SysInfo => draw_sysinfo(frame, app, area),
    }
}

fn draw_dashboard(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    // Quick stats
    let info = app.sys_info.gather();
    let stats_text = vec![
        Line::from(vec![
            Span::styled("Host: ", Style::default().fg(Color::Cyan)),
            Span::raw(&info.hostname),
        ]),
        Line::from(vec![
            Span::styled("OS: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} {}", info.os_name, info.os_version)),
        ]),
        Line::from(vec![
            Span::styled("Kernel: ", Style::default().fg(Color::Cyan)),
            Span::raw(&info.kernel_version),
        ]),
        Line::from(vec![
            Span::styled("CPUs: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} ({:.1}%)", info.cpu_count, info.cpu_usage)),
        ]),
        Line::from(vec![
            Span::styled("Memory: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{} / {} MB ({:.0}%)",
                info.used_memory_mb,
                info.total_memory_mb,
                if info.total_memory_mb > 0 {
                    (info.used_memory_mb as f64 / info.total_memory_mb as f64) * 100.0
                } else {
                    0.0
                }
            )),
        ]),
        Line::from(vec![
            Span::styled("Uptime: ", Style::default().fg(Color::Cyan)),
            Span::raw(format_uptime(info.uptime_secs)),
        ]),
    ];

    let stats = Paragraph::new(stats_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" System Overview ")
                .title_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(stats, left_chunks[0]);

    // WSL distributions
    let distros = app.wsl_manager.list_distributions().unwrap_or_default();
    let wsl_items: Vec<ListItem> = distros
        .iter()
        .map(|d| {
            let marker = if d.is_default { "* " } else { "  " };
            let state_color = match d.state {
                crate::wsl::manager::WslState::Running => Color::Green,
                crate::wsl::manager::WslState::Stopped => Color::Red,
                _ => Color::Yellow,
            };
            ListItem::new(Line::from(vec![
                Span::raw(marker),
                Span::styled(&d.name, Style::default().fg(Color::White)),
                Span::raw(" ["),
                Span::styled(d.state.to_string(), Style::default().fg(state_color)),
                Span::raw(format!("] WSL{}", d.version)),
            ]))
        })
        .collect();

    let wsl_list = List::new(wsl_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" WSL Distributions ")
            .title_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(wsl_list, left_chunks[1]);

    // Right panel: quick actions + sessions
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    let actions = vec![
        Line::from(Span::styled(
            "Quick Actions",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [1] ", Style::default().fg(Color::Cyan)),
            Span::raw("New SSH Connection"),
        ]),
        Line::from(vec![
            Span::styled("  [2] ", Style::default().fg(Color::Cyan)),
            Span::raw("Open WSL Terminal"),
        ]),
        Line::from(vec![
            Span::styled("  [3] ", Style::default().fg(Color::Cyan)),
            Span::raw("File Transfer (SFTP)"),
        ]),
        Line::from(vec![
            Span::styled("  [4] ", Style::default().fg(Color::Cyan)),
            Span::raw("Create SSH Tunnel"),
        ]),
        Line::from(vec![
            Span::styled("  [5] ", Style::default().fg(Color::Cyan)),
            Span::raw("System Information"),
        ]),
    ];

    let actions_widget = Paragraph::new(actions).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Actions ")
            .title_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(actions_widget, right_chunks[0]);

    // Recent sessions
    let session_items: Vec<ListItem> = app
        .session_manager
        .sessions
        .iter()
        .take(10)
        .map(|s| {
            let type_str = match &s.session_type {
                crate::core::session::SessionType::Ssh(ssh) => {
                    format!("SSH {}@{}", ssh.username, ssh.host)
                }
                crate::core::session::SessionType::Wsl(wsl) => {
                    format!("WSL {}", wsl.distribution)
                }
                crate::core::session::SessionType::Local => "Local".to_string(),
            };
            ListItem::new(Line::from(vec![
                Span::styled(&s.name, Style::default().fg(Color::White)),
                Span::raw(" - "),
                Span::styled(type_str, Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let sessions_widget = List::new(session_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Recent Sessions ")
            .title_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(sessions_widget, right_chunks[1]);
}

fn draw_wsl(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left: distribution list
    let distros = app.wsl_manager.list_distributions().unwrap_or_default();
    let items: Vec<ListItem> = distros
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let style = if i == app.wsl_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let state_icon = match d.state {
                crate::wsl::manager::WslState::Running => "●",
                crate::wsl::manager::WslState::Stopped => "○",
                _ => "◌",
            };
            ListItem::new(Line::from(vec![
                Span::raw(format!(" {} ", state_icon)),
                Span::styled(format!("{} (WSL{})", d.name, d.version), style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Distributions ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, chunks[0]);

    // Right: command output + input
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(chunks[1]);

    let output_lines: Vec<Line> = app
        .command_output
        .iter()
        .map(|l| {
            if l.starts_with('$') {
                Line::from(Span::styled(l.as_str(), Style::default().fg(Color::Green)))
            } else if l.starts_with("Error:") {
                Line::from(Span::styled(l.as_str(), Style::default().fg(Color::Red)))
            } else {
                Line::from(l.as_str())
            }
        })
        .collect();

    let output = Paragraph::new(output_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Terminal Output ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(output, right_chunks[0]);

    let input_style = if app.input_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };
    let input = Paragraph::new(Line::from(vec![
        Span::styled("$ ", Style::default().fg(Color::Green)),
        Span::styled(app.command_input.as_str(), input_style),
        Span::styled(if app.input_mode { "█" } else { "" }, input_style),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(if app.input_mode {
                " Input (Esc to exit) "
            } else {
                " Press 'i' to type "
            }),
    );
    frame.render_widget(input, right_chunks[1]);
}

fn draw_ssh(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(5)])
        .split(area);

    let fields = [
        ("Host", &app.ssh_form.host),
        ("Port", &app.ssh_form.port),
        ("Username", &app.ssh_form.username),
        ("Password", &app.ssh_form.password),
    ];

    let rows: Vec<Row> = fields
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let style = if i == app.ssh_form.focused_field {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let display_value = if *label == "Password" && !value.is_empty() {
                "*".repeat(value.len())
            } else {
                value.to_string()
            };
            let cursor = if i == app.ssh_form.focused_field && app.input_mode {
                "█"
            } else {
                ""
            };
            Row::new(vec![
                Cell::from(Span::styled(format!("  {}: ", label), style)),
                Cell::from(Span::styled(
                    format!("{}{}", display_value, cursor),
                    style,
                )),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(15), Constraint::Min(30)])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" New SSH Connection (i=edit, Tab=next, Enter=connect) ")
                .title_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(table, chunks[0]);

    // SSH sessions list
    let ssh_sessions: Vec<ListItem> = app
        .session_manager
        .list_by_type("ssh")
        .iter()
        .map(|s| {
            if let crate::core::session::SessionType::Ssh(ssh) = &s.session_type {
                ListItem::new(Line::from(vec![
                    Span::styled(&s.name, Style::default().fg(Color::White)),
                    Span::raw(format!(" - {}@{}:{}", ssh.username, ssh.host, ssh.port)),
                ]))
            } else {
                ListItem::new("")
            }
        })
        .collect();

    let list = List::new(ssh_sessions).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Saved SSH Sessions ")
            .title_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(list, chunks[1]);
}

fn draw_sftp(frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let local = Paragraph::new(vec![
        Line::from(Span::styled(
            "  Local Files",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Connect to a server via SSH tab first"),
        Line::from("  then browse files here."),
        Line::from(""),
        Line::from(Span::styled(
            "  Drag & drop or Enter to transfer",
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Local ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(local, chunks[0]);

    let remote = Paragraph::new(vec![
        Line::from(Span::styled(
            "  Remote Files",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  No active SSH connection."),
        Line::from("  Use SSH tab to connect first."),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Remote ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(remote, chunks[1]);
}

fn draw_tunnels(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Active tunnels
    let tunnel_list = app.tunnel_manager.list_tunnels();
    let items: Vec<ListItem> = if tunnel_list.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No active tunnels. Press 'n' to create one.",
            Style::default().fg(Color::Gray),
        ))]
    } else {
        tunnel_list
            .iter()
            .map(|t| {
                ListItem::new(Line::from(vec![
                    Span::styled("  ● ", Style::default().fg(Color::Green)),
                    Span::styled(&t.name, Style::default().fg(Color::White)),
                    Span::raw(format!(
                        " {} localhost:{} -> {}:{}",
                        t.tunnel_type, t.local_port, t.remote_host, t.remote_port
                    )),
                ]))
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Active Tunnels ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, chunks[0]);

    // Tunnel help
    let help = Paragraph::new(vec![
        Line::from(Span::styled(
            "  SSH Tunnel Types",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Local (-L):   ", Style::default().fg(Color::Cyan)),
            Span::raw("Forward local port to remote destination"),
        ]),
        Line::from(vec![
            Span::styled("  Remote (-R):  ", Style::default().fg(Color::Cyan)),
            Span::raw("Forward remote port to local destination"),
        ]),
        Line::from(vec![
            Span::styled("  Dynamic (-D): ", Style::default().fg(Color::Cyan)),
            Span::raw("SOCKS proxy through SSH server"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press 'n' to create a new tunnel",
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .title_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(help, chunks[1]);
}

fn draw_sessions(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .session_manager
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == app.session_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let type_badge = match &s.session_type {
                crate::core::session::SessionType::Ssh(_) => ("SSH", Color::Blue),
                crate::core::session::SessionType::Wsl(_) => ("WSL", Color::Green),
                crate::core::session::SessionType::Local => ("LOC", Color::Yellow),
            };
            let detail = match &s.session_type {
                crate::core::session::SessionType::Ssh(ssh) => {
                    format!("{}@{}:{}", ssh.username, ssh.host, ssh.port)
                }
                crate::core::session::SessionType::Wsl(wsl) => wsl.distribution.clone(),
                crate::core::session::SessionType::Local => "local shell".to_string(),
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" [{}] ", type_badge.0),
                    Style::default().fg(type_badge.1).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&s.name, style),
                Span::styled(format!("  {}", detail), Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let list = if items.is_empty() {
        List::new(vec![ListItem::new(Span::styled(
            "  No saved sessions. Create one from SSH or WSL tabs.",
            Style::default().fg(Color::Gray),
        ))])
    } else {
        List::new(items)
    }
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Saved Sessions (Enter=connect, d=delete, n=new) ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, area);
}

fn draw_sysinfo(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Min(5),
        ])
        .split(area);

    let info = app.sys_info.gather();

    // System overview
    let mem_pct = if info.total_memory_mb > 0 {
        (info.used_memory_mb as f64 / info.total_memory_mb as f64) * 100.0
    } else {
        0.0
    };
    let mem_bar_width = 30;
    let filled = (mem_pct / 100.0 * mem_bar_width as f64) as usize;
    let mem_bar = format!(
        "[{}{}] {:.0}%",
        "█".repeat(filled),
        "░".repeat(mem_bar_width - filled),
        mem_pct
    );

    let sys_text = vec![
        Line::from(vec![
            Span::styled("  Hostname:  ", Style::default().fg(Color::Cyan)),
            Span::raw(&info.hostname),
        ]),
        Line::from(vec![
            Span::styled("  OS:        ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} {}", info.os_name, info.os_version)),
        ]),
        Line::from(vec![
            Span::styled("  Kernel:    ", Style::default().fg(Color::Cyan)),
            Span::raw(&info.kernel_version),
        ]),
        Line::from(vec![
            Span::styled("  CPU:       ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} cores, {:.1}% usage", info.cpu_count, info.cpu_usage)),
        ]),
        Line::from(vec![
            Span::styled("  Memory:    ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{} / {} MB  {}",
                info.used_memory_mb, info.total_memory_mb, mem_bar
            )),
        ]),
        Line::from(vec![
            Span::styled("  Swap:      ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} / {} MB", info.used_swap_mb, info.total_swap_mb)),
        ]),
        Line::from(vec![
            Span::styled("  Uptime:    ", Style::default().fg(Color::Cyan)),
            Span::raw(format_uptime(info.uptime_secs)),
        ]),
    ];

    let overview = Paragraph::new(sys_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" System Information ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(overview, chunks[0]);

    // Network interfaces
    let networks = app.sys_info.get_networks();
    let net_rows: Vec<Row> = networks
        .iter()
        .map(|n| {
            Row::new(vec![
                Cell::from(Span::styled(
                    format!("  {}", n.interface),
                    Style::default().fg(Color::White),
                )),
                Cell::from(format_bytes(n.received_bytes)),
                Cell::from(format_bytes(n.transmitted_bytes)),
            ])
        })
        .collect();

    let net_table = Table::new(
        net_rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ],
    )
    .header(
        Row::new(vec![
            Cell::from(Span::styled(
                "  Interface",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Received",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Transmitted",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Network Interfaces ")
            .title_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(net_table, chunks[1]);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            " LAGIDE ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(&app.status_message, Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(
            " Tab/←→: Navigate | ?: Help | q: Quit ",
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, area);
}

fn draw_help_popup(frame: &mut Frame, _app: &App) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            "  Lagide Keyboard Shortcuts",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Global", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  Tab / → / l    Next tab"),
        Line::from("  Shift+Tab / ← / h  Previous tab"),
        Line::from("  1-7            Jump to tab"),
        Line::from("  ?              Toggle help"),
        Line::from("  q              Quit"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  WSL Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  j/k or ↑/↓     Select distribution"),
        Line::from("  i              Enter input mode"),
        Line::from("  Enter          Execute command"),
        Line::from("  Esc            Exit input mode"),
        Line::from("  s              Start distribution"),
        Line::from("  x              Stop distribution"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  SSH Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  i              Edit fields"),
        Line::from("  Tab            Next field"),
        Line::from("  Enter          Connect"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Sessions", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  j/k or ↑/↓     Select session"),
        Line::from("  Enter          Connect to session"),
        Line::from("  d              Delete session"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help (? to close) ")
                .title_style(Style::default().fg(Color::Yellow))
                .style(Style::default().bg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(help, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
