//! Inter-process communication between TUI and daemon

use std::path::PathBuf;

/// IPC socket path
pub fn socket_path() -> PathBuf {
    dirs::runtime_dir()
        .or_else(dirs::data_dir)
        .map(|d| d.join("hazelnut.sock"))
        .unwrap_or_else(|| PathBuf::from("/tmp/hazelnut.sock"))
}

/// Messages from TUI to daemon
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

// TODO: Implement actual IPC using Unix sockets or named pipes
