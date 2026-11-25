//! Typed-path expansion and the user's common folders.
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommonFolder {
    Home,
    Desktop,
    Documents,
    Downloads,
}

pub fn home() -> Option<PathBuf> {
    std::env::home_dir().filter(|path| path.is_absolute())
}

/// Expand a leading `~` typed by the user. Other input is returned unchanged;
/// `~name` forms are not interpreted.
pub fn expand(input: &str, home: Option<&Path>) -> PathBuf {
    let trimmed = input.trim();
    let Some(home) = home else {
        return PathBuf::from(trimmed);
    };
    if trimmed == "~" {
        return home.to_path_buf();
    }
    for separator in ["~/", "~\\"] {
        if let Some(rest) = trimmed.strip_prefix(separator) {
            return home.join(rest.trim_start_matches(['/', '\\']));
        }
    }
    PathBuf::from(trimmed)
}

/// Read `XDG_*_DIR="$HOME/…"` lines from a freedesktop `user-dirs.dirs` file,
/// where desktops record localized folder names.
pub fn parse_user_dirs(contents: &str, home: &Path) -> Vec<(CommonFolder, PathBuf)> {
    let mut found = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let folder = match key.trim() {
            "XDG_DESKTOP_DIR" => CommonFolder::Desktop,
            "XDG_DOCUMENTS_DIR" => CommonFolder::Documents,
            "XDG_DOWNLOAD_DIR" => CommonFolder::Downloads,
            _ => continue,
        };
        let value = value.trim();
        let Some(value) = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        else {
            continue;
        };
        let path = if let Some(rest) = value.strip_prefix("$HOME/") {
            home.join(rest)
        } else if value.starts_with('/') {
            PathBuf::from(value)
        } else {
            continue;
        };
        // A disabled entry points at the home directory itself.
        if path != home {
            found.push((folder, path));
        }
    }
    found
}

/// Existing common folders, home first. Localized desktop configuration wins
/// over the conventional English names.
pub fn common(home: &Path, user_dirs: Option<&str>) -> Vec<(CommonFolder, PathBuf)> {
    let configured = user_dirs
        .map(|contents| parse_user_dirs(contents, home))
        .unwrap_or_default();
    let mut result = Vec::new();
    if home.is_dir() {
        result.push((CommonFolder::Home, home.to_path_buf()));
    }
    for (folder, conventional) in [
        (CommonFolder::Desktop, "Desktop"),
        (CommonFolder::Documents, "Documents"),
        (CommonFolder::Downloads, "Downloads"),
    ] {
        let path = configured
            .iter()
            .find(|(kind, _)| *kind == folder)
            .map(|(_, path)| path.clone())
            .filter(|path| path.is_dir())
            .unwrap_or_else(|| home.join(conventional));
        if path.is_dir() {
            result.push((folder, path));
        }
    }
    result
}

/// The current user's common folders.
pub fn detect() -> Vec<(CommonFolder, PathBuf)> {
    let Some(home) = home() else {
        return Vec::new();
    };
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    let user_dirs = std::fs::read_to_string(config.join("user-dirs.dirs")).ok();
    common(&home, user_dirs.as_deref())
}
