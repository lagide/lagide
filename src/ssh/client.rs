use crate::core::error::{LagideError, Result};
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;

pub struct SshClient {
    session: Option<Session>,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl SshClient {
    pub fn new(host: &str, port: u16, username: &str) -> Self {
        Self {
            session: None,
            host: host.to_string(),
            port,
            username: username.to_string(),
            connected: false,
        }
    }

    /// Connect using password authentication
    pub fn connect_password(&mut self, password: &str) -> Result<()> {
        let tcp = TcpStream::connect(format!("{}:{}", self.host, self.port))
            .map_err(|e| LagideError::Ssh(format!("Connection failed: {}", e)))?;

        let mut session = Session::new()
            .map_err(|e| LagideError::Ssh(format!("Session creation failed: {}", e)))?;

        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| LagideError::Ssh(format!("Handshake failed: {}", e)))?;

        session
            .userauth_password(&self.username, password)
            .map_err(|e| LagideError::Ssh(format!("Authentication failed: {}", e)))?;

        if !session.authenticated() {
            return Err(LagideError::Ssh("Authentication failed".into()));
        }

        self.session = Some(session);
        self.connected = true;
        Ok(())
    }

    /// Connect using SSH key authentication
    pub fn connect_key(&mut self, key_path: &Path, passphrase: Option<&str>) -> Result<()> {
        let tcp = TcpStream::connect(format!("{}:{}", self.host, self.port))
            .map_err(|e| LagideError::Ssh(format!("Connection failed: {}", e)))?;

        let mut session = Session::new()
            .map_err(|e| LagideError::Ssh(format!("Session creation failed: {}", e)))?;

        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| LagideError::Ssh(format!("Handshake failed: {}", e)))?;

        session
            .userauth_pubkey_file(&self.username, None, key_path, passphrase)
            .map_err(|e| LagideError::Ssh(format!("Key authentication failed: {}", e)))?;

        if !session.authenticated() {
            return Err(LagideError::Ssh("Key authentication failed".into()));
        }

        self.session = Some(session);
        self.connected = true;
        Ok(())
    }

    /// Execute a command on the remote server
    pub fn exec(&self, command: &str) -> Result<CommandOutput> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| LagideError::Ssh("Not connected".into()))?;

        let mut channel = session
            .channel_session()
            .map_err(|e| LagideError::Ssh(format!("Channel creation failed: {}", e)))?;

        channel
            .exec(command)
            .map_err(|e| LagideError::Ssh(format!("Command execution failed: {}", e)))?;

        let mut stdout = String::new();
        channel
            .read_to_string(&mut stdout)
            .map_err(|e| LagideError::Ssh(format!("Read stdout failed: {}", e)))?;

        let mut stderr = String::new();
        channel
            .stderr()
            .read_to_string(&mut stderr)
            .map_err(|e| LagideError::Ssh(format!("Read stderr failed: {}", e)))?;

        channel.wait_close().ok();
        let exit_code = channel.exit_status().unwrap_or(-1);

        Ok(CommandOutput {
            stdout,
            stderr,
            exit_code,
        })
    }

    /// Get the underlying session for SFTP operations
    pub fn session(&self) -> Result<&Session> {
        self.session
            .as_ref()
            .ok_or_else(|| LagideError::Ssh("Not connected".into()))
    }

    /// Disconnect from the server
    pub fn disconnect(&mut self) {
        if let Some(ref session) = self.session {
            let _ = session.disconnect(None, "Goodbye", None);
        }
        self.session = None;
        self.connected = false;
    }

    /// Get server fingerprint
    pub fn fingerprint(&self) -> Result<String> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| LagideError::Ssh("Not connected".into()))?;

        let hash = session.host_key_hash(ssh2::HashType::Sha256);
        match hash {
            Some(bytes) => Ok(bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(":")),
            None => Ok("unknown".to_string()),
        }
    }
}

impl Drop for SshClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}
