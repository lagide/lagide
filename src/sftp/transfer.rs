use crate::core::error::{LagideError, Result};
use ssh2::Session;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RemoteFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub permissions: String,
    pub modified: Option<String>,
}

pub struct SftpManager<'a> {
    session: &'a Session,
}

impl<'a> SftpManager<'a> {
    pub fn new(session: &'a Session) -> Result<Self> {
        Ok(Self { session })
    }

    /// List files in a remote directory
    pub fn list_dir(&self, path: &str) -> Result<Vec<RemoteFile>> {
        let sftp = self
            .session
            .sftp()
            .map_err(|e| LagideError::Sftp(format!("SFTP init failed: {}", e)))?;

        let entries = sftp
            .readdir(Path::new(path))
            .map_err(|e| LagideError::Sftp(format!("Failed to list directory: {}", e)))?;

        let files = entries
            .into_iter()
            .map(|(path_buf, stat)| {
                let name = path_buf
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let full_path = path_buf.to_string_lossy().to_string();

                RemoteFile {
                    name,
                    path: full_path,
                    size: stat.size.unwrap_or(0),
                    is_dir: stat.is_dir(),
                    permissions: format!("{:o}", stat.perm.unwrap_or(0) & 0o777),
                    modified: stat.mtime.map(|t| {
                        chrono::DateTime::from_timestamp(t as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_default()
                    }),
                }
            })
            .collect();

        Ok(files)
    }

    /// Download a file from the remote server
    pub fn download(&self, remote_path: &str, local_path: &str) -> Result<u64> {
        let sftp = self
            .session
            .sftp()
            .map_err(|e| LagideError::Sftp(format!("SFTP init failed: {}", e)))?;

        let mut remote_file = sftp
            .open(Path::new(remote_path))
            .map_err(|e| LagideError::Sftp(format!("Failed to open remote file: {}", e)))?;

        let mut local_file = std::fs::File::create(local_path)?;
        let mut buffer = [0u8; 8192];
        let mut total = 0u64;

        loop {
            let n = remote_file
                .read(&mut buffer)
                .map_err(|e| LagideError::Sftp(format!("Read error: {}", e)))?;
            if n == 0 {
                break;
            }
            local_file
                .write_all(&buffer[..n])
                .map_err(|e| LagideError::Sftp(format!("Write error: {}", e)))?;
            total += n as u64;
        }

        Ok(total)
    }

    /// Upload a file to the remote server
    pub fn upload(&self, local_path: &str, remote_path: &str) -> Result<u64> {
        let sftp = self
            .session
            .sftp()
            .map_err(|e| LagideError::Sftp(format!("SFTP init failed: {}", e)))?;

        let mut local_file = std::fs::File::open(local_path)?;
        let mut remote_file = sftp
            .create(Path::new(remote_path))
            .map_err(|e| LagideError::Sftp(format!("Failed to create remote file: {}", e)))?;

        let mut buffer = [0u8; 8192];
        let mut total = 0u64;

        loop {
            let n = local_file
                .read(&mut buffer)
                .map_err(|e| LagideError::Sftp(format!("Read error: {}", e)))?;
            if n == 0 {
                break;
            }
            remote_file
                .write_all(&buffer[..n])
                .map_err(|e| LagideError::Sftp(format!("Write error: {}", e)))?;
            total += n as u64;
        }

        Ok(total)
    }

    /// Create a remote directory
    pub fn mkdir(&self, path: &str) -> Result<()> {
        let sftp = self
            .session
            .sftp()
            .map_err(|e| LagideError::Sftp(format!("SFTP init failed: {}", e)))?;

        sftp.mkdir(Path::new(path), 0o755)
            .map_err(|e| LagideError::Sftp(format!("Failed to create directory: {}", e)))?;

        Ok(())
    }

    /// Remove a remote file
    pub fn remove(&self, path: &str) -> Result<()> {
        let sftp = self
            .session
            .sftp()
            .map_err(|e| LagideError::Sftp(format!("SFTP init failed: {}", e)))?;

        sftp.unlink(Path::new(path))
            .map_err(|e| LagideError::Sftp(format!("Failed to remove file: {}", e)))?;

        Ok(())
    }

    /// Get file info
    pub fn stat(&self, path: &str) -> Result<RemoteFile> {
        let sftp = self
            .session
            .sftp()
            .map_err(|e| LagideError::Sftp(format!("SFTP init failed: {}", e)))?;

        let stat = sftp
            .stat(Path::new(path))
            .map_err(|e| LagideError::Sftp(format!("Failed to stat file: {}", e)))?;

        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(RemoteFile {
            name,
            path: path.to_string(),
            size: stat.size.unwrap_or(0),
            is_dir: stat.is_dir(),
            permissions: format!("{:o}", stat.perm.unwrap_or(0) & 0o777),
            modified: stat.mtime.map(|t| {
                chrono::DateTime::from_timestamp(t as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default()
            }),
        })
    }
}
