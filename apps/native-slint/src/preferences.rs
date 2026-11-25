//! Local presentation and setup preferences. The access key is never stored.
use crate::presentation::{Access, Locale};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

const FILE: &str = "native-preferences.conf";
const MAX_BYTES: u64 = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    /// `None` follows the system language.
    pub locale: Option<Locale>,
    pub accent: i32,
    /// 0 follows the system, 1 is light, 2 is dark.
    pub appearance: i32,
    pub folder: String,
    pub port: String,
    pub network: bool,
    pub access: Access,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            locale: None,
            accent: 0,
            appearance: 0,
            folder: String::new(),
            port: "5000".into(),
            network: true,
            access: Access::ReadOnly,
        }
    }
}

fn single_line(value: &str) -> bool {
    !value.chars().any(char::is_control)
}

impl Preferences {
    /// Unknown keys and invalid values keep their defaults.
    pub fn parse(text: &str) -> Self {
        let mut preferences = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "locale" => {
                    preferences.locale = match value {
                        "en" => Some(Locale::English),
                        "zh-Hans" => Some(Locale::Simplified),
                        "zh-Hant" => Some(Locale::Traditional),
                        _ => None,
                    }
                }
                "accent" => {
                    if let Some(accent) = value.parse().ok().filter(|index| (0..4).contains(index))
                    {
                        preferences.accent = accent;
                    }
                }
                "appearance" => {
                    if let Some(mode) = value.parse().ok().filter(|index| (0..3).contains(index)) {
                        preferences.appearance = mode;
                    }
                }
                "folder" => preferences.folder = value.to_owned(),
                "port" => {
                    if crate::presentation::parse_port(value).is_ok() {
                        preferences.port = value.trim().to_owned();
                    }
                }
                "network" => preferences.network = value != "false",
                "access" => {
                    preferences.access = match value {
                        "upload" => Access::Upload,
                        "manage" => Access::Manage,
                        _ => Access::ReadOnly,
                    }
                }
                _ => {}
            }
        }
        preferences
    }

    pub fn serialize(&self) -> String {
        let mut text = String::new();
        if let Some(locale) = self.locale {
            text.push_str(match locale {
                Locale::English => "locale=en\n",
                Locale::Simplified => "locale=zh-Hans\n",
                Locale::Traditional => "locale=zh-Hant\n",
            });
        }
        text.push_str(&format!("accent={}\n", self.accent.clamp(0, 3)));
        text.push_str(&format!("appearance={}\n", self.appearance.clamp(0, 2)));
        if !self.folder.is_empty() && single_line(&self.folder) {
            text.push_str(&format!("folder={}\n", self.folder));
        }
        if crate::presentation::parse_port(&self.port).is_ok() {
            text.push_str(&format!("port={}\n", self.port.trim()));
        }
        text.push_str(&format!("network={}\n", self.network));
        text.push_str(match self.access {
            Access::ReadOnly => "access=readonly\n",
            Access::Upload => "access=upload\n",
            Access::Manage => "access=manage\n",
        });
        text
    }

    /// A missing, oversized or unreadable file yields the defaults.
    pub fn load(path: &Path) -> Self {
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_BYTES => {
                std::fs::read_to_string(path)
                    .map(|text| Self::parse(&text))
                    .unwrap_or_default()
            }
            _ => Self::default(),
        }
    }

    /// Publish one complete setup from an exclusively created sibling file.
    /// Concurrent instances may replace each other's complete documents, but
    /// never share a staged file or remove another instance's work.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let directory = path
            .parent()
            .ok_or_else(|| io::Error::other("preferences need a directory"))?;
        std::fs::create_dir_all(directory)?;
        let mut staged = tempfile::NamedTempFile::new_in(directory)?;
        staged.write_all(self.serialize().as_bytes())?;
        staged.flush()?;
        staged
            .persist(path)
            .map(|_| ())
            .map_err(|error| error.error)
    }
}

/// The per-user file for this application ID. `INOUT_NATIVE_PREFERENCES`
/// overrides the location; an empty value disables saved preferences.
pub fn location(application_id: &str) -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("INOUT_NATIVE_PREFERENCES") {
        return (!explicit.is_empty()).then(|| PathBuf::from(explicit));
    }
    config_directory().map(|directory| directory.join(application_id).join(FILE))
}

fn absolute(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn config_directory() -> Option<PathBuf> {
    if cfg!(windows) {
        absolute("APPDATA")
    } else if cfg!(target_os = "macos") {
        crate::folders::home().map(|home| home.join("Library").join("Application Support"))
    } else {
        absolute("XDG_CONFIG_HOME")
            .or_else(|| crate::folders::home().map(|home| home.join(".config")))
    }
}
