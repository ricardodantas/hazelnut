//! Inter-process communication between TUI and daemon
//!
//! Uses Unix domain sockets for communication. Messages are serialized
//! as JSON with a newline delimiter.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// IPC socket path
pub fn socket_path() -> PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::data_dir)
        .map(|d| d.join("hazelnut.sock"))
        .unwrap_or_else(|| {
            #[cfg(unix)]
            {
                let uid = crate::current_uid();
                PathBuf::from(format!("/tmp/hazelnut-{}.sock", uid))
            }
            #[cfg(not(unix))]
            {
                PathBuf::from("/tmp/hazelnut.sock")
            }
        })
}

/// Messages from TUI to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonCommand {
    /// Get current status
    Status,

    /// Reload configuration
    Reload,

    /// Stop the daemon
    Stop,

    /// Get activity log
    GetLog { limit: usize },

    /// Get statistics
    GetStats,
}

/// Messages from daemon to TUI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    /// Status information
    Status {
        running: bool,
        uptime_seconds: u64,
        watches: usize,
        rules: usize,
        files_processed: u64,
    },

    /// Log entries
    Log { entries: Vec<String> },

    /// Acknowledgment
    Ok,

    /// Error
    Error { message: String },
}

/// Send a command to the daemon and receive a response.
///
/// Connects to the Unix socket, sends a JSON-encoded command,
/// and reads back a JSON-encoded response.
#[cfg(unix)]
pub fn send_command(cmd: &DaemonCommand) -> Result<DaemonResponse> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let path = socket_path();
    let stream = UnixStream::connect(&path)
        .with_context(|| format!("Failed to connect to daemon at {}", path.display()))?;

    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut stream_write = stream.try_clone()?;
    let mut line = serde_json::to_string(cmd)?;
    line.push('\n');
    stream_write.write_all(line.as_bytes())?;
    stream_write.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .context("Failed to read daemon response")?;

    serde_json::from_str(&response_line).context("Failed to parse daemon response")
}

#[cfg(not(unix))]
pub fn send_command(_cmd: &DaemonCommand) -> Result<DaemonResponse> {
    anyhow::bail!("IPC is only supported on Unix platforms")
}

/// Check if the daemon is running by probing the socket.
pub fn is_daemon_running() -> bool {
    #[cfg(unix)]
    {
        send_command(&DaemonCommand::Status).is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}
