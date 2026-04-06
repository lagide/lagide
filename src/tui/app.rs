use crate::core::config::AppConfig;
use crate::core::session::SessionManager;
use crate::sysinfo_mod::SystemInfo;
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
    pub command_input: String,
    pub command_output: Vec<String>,
    pub input_mode: bool,
    pub show_help: bool,
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
            command_input: String::new(),
            command_output: Vec::new(),
            input_mode: false,
            show_help: false,
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
            match self.wsl_manager.exec_command(&distro.name, &self.command_input) {
                Ok(output) => {
                    self.command_output
                        .push(format!("$ {}", self.command_input));
                    for line in output.lines() {
                        self.command_output.push(line.to_string());
                    }
                    self.status_message = "Command executed successfully".to_string();
                }
                Err(e) => {
                    self.command_output
                        .push(format!("Error: {}", e));
                    self.status_message = format!("Error: {}", e);
                }
            }
        }
        self.command_input.clear();
    }
}
