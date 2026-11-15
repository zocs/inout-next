use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub version: String,
    pub application_id: String,
    pub repository: String,
}

impl Default for Product {
    fn default() -> Self {
        serde_json::from_str(include_str!("../../../product.json")).expect("valid product manifest")
    }
}

impl Product {
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        serde_json::from_slice(&std::fs::read(path).map_err(|_| Error::Io)?)
            .map_err(|_| Error::Conflict)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelativePath(String);

impl RelativePath {
    pub fn parse(value: &str, allow_root: bool) -> Result<Self, Error> {
        if value.is_empty() {
            return if allow_root {
                Ok(Self(String::new()))
            } else {
                Err(Error::InvalidPath)
            };
        }
        for component in value.split('/') {
            let stem = component
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || stem
                    .strip_prefix("COM")
                    .or_else(|| stem.strip_prefix("LPT"))
                    .is_some_and(|suffix| {
                        matches!(
                            suffix,
                            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                        )
                    });
            if component.is_empty()
                || component == "."
                || component == ".."
                || component.ends_with(['.', ' '])
                || component.to_ascii_lowercase().starts_with(".inout-stage-")
                || component.eq_ignore_ascii_case("INOUTTMP")
                // DOS short-name aliases contain a tilde followed by a digit.
                // Conservatively reserve that shape across all platforms.
                || component.as_bytes().windows(2).any(|pair| pair[0] == b'~' && pair[1].is_ascii_digit())
                || component.chars().any(|c| {
                    c.is_control() || matches!(c, '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')
                })
                || reserved
            {
                return Err(Error::InvalidPath);
            }
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions {
    pub list: bool,
    pub download: bool,
    pub upload: bool,
    pub manage: bool,
    #[serde(default)]
    pub rename: bool,
    #[serde(default)]
    pub delete: bool,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("Invalid relative path")]
    InvalidPath,
    #[error("Authentication required")]
    Unauthorized,
    #[error("Operation is forbidden")]
    Forbidden,
    #[error("Entry not found")]
    NotFound,
    #[error("Destination already exists")]
    TargetExists,
    #[error("Upload exceeds the configured limit")]
    TooLarge,
    #[error("Directory is not empty")]
    NotEmpty,
    #[error("Operation is unsupported")]
    Unsupported,
    #[error("Range is not satisfiable")]
    InvalidRange,
    #[error("Operation conflicts with current state")]
    Conflict,
    #[error("Storage operation failed")]
    Io,
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidPath => "invalid_path",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::TargetExists => "target_exists",
            Self::TooLarge => "too_large",
            Self::NotEmpty => "not_empty",
            Self::Unsupported => "unsupported",
            Self::InvalidRange => "invalid_range",
            Self::Conflict => "conflict",
            Self::Io => "io_error",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModuleManifest {
    pub id: &'static str,
    pub dependencies: &'static [&'static str],
}

pub trait FeatureModule: Send + Sync {
    fn manifest(&self) -> ModuleManifest;
}

pub struct FilesModule;
pub struct UploadsModule;
pub struct ManageModule;

impl FeatureModule for FilesModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: "files",
            dependencies: &[],
        }
    }
}
impl FeatureModule for UploadsModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: "uploads",
            dependencies: &["files"],
        }
    }
}
impl FeatureModule for ManageModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            id: "manage",
            dependencies: &["files"],
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModuleRegistry {
    enabled: Vec<String>,
}

impl ModuleRegistry {
    pub fn new(requested: &[String]) -> Result<Self, Error> {
        let available: Vec<Box<dyn FeatureModule>> = vec![
            Box::new(FilesModule),
            Box::new(UploadsModule),
            Box::new(ManageModule),
        ];
        if requested
            .iter()
            .any(|id| !available.iter().any(|module| module.manifest().id == id))
        {
            return Err(Error::Unsupported);
        }
        let mut enabled = Vec::new();
        for module in available {
            let manifest = module.manifest();
            if requested.iter().any(|id| id == manifest.id) {
                if manifest
                    .dependencies
                    .iter()
                    .any(|dep| !requested.iter().any(|id| id == dep))
                {
                    return Err(Error::Conflict);
                }
                enabled.push(manifest.id.to_owned());
            }
        }
        Ok(Self { enabled })
    }
    pub fn enabled(&self) -> &[String] {
        &self.enabled
    }
    pub fn has(&self, id: &str) -> bool {
        self.enabled.iter().any(|item| item == id)
    }
    pub fn permissions(&self, writable: bool) -> Permissions {
        Permissions {
            list: self.has("files"),
            download: self.has("files"),
            upload: writable && self.has("uploads"),
            manage: writable && self.has("manage"),
            rename: writable && self.has("manage"),
            delete: writable && self.has("manage"),
        }
    }
}
