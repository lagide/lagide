use crate::core::config::AppConfig;
use crate::core::session::SessionManager;
use crate::sysinfo_mod::SystemInfo;
use crate::terminal::TerminalEmulator;
use crate::tunnel::forward::{TunnelConfig, TunnelType};
use crate::tunnel::TunnelManager;
use crate::wsl::WslManager;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Dashboard,
    Wsl,
    Ssh,
    Sftp,
    Tunnels,
    Sessions,
    SysInfo,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Dashboard,
            Tab::Wsl,
            Tab::Ssh,
            Tab::Sftp,
            Tab::Tunnels,
            Tab::Sessions,
            Tab::SysInfo,
        ]
    }

    pub fn title(&self) -> &str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Wsl => "WSL2",
            Tab::Ssh => "SSH",
            Tab::Sftp => "SFTP",
            Tab::Tunnels => "Tunnels",
            Tab::Sessions => "Sessions",
            Tab::SysInfo => "System",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Dashboard => 0,
            Tab::Wsl => 1,
            Tab::Ssh => 2,
            Tab::Sftp => 3,
            Tab::Tunnels => 4,
            Tab::Sessions => 5,
            Tab::SysInfo => 6,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SshFormState {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub focused_field: usize,
}

impl Default for SshFormState {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: "22".to_string(),
            username: String::new(),
            password: String::new(),
            focused_field: 0,
        }
    }
}

/// State for the interactive tunnel creation form
#[derive(Debug, Clone)]
pub struct TunnelFormState {
    pub name: String,
    pub tunnel_type: TunnelTypeChoice,
    pub local_port: String,
    pub remote_host: String,
    pub remote_port: String,
    pub ssh_host: String,
    pub ssh_port: String,
    pub ssh_user: String,
    pub focused_field: usize,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TunnelTypeChoice {
    Local,
    Remote,
    Dynamic,
}

impl TunnelTypeChoice {
    pub fn label(&self) -> &str {
        match self {
            TunnelTypeChoice::Local => "Local (-L)",
            TunnelTypeChoice::Remote => "Remote (-R)",
            TunnelTypeChoice::Dynamic => "Dynamic (-D)",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            TunnelTypeChoice::Local => TunnelTypeChoice::Remote,
            TunnelTypeChoice::Remote => TunnelTypeChoice::Dynamic,
            TunnelTypeChoice::Dynamic => TunnelTypeChoice::Local,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            TunnelTypeChoice::Local => TunnelTypeChoice::Dynamic,
            TunnelTypeChoice::Remote => TunnelTypeChoice::Local,
            TunnelTypeChoice::Dynamic => TunnelTypeChoice::Remote,
        }
    }
}

impl Default for TunnelFormState {
    fn default() -> Self {
        Self {
            name: String::new(),
            tunnel_type: TunnelTypeChoice::Local,
            local_port: "8080".to_string(),
            remote_host: "localhost".to_string(),
            remote_port: "80".to_string(),
            ssh_host: String::new(),
            ssh_port: "22".to_string(),
            ssh_user: String::new(),
            focused_field: 0,
            active: false,
        }
    }
}

/// Represents an active terminal tab
pub struct TerminalTab {
    pub emulator: TerminalEmulator,
    pub title: String,
}

/// Which input focus mode we are in
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,      // Form editing (SSH, tunnel forms, command input)
    Terminal,     // Full terminal passthrough mode
    TunnelForm,   // Tunnel creation form
}

pub struct App {
    pub running: bool,
    pub active_tab: Tab,
    pub config: AppConfig,
    pub session_manager: SessionManager,
    pub wsl_manager: WslManager,
    pub tunnel_manager: TunnelManager,
    pub sys_info: SystemInfo,
    pub status_message: String,
    pub wsl_selected: usize,
    pub session_selected: usize,
    pub ssh_form: SshFormState,
    pub tunnel_form: TunnelFormState,
    pub command_input: String,
    pub command_output: Vec<String>,
    pub input_mode: InputMode,
    pub show_help: bool,
    // Terminal emulator tabs
    pub terminal_tabs: Vec<TerminalTab>,
    pub active_terminal: Option<usize>,
    pub tunnel_selected: usize,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let config = AppConfig::load().unwrap_or_default();
        let session_manager =
            SessionManager::load(config.sessions_file.clone()).unwrap_or_else(|_| {
                SessionManager::new(config.sessions_file.clone())
            });

        Ok(Self {
            running: true,
            active_tab: Tab::Dashboard,
            config,
            session_manager,
            wsl_manager: WslManager::new(),
            tunnel_manager: TunnelManager::new(),
            sys_info: SystemInfo::new(),
            status_message: "Welcome to Lagide - Press ? for help".to_string(),
            wsl_selected: 0,
            session_selected: 0,
            ssh_form: SshFormState::default(),
            tunnel_form: TunnelFormState::default(),
            command_input: String::new(),
            command_output: Vec::new(),
            input_mode: InputMode::Normal,
            show_help: false,
            terminal_tabs: Vec::new(),
            active_terminal: None,
            tunnel_selected: 0,
        })
    }

    pub fn next_tab(&mut self) {
        let tabs = Tab::all();
        let idx = self.active_tab.index();
        self.active_tab = tabs[(idx + 1) % tabs.len()];
    }

    pub fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let idx = self.active_tab.index();
        self.active_tab = if idx == 0 {
            tabs[tabs.len() - 1]
        } else {
            tabs[idx - 1]
        };
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn exec_wsl_command(&mut self) {
        if self.command_input.is_empty() {
            return;
        }

        let distros = self.wsl_manager.list_distributions().unwrap_or_default();
        if let Some(distro) = distros.get(self.wsl_selected) {
            match self
                .wsl_manager
                .exec_command(&distro.name, &self.command_input)
            {
                Ok(output) => {
                    self.command_output
                        .push(format!("$ {}", self.command_input));
                    for line in output.lines() {
                        self.command_output.push(line.to_string());
                    }
                    self.status_message = "Command executed successfully".to_string();
                }
                Err(e) => {
                    self.command_output.push(format!("Error: {}", e));
                    self.status_message = format!("Error: {}", e);
                }
            }
        }
        self.command_input.clear();
    }

    /// Open a local shell terminal
    pub fn open_local_terminal(&mut self, rows: u16, cols: u16) {
        match TerminalEmulator::spawn_shell(rows, cols) {
            Ok(emu) => {
                let title = emu.title.clone();
                self.terminal_tabs.push(TerminalTab {
                    emulator: emu,
                    title: title.clone(),
                });
                self.active_terminal = Some(self.terminal_tabs.len() - 1);
                self.input_mode = InputMode::Terminal;
                self.status_message = format!("Opened terminal: {}", title);
            }
            Err(e) => {
                self.status_message = format!("Failed to open terminal: {}", e);
            }
        }
    }

    /// Open a WSL terminal
    pub fn open_wsl_terminal(&mut self, distro: &str, rows: u16, cols: u16) {
        match TerminalEmulator::spawn_wsl(distro, rows, cols) {
            Ok(emu) => {
                let title = emu.title.clone();
                self.terminal_tabs.push(TerminalTab {
                    emulator: emu,
                    title: title.clone(),
                });
                self.active_terminal = Some(self.terminal_tabs.len() - 1);
                self.input_mode = InputMode::Terminal;
                self.status_message = format!("Opened: {}", title);
            }
            Err(e) => {
                self.status_message = format!("Failed to open WSL terminal: {}", e);
            }
        }
    }

    /// Open an SSH terminal
    pub fn open_ssh_terminal(&mut self, host: &str, port: u16, username: &str, rows: u16, cols: u16) {
        match TerminalEmulator::spawn_ssh(host, port, username, rows, cols) {
            Ok(emu) => {
                let title = emu.title.clone();
                self.terminal_tabs.push(TerminalTab {
                    emulator: emu,
                    title: title.clone(),
                });
                self.active_terminal = Some(self.terminal_tabs.len() - 1);
                self.input_mode = InputMode::Terminal;
                self.status_message = format!("Opened: {}", title);
            }
            Err(e) => {
                self.status_message = format!("Failed to open SSH terminal: {}", e);
            }
        }
    }

    /// Close the active terminal
    pub fn close_active_terminal(&mut self) {
        if let Some(idx) = self.active_terminal {
            if idx < self.terminal_tabs.len() {
                let title = self.terminal_tabs[idx].title.clone();
                self.terminal_tabs.remove(idx);
                self.status_message = format!("Closed: {}", title);

                if self.terminal_tabs.is_empty() {
                    self.active_terminal = None;
                    self.input_mode = InputMode::Normal;
                } else {
                    self.active_terminal = Some(idx.min(self.terminal_tabs.len() - 1));
                }
            }
        }
    }

    /// Create a tunnel from the form state
    pub fn create_tunnel_from_form(&mut self) {
        let tunnel_type = match self.tunnel_form.tunnel_type {
            TunnelTypeChoice::Local => TunnelType::Local,
            TunnelTypeChoice::Remote => TunnelType::Remote,
            TunnelTypeChoice::Dynamic => TunnelType::Dynamic,
        };

        let local_port = self.tunnel_form.local_port.parse().unwrap_or(8080);
        let remote_port = self.tunnel_form.remote_port.parse().unwrap_or(80);
        let ssh_port = self.tunnel_form.ssh_port.parse().unwrap_or(22);

        let name = if self.tunnel_form.name.is_empty() {
            format!(
                "{} :{}->{}:{}",
                tunnel_type, local_port, self.tunnel_form.remote_host, remote_port
            )
        } else {
            self.tunnel_form.name.clone()
        };

        let config = TunnelConfig {
            name: name.clone(),
            tunnel_type,
            local_port,
            remote_host: self.tunnel_form.remote_host.clone(),
            remote_port,
            ssh_host: self.tunnel_form.ssh_host.clone(),
            ssh_port,
            ssh_user: self.tunnel_form.ssh_user.clone(),
        };

        match self.tunnel_manager.create_tunnel(config) {
            Ok(()) => {
                self.status_message = format!("Tunnel '{}' created successfully", name);
                self.tunnel_form = TunnelFormState::default();
            }
            Err(e) => {
                self.status_message = format!("Failed to create tunnel: {}", e);
            }
        }
    }

    /// Stop a tunnel by index
    pub fn stop_selected_tunnel(&mut self) {
        let tunnels = self.tunnel_manager.list_tunnels();
        if let Some(t) = tunnels.get(self.tunnel_selected) {
            let name = t.name.clone();
            match self.tunnel_manager.stop_tunnel(&name) {
                Ok(()) => {
                    self.status_message = format!("Tunnel '{}' stopped", name);
                    if self.tunnel_selected > 0 {
                        self.tunnel_selected -= 1;
                    }
                }
                Err(e) => {
                    self.status_message = format!("Failed to stop tunnel: {}", e);
                }
            }
        }
    }
}
