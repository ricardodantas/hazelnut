//! Rule actions - what to do with matched files

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tracing::{debug, info};

#[cfg(unix)]
use libc;

/// Pre-compiled regex for `{date:FORMAT}` patterns.
static DATE_FORMAT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{date:([^}]+)\}").expect("invalid date format regex"));

/// Action to perform on a matched file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    /// Move file to a destination folder
    Move {
        destination: PathBuf,
        /// Create destination if it doesn't exist
        #[serde(default = "default_true")]
        create_destination: bool,
        /// Overwrite if file exists
        #[serde(default)]
        overwrite: bool,
    },

    /// Copy file to a destination folder
    Copy {
        destination: PathBuf,
        #[serde(default = "default_true")]
        create_destination: bool,
        #[serde(default)]
        overwrite: bool,
    },

    /// Rename the file
    Rename {
        /// New name pattern (supports {name}, {ext}, {date}, etc.)
        pattern: String,
    },

    /// Move to trash
    Trash,

    /// Delete permanently (use with caution!)
    Delete,

    /// Run a shell command
    Run {
        command: String,
        /// Arguments (supports {path}, {name}, {dir}, etc.)
        #[serde(default)]
        args: Vec<String>,
    },

    /// Archive the file (zip)
    Archive {
        /// Destination for the archive
        destination: Option<PathBuf>,
        /// Delete original after archiving
        #[serde(default)]
        delete_original: bool,
    },

    /// Do nothing (useful for testing conditions)
    Nothing,
}

fn default_true() -> bool {
    true
}

impl Action {
    /// Execute this action on a file
    pub fn execute(&self, path: &Path) -> Result<()> {
        match self {
            Action::Move {
                destination,
                create_destination,
                overwrite,
            } => {
                let dest = expand_path(destination);

                if *create_destination {
                    std::fs::create_dir_all(&dest).with_context(|| {
                        format!("Failed to create directory: {}", dest.display())
                    })?;
                }

                let filename = path.file_name().context("File has no name")?;
                let dest_path = dest.join(filename);

                if dest_path.exists() && !overwrite {
                    anyhow::bail!(
                        "Destination exists and overwrite is false: {}",
                        dest_path.display()
                    );
                }

                info!("Moving {} -> {}", path.display(), dest_path.display());
                if std::fs::rename(path, &dest_path).is_err() {
                    // rename fails across filesystems; fall back to copy + remove
                    if path.is_dir() {
                        copy_dir_recursive(path, &dest_path).with_context(|| {
                            format!(
                                "Failed to copy directory {} to {}",
                                path.display(),
                                dest_path.display()
                            )
                        })?;
                        std::fs::remove_dir_all(path).with_context(|| {
                            format!("Failed to remove original directory {}", path.display())
                        })?;
                    } else {
                        std::fs::copy(path, &dest_path).with_context(|| {
                            format!(
                                "Failed to copy {} to {}",
                                path.display(),
                                dest_path.display()
                            )
                        })?;
                        std::fs::remove_file(path).with_context(|| {
                            format!("Failed to remove original file {}", path.display())
                        })?;
                    }
                }
            }

            Action::Copy {
                destination,
                create_destination,
                overwrite,
            } => {
                let dest = expand_path(destination);

                if *create_destination {
                    std::fs::create_dir_all(&dest)?;
                }

                let filename = path.file_name().context("File has no name")?;
                let dest_path = dest.join(filename);

                if dest_path.exists() && !overwrite {
                    anyhow::bail!(
                        "Destination exists and overwrite is false: {}",
                        dest_path.display()
                    );
                }

                info!("Copying {} -> {}", path.display(), dest_path.display());
                std::fs::copy(path, &dest_path)?;
            }

            Action::Rename { pattern } => {
                let new_name = expand_pattern(pattern, path)?;
                let new_path = path.parent().unwrap_or(Path::new(".")).join(&new_name);

                info!("Renaming {} -> {}", path.display(), new_path.display());
                std::fs::rename(path, &new_path)?;
            }

            Action::Trash => {
                info!("Trashing {}", path.display());
                // Use the `trash` crate for proper .trashinfo / Finder integration.
                // Fall back to a manual move if the crate fails (e.g. headless CI).
                if let Err(e) = trash::delete(path) {
                    debug!("trash crate failed ({}), falling back to manual move", e);
                    let trash_dir = dirs::data_dir()
                        .map(|d| d.join("Trash").join("files"))
                        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share/Trash/files")))
                        .unwrap_or_else(|| PathBuf::from("/tmp/trash"));

                    std::fs::create_dir_all(&trash_dir)?;

                    let filename = path.file_name().context("File has no name")?;
                    let mut trash_path = trash_dir.join(filename);

                    if trash_path.exists() {
                        let stem = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let ext = path
                            .extension()
                            .map(|e| format!(".{}", e.to_string_lossy()))
                            .unwrap_or_default();
                        let mut counter = 1u32;
                        loop {
                            if counter > 10000 {
                                anyhow::bail!(
                                    "Too many duplicate files in trash for: {}",
                                    path.display()
                                );
                            }
                            trash_path = trash_dir.join(format!("{}_{}{}", stem, counter, ext));
                            if !trash_path.exists() {
                                break;
                            }
                            counter += 1;
                        }
                    }

                    if std::fs::rename(path, &trash_path).is_err() {
                        std::fs::copy(path, &trash_path)?;
                        std::fs::remove_file(path)?;
                    }
                }
            }

            Action::Delete => {
                info!("Deleting {}", path.display());
                if path.is_dir() {
                    std::fs::remove_dir_all(path)?;
                } else {
                    std::fs::remove_file(path)?;
                }
            }

            Action::Run { command, args } => {
                // Check if command contains shell operators - if so, run through shell
                let has_shell_operators = command.contains("&&")
                    || command.contains("||")
                    || command.contains(';')
                    || command.contains('|')
                    || command.contains('>')
                    || command.contains('<');

                if has_shell_operators && args.is_empty() {
                    // Run through shell
                    let shell = if cfg!(target_os = "windows") {
                        "cmd"
                    } else {
                        "sh"
                    };
                    let shell_arg = if cfg!(target_os = "windows") {
                        "/C"
                    } else {
                        "-c"
                    };

                    // Expand {path} patterns in the command, shell-escaping values
                    let expanded_command = expand_pattern_shell_escaped(command, path)
                        .unwrap_or_else(|_| command.clone());

                    info!("Running (shell): {}", expanded_command);

                    let mut child = std::process::Command::new(shell)
                        .arg(shell_arg)
                        .arg(&expanded_command)
                        .spawn()
                        .with_context(|| {
                            format!("Failed to run shell command: {}", expanded_command)
                        })?;

                    // Grab the PID before moving child into the wait thread, so we
                    // can kill the process on timeout (the thread owns the Child).
                    let child_pid = child.id();

                    // Wait with a 60-second timeout using a channel
                    let timeout = std::time::Duration::from_secs(60);
                    let (tx, rx) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        let result = child.wait();
                        let _ = tx.send(result);
                    });

                    let status = match rx.recv_timeout(timeout) {
                        Ok(Ok(status)) => status,
                        Ok(Err(e)) => return Err(e.into()),
                        Err(_) => {
                            // Timeout — kill the orphaned child process via its PID
                            // since ownership was moved into the wait thread.
                            #[cfg(unix)]
                            unsafe {
                                if let Ok(pid) = i32::try_from(child_pid) {
                                    libc::kill(pid, libc::SIGKILL);
                                }
                            }
                            let err_msg = "timed out after 60s";
                            crate::notifications::notify_command_error(&expanded_command, err_msg);
                            anyhow::bail!("Command timed out after 60s: {}", expanded_command);
                        }
                    };

                    if !status.success() {
                        let err_msg = format!("exited with status {}", status);
                        crate::notifications::notify_command_error(&expanded_command, &err_msg);
                        anyhow::bail!("Command failed with status: {}", status);
                    }
                } else {
                    // Direct command execution
                    // If args is empty and command contains spaces, split it
                    let (actual_command, base_args): (&str, Vec<&str>) =
                        if args.is_empty() && command.contains(' ') {
                            let parts: Vec<&str> = command.split_whitespace().collect();
                            if parts.is_empty() {
                                anyhow::bail!("Empty command");
                            }
                            (parts[0], parts[1..].to_vec())
                        } else {
                            (command.as_str(), vec![])
                        };

                    let mut expanded_args: Vec<String> =
                        base_args.iter().map(|s| s.to_string()).collect();
                    expanded_args.extend(
                        args.iter()
                            .map(|a| expand_pattern(a, path).unwrap_or_else(|_| a.clone())),
                    );

                    info!("Running: {} {:?}", actual_command, expanded_args);

                    let mut child = std::process::Command::new(actual_command)
                        .args(&expanded_args)
                        .spawn()
                        .with_context(|| format!("Failed to run command: {}", actual_command))?;

                    // Grab PID before moving child into wait thread (see shell branch above).
                    let child_pid = child.id();

                    // Wait with a 60-second timeout using a channel
                    let timeout = std::time::Duration::from_secs(60);
                    let (tx, rx) = std::sync::mpsc::channel();
                    let actual_command_owned = actual_command.to_string();
                    std::thread::spawn(move || {
                        let result = child.wait();
                        let _ = tx.send(result);
                    });

                    let status = match rx.recv_timeout(timeout) {
                        Ok(Ok(status)) => status,
                        Ok(Err(e)) => return Err(e.into()),
                        Err(_) => {
                            #[cfg(unix)]
                            unsafe {
                                if let Ok(pid) = i32::try_from(child_pid) {
                                    libc::kill(pid, libc::SIGKILL);
                                }
                            }
                            let err_msg = "timed out after 60s";
                            crate::notifications::notify_command_error(
                                &actual_command_owned,
                                err_msg,
                            );
                            anyhow::bail!("Command timed out after 60s: {}", actual_command_owned);
                        }
                    };

                    if !status.success() {
                        let err_msg = format!("exited with status {}", status);
                        crate::notifications::notify_command_error(actual_command, &err_msg);
                        anyhow::bail!("Command failed with status: {}", status);
                    }
                }
            }

            Action::Archive {
                destination,
                delete_original,
            } => {
                let dest = destination
                    .as_ref()
                    .map(|p| expand_path(p))
                    .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).to_path_buf());

                let filename = path.file_stem().context("File has no name")?;
                let archive_name = format!("{}.zip", filename.to_string_lossy());
                let archive_path = dest.join(&archive_name);

                info!("Archiving {} -> {}", path.display(), archive_path.display());

                // Create the zip archive
                let zip_file = std::fs::File::create(&archive_path)?;
                let mut zip = zip::ZipWriter::new(zip_file);
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);

                if path.is_dir() {
                    // Recursively add all files in the directory
                    fn add_dir_to_zip(
                        zip: &mut zip::ZipWriter<std::fs::File>,
                        dir: &Path,
                        base: &Path,
                        options: zip::write::SimpleFileOptions,
                    ) -> Result<()> {
                        for entry in std::fs::read_dir(dir)? {
                            let entry = entry?;
                            let entry_path = entry.path();
                            let relative = entry_path
                                .strip_prefix(base)
                                .unwrap_or(&entry_path)
                                .to_string_lossy();
                            if entry_path.is_dir() {
                                zip.add_directory(format!("{}/", relative), options)?;
                                add_dir_to_zip(zip, &entry_path, base, options)?;
                            } else {
                                zip.start_file(relative.as_ref(), options)?;
                                let mut source = std::fs::File::open(&entry_path)?;
                                std::io::copy(&mut source, zip)?;
                            }
                        }
                        Ok(())
                    }
                    add_dir_to_zip(
                        &mut zip,
                        path,
                        path.parent().unwrap_or(Path::new(".")),
                        options,
                    )?;
                } else {
                    let file_name = path
                        .file_name()
                        .context("File has no name")?
                        .to_string_lossy();
                    zip.start_file(file_name.as_ref(), options)?;
                    let mut source = std::fs::File::open(path)?;
                    std::io::copy(&mut source, &mut zip)?;
                }
                zip.finish()?;

                info!("Created archive: {}", archive_path.display());

                if *delete_original {
                    if path.is_dir() {
                        std::fs::remove_dir_all(path)?;
                    } else {
                        std::fs::remove_file(path)?;
                    }
                }
            }

            Action::Nothing => {
                debug!("No action for {}", path.display());
            }
        }

        Ok(())
    }
}

/// Recursively copy a directory tree from `src` to `dst`.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_child = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_child)?;
        } else {
            std::fs::copy(entry.path(), &dest_child)?;
        }
    }
    Ok(())
}

/// Expand ~ and environment variables in a path
fn expand_path(path: &Path) -> PathBuf {
    crate::expand_path(path)
}

/// Internal pattern expansion with optional shell escaping of path-derived values.
fn expand_pattern_inner(pattern: &str, path: &Path, shell_escape: bool) -> Result<String> {
    let mut result = pattern.to_string();

    let maybe_escape = |s: std::borrow::Cow<'_, str>| -> String {
        if shell_escape {
            shell_escape::escape(s).to_string()
        } else {
            s.to_string()
        }
    };

    // {path} - full path
    result = result.replace("{path}", &maybe_escape(path.to_string_lossy()));

    // {dir} - parent directory
    if let Some(parent) = path.parent() {
        result = result.replace("{dir}", &maybe_escape(parent.to_string_lossy()));
    }

    // {name} - filename without extension
    if let Some(stem) = path.file_stem() {
        result = result.replace("{name}", &maybe_escape(stem.to_string_lossy()));
    }

    // {filename} - full filename with extension
    if let Some(filename) = path.file_name() {
        result = result.replace("{filename}", &maybe_escape(filename.to_string_lossy()));
    }

    // {ext} - extension
    if let Some(ext) = path.extension() {
        result = result.replace("{ext}", &maybe_escape(ext.to_string_lossy()));
    } else {
        result = result.replace("{ext}", "");
    }

    // {date} - current date
    let now = chrono::Local::now();
    result = result.replace("{date}", &now.format("%Y-%m-%d").to_string());
    result = result.replace("{datetime}", &now.format("%Y-%m-%d_%H-%M-%S").to_string());

    // {date:FORMAT} - custom date format
    result = DATE_FORMAT_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let format = &caps[1];
            now.format(format).to_string()
        })
        .to_string();

    Ok(result)
}

/// Expand pattern variables like {name}, {ext}, {date}
fn expand_pattern(pattern: &str, path: &Path) -> Result<String> {
    expand_pattern_inner(pattern, path, false)
}

/// Expand pattern variables with shell-escaped values (for use in shell commands)
fn expand_pattern_shell_escaped(pattern: &str, path: &Path) -> Result<String> {
    expand_pattern_inner(pattern, path, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_pattern() {
        let path = Path::new("/tmp/test.pdf");

        assert_eq!(expand_pattern("{name}", path).unwrap(), "test");
        assert_eq!(expand_pattern("{ext}", path).unwrap(), "pdf");
        assert_eq!(expand_pattern("{filename}", path).unwrap(), "test.pdf");
        assert_eq!(expand_pattern("{name}.{ext}", path).unwrap(), "test.pdf");
    }

    #[test]
    fn test_expand_path() {
        // This test depends on the home directory existing
        let path = Path::new("~/Downloads");
        let expanded = expand_path(path);
        assert!(!expanded.to_string_lossy().contains('~'));
    }
}
