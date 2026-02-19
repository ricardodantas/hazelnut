//! Rule conditions - matching files based on attributes

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// Simple thread-local caches for compiled patterns
std::thread_local! {
    static GLOB_CACHE: std::cell::RefCell<HashMap<String, glob::Pattern>> = std::cell::RefCell::new(HashMap::new());
    static REGEX_CACHE: std::cell::RefCell<HashMap<String, Regex>> = std::cell::RefCell::new(HashMap::new());
}

/// Conditions for matching files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Condition {
    /// Match file extension (without dot, e.g., "pdf")
    #[serde(default)]
    pub extension: Option<String>,

    /// Match file extensions (any of these)
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Match filename with glob pattern
    #[serde(default)]
    pub name_matches: Option<String>,

    /// Match filename with regex
    #[serde(default)]
    pub name_regex: Option<String>,

    /// File size greater than (in bytes)
    #[serde(default)]
    pub size_greater_than: Option<u64>,

    /// File size less than (in bytes)
    #[serde(default)]
    pub size_less_than: Option<u64>,

    /// File age greater than (in days)
    #[serde(default)]
    pub age_days_greater_than: Option<u64>,

    /// File age less than (in days)
    #[serde(default)]
    pub age_days_less_than: Option<u64>,

    /// File is a directory
    #[serde(default)]
    pub is_directory: Option<bool>,

    /// File is hidden (starts with .)
    #[serde(default)]
    pub is_hidden: Option<bool>,
}

impl Condition {
    /// Check if a file matches this condition
    pub fn matches(&self, path: &Path) -> Result<bool> {
        // Check extension
        if let Some(ref ext) = self.extension
            && !check_extension(path, ext)
        {
            return Ok(false);
        }

        // Check extensions list
        if !self.extensions.is_empty() {
            let matches_any = self.extensions.iter().any(|ext| check_extension(path, ext));
            if !matches_any {
                return Ok(false);
            }
        }

        // Check name glob pattern
        if let Some(ref pattern) = self.name_matches
            && !check_glob(path, pattern)?
        {
            return Ok(false);
        }

        // Check name regex
        if let Some(ref pattern) = self.name_regex
            && !check_regex(path, pattern)?
        {
            return Ok(false);
        }

        // Check file size
        if (self.size_greater_than.is_some() || self.size_less_than.is_some())
            && let Ok(metadata) = path.metadata()
        {
            let size = metadata.len();

            if let Some(min) = self.size_greater_than
                && size <= min
            {
                return Ok(false);
            }

            if let Some(max) = self.size_less_than
                && size >= max
            {
                return Ok(false);
            }
        }

        // Check file age
        if (self.age_days_greater_than.is_some() || self.age_days_less_than.is_some())
            && let Ok(metadata) = path.metadata()
            && let Ok(modified) = metadata.modified()
        {
            let age = modified.elapsed().map(|d| d.as_secs() / 86400).unwrap_or(0);

            if let Some(min_days) = self.age_days_greater_than
                && age <= min_days
            {
                return Ok(false);
            }

            if let Some(max_days) = self.age_days_less_than
                && age >= max_days
            {
                return Ok(false);
            }
        }

        // Check if directory
        if let Some(is_dir) = self.is_directory
            && path.is_dir() != is_dir
        {
            return Ok(false);
        }

        // Check if hidden
        if let Some(is_hidden) = self.is_hidden {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let actually_hidden = filename.starts_with('.');
            if actually_hidden != is_hidden {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

fn check_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}

fn check_glob(path: &Path, pattern: &str) -> Result<bool> {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    GLOB_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let glob_pattern = if let Some(p) = cache.get(pattern) {
            p.clone()
        } else {
            let p = glob::Pattern::new(pattern)?;
            cache.insert(pattern.to_string(), p.clone());
            p
        };
        Ok(glob_pattern.matches(filename))
    })
}

fn check_regex(path: &Path, pattern: &str) -> Result<bool> {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    REGEX_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let regex = if let Some(r) = cache.get(pattern) {
            r.clone()
        } else {
            let r = Regex::new(pattern)?;
            cache.insert(pattern.to_string(), r.clone());
            r
        };
        Ok(regex.is_match(filename))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_match() {
        let condition = Condition {
            extension: Some("pdf".to_string()),
            ..Default::default()
        };

        assert!(condition.matches(Path::new("/tmp/test.pdf")).unwrap());
        assert!(condition.matches(Path::new("/tmp/test.PDF")).unwrap());
        assert!(!condition.matches(Path::new("/tmp/test.txt")).unwrap());
    }

    #[test]
    fn test_glob_match() {
        let condition = Condition {
            name_matches: Some("Screenshot*.png".to_string()),
            ..Default::default()
        };

        assert!(
            condition
                .matches(Path::new("/tmp/Screenshot 2024-01-01.png"))
                .unwrap()
        );
        assert!(!condition.matches(Path::new("/tmp/photo.png")).unwrap());
    }

    #[test]
    fn test_hidden_match() {
        let condition = Condition {
            is_hidden: Some(true),
            ..Default::default()
        };

        assert!(condition.matches(Path::new("/tmp/.hidden")).unwrap());
        assert!(!condition.matches(Path::new("/tmp/visible")).unwrap());
    }
}
