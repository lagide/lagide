use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    Ssh(SshSession),
    Wsl(WslSession),
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub session_type: SessionType,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshSession {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Password,
    Key(PathBuf),
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslSession {
    pub distribution: String,
    pub default_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManager {
    pub sessions: Vec<Session>,
    file_path: PathBuf,
}

impl SessionManager {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            sessions: Vec::new(),
            file_path,
        }
    }

    pub fn load(file_path: PathBuf) -> anyhow::Result<Self> {
        if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)?;
            let sessions: Vec<Session> = serde_json::from_str(&content)?;
            Ok(Self {
                sessions,
                file_path,
            })
        } else {
            Ok(Self::new(file_path))
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.sessions)?;
        std::fs::write(&self.file_path, content)?;
        Ok(())
    }

    pub fn add_session(&mut self, session: Session) {
        self.sessions.push(session);
    }

    pub fn remove_session(&mut self, id: &str) -> Option<Session> {
        if let Some(pos) = self.sessions.iter().position(|s| s.id == id) {
            Some(self.sessions.remove(pos))
        } else {
            None
        }
    }

    pub fn get_session(&self, id: &str) -> Option<&Session> {
        self.sessions.iter().find(|s| s.id == id)
    }

    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }

    pub fn list_by_type(&self, filter: &str) -> Vec<&Session> {
        self.sessions
            .iter()
            .filter(|s| match (&s.session_type, filter) {
                (SessionType::Ssh(_), "ssh") => true,
                (SessionType::Wsl(_), "wsl") => true,
                (SessionType::Local, "local") => true,
                (_, "all") => true,
                _ => false,
            })
            .collect()
    }
}
