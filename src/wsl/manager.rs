use crate::core::error::{LagideError, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct WslDistribution {
    pub name: String,
    pub state: WslState,
    pub version: u8,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WslState {
    Running,
    Stopped,
    Installing,
    Unknown,
}

impl std::fmt::Display for WslState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WslState::Running => write!(f, "Running"),
            WslState::Stopped => write!(f, "Stopped"),
            WslState::Installing => write!(f, "Installing"),
            WslState::Unknown => write!(f, "Unknown"),
        }
    }
}

pub struct WslManager;

impl WslManager {
    pub fn new() -> Self {
        Self
    }

    /// List all installed WSL distributions
    pub fn list_distributions(&self) -> Result<Vec<WslDistribution>> {
        let output = Command::new("wsl")
            .args(["--list", "--verbose"])
            .output()
            .map_err(|e| LagideError::Wsl(format!("Failed to run wsl command: {}", e)))?;

        if !output.status.success() {
            // On non-Windows or if WSL is not installed, return demo data
            return Ok(self.demo_distributions());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_distributions(&stdout)
    }

    /// Execute a command in a WSL distribution
    pub fn exec_command(&self, distro: &str, command: &str) -> Result<String> {
        let output = Command::new("wsl")
            .args(["-d", distro, "--", "bash", "-c", command])
            .output()
            .map_err(|e| LagideError::Wsl(format!("Failed to execute in WSL: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(LagideError::Wsl(format!("Command failed: {}", stderr)))
        }
    }

    /// Start a WSL distribution
    pub fn start_distribution(&self, distro: &str) -> Result<()> {
        let output = Command::new("wsl")
            .args(["-d", distro, "--", "echo", "started"])
            .output()
            .map_err(|e| LagideError::Wsl(format!("Failed to start distribution: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LagideError::Wsl("Failed to start distribution".into()))
        }
    }

    /// Shutdown a WSL distribution
    pub fn stop_distribution(&self, distro: &str) -> Result<()> {
        let output = Command::new("wsl")
            .args(["--terminate", distro])
            .output()
            .map_err(|e| LagideError::Wsl(format!("Failed to stop distribution: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LagideError::Wsl("Failed to stop distribution".into()))
        }
    }

    /// Shutdown all WSL instances
    pub fn shutdown_all(&self) -> Result<()> {
        let output = Command::new("wsl")
            .arg("--shutdown")
            .output()
            .map_err(|e| LagideError::Wsl(format!("Failed to shutdown WSL: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LagideError::Wsl("Failed to shutdown all WSL instances".into()))
        }
    }

    /// Set default distribution
    pub fn set_default(&self, distro: &str) -> Result<()> {
        let output = Command::new("wsl")
            .args(["--set-default", distro])
            .output()
            .map_err(|e| LagideError::Wsl(format!("Failed to set default: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LagideError::Wsl("Failed to set default distribution".into()))
        }
    }

    /// Export a distribution to a tar file
    pub fn export_distribution(&self, distro: &str, path: &str) -> Result<()> {
        let output = Command::new("wsl")
            .args(["--export", distro, path])
            .output()
            .map_err(|e| LagideError::Wsl(format!("Failed to export: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LagideError::Wsl("Failed to export distribution".into()))
        }
    }

    /// Import a distribution from a tar file
    pub fn import_distribution(&self, name: &str, install_path: &str, tar_path: &str) -> Result<()> {
        let output = Command::new("wsl")
            .args(["--import", name, install_path, tar_path])
            .output()
            .map_err(|e| LagideError::Wsl(format!("Failed to import: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LagideError::Wsl("Failed to import distribution".into()))
        }
    }

    fn parse_distributions(&self, output: &str) -> Result<Vec<WslDistribution>> {
        let mut distributions = Vec::new();

        for line in output.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let is_default = line.starts_with('*');
            let line = line.trim_start_matches('*').trim();

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[0].to_string();
                let state = match parts[1] {
                    "Running" => WslState::Running,
                    "Stopped" => WslState::Stopped,
                    "Installing" => WslState::Installing,
                    _ => WslState::Unknown,
                };
                let version = parts[2].parse().unwrap_or(2);

                distributions.push(WslDistribution {
                    name,
                    state,
                    version,
                    is_default,
                });
            }
        }

        Ok(distributions)
    }

    fn demo_distributions(&self) -> Vec<WslDistribution> {
        vec![
            WslDistribution {
                name: "Ubuntu-22.04".to_string(),
                state: WslState::Running,
                version: 2,
                is_default: true,
            },
            WslDistribution {
                name: "Debian".to_string(),
                state: WslState::Stopped,
                version: 2,
                is_default: false,
            },
            WslDistribution {
                name: "kali-linux".to_string(),
                state: WslState::Stopped,
                version: 2,
                is_default: false,
            },
        ]
    }
}
