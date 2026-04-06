use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub default_shell: String,
    pub ssh_key_path: PathBuf,
    pub sessions_file: PathBuf,
    pub theme: Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub primary_color: String,
    pub accent_color: String,
    pub background: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lagide");

        Self {
            default_shell: "bash".to_string(),
            ssh_key_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ssh")
                .join("id_rsa"),
            sessions_file: config_dir.join("sessions.json"),
            theme: Theme::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary_color: "#61AFEF".to_string(),
            accent_color: "#98C379".to_string(),
            background: "#282C34".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lagide")
            .join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = AppConfig::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lagide");

        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}
