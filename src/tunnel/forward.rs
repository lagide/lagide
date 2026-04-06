use crate::core::error::{LagideError, Result};
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub name: String,
    pub tunnel_type: TunnelType,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TunnelType {
    Local,   // -L: local port -> remote
    Remote,  // -R: remote port -> local
    Dynamic, // -D: SOCKS proxy
}

impl std::fmt::Display for TunnelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TunnelType::Local => write!(f, "Local (-L)"),
            TunnelType::Remote => write!(f, "Remote (-R)"),
            TunnelType::Dynamic => write!(f, "Dynamic (-D)"),
        }
    }
}

pub struct ActiveTunnel {
    pub config: TunnelConfig,
    pub process: Child,
}

pub struct TunnelManager {
    pub tunnels: Vec<ActiveTunnel>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: Vec::new(),
        }
    }

    /// Create and start a new SSH tunnel
    pub fn create_tunnel(&mut self, config: TunnelConfig) -> Result<()> {
        let tunnel_arg = match config.tunnel_type {
            TunnelType::Local => format!(
                "-L {}:{}:{}",
                config.local_port, config.remote_host, config.remote_port
            ),
            TunnelType::Remote => format!(
                "-R {}:{}:{}",
                config.remote_port, config.remote_host, config.local_port
            ),
            TunnelType::Dynamic => format!("-D {}", config.local_port),
        };

        let child = Command::new("ssh")
            .args([
                "-N",
                "-f",
                &tunnel_arg,
                "-p",
                &config.ssh_port.to_string(),
                &format!("{}@{}", config.ssh_user, config.ssh_host),
            ])
            .spawn()
            .map_err(|e| LagideError::Tunnel(format!("Failed to create tunnel: {}", e)))?;

        self.tunnels.push(ActiveTunnel {
            config,
            process: child,
        });

        Ok(())
    }

    /// Stop a tunnel by name
    pub fn stop_tunnel(&mut self, name: &str) -> Result<()> {
        if let Some(pos) = self.tunnels.iter().position(|t| t.config.name == name) {
            let mut tunnel = self.tunnels.remove(pos);
            tunnel
                .process
                .kill()
                .map_err(|e| LagideError::Tunnel(format!("Failed to stop tunnel: {}", e)))?;
            Ok(())
        } else {
            Err(LagideError::Tunnel(format!("Tunnel '{}' not found", name)))
        }
    }

    /// Stop all tunnels
    pub fn stop_all(&mut self) {
        for tunnel in &mut self.tunnels {
            let _ = tunnel.process.kill();
        }
        self.tunnels.clear();
    }

    /// List active tunnels
    pub fn list_tunnels(&self) -> Vec<&TunnelConfig> {
        self.tunnels.iter().map(|t| &t.config).collect()
    }

    /// Check if a tunnel is still running
    pub fn is_alive(&mut self, name: &str) -> bool {
        if let Some(tunnel) = self.tunnels.iter_mut().find(|t| t.config.name == name) {
            tunnel.process.try_wait().ok().flatten().is_none()
        } else {
            false
        }
    }
}

impl Drop for TunnelManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}
