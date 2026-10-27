use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub const IMAGE_FORMATS: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tif", ".tiff", ".webp", ".svg", ".ico", ".arw",
    ".cr2", ".cr3", ".crw", ".dng", ".nef", ".orf", ".raf", ".rw2",
];
pub const ARCHIVE_FORMATS: &[&str] = &[".zip", ".rar", ".7z"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub initialized: bool,
    pub enabled_formats: BTreeMap<String, bool>,
    pub context_files: bool,
    pub context_archives: bool,
    pub context_folders: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mut enabled_formats = BTreeMap::new();
        for extension in IMAGE_FORMATS {
            enabled_formats.insert((*extension).to_string(), true);
        }
        Self {
            initialized: false,
            enabled_formats,
            context_files: true,
            context_archives: true,
            context_folders: true,
        }
    }
}

pub fn settings_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Widno")
        .join("settings.json")
}

impl Settings {
    pub fn load() -> (Self, PathBuf) {
        let path = settings_path();
        let settings = fs::read_to_string(&path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default();
        (settings, path)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(path, contents)
    }
}

#[cfg(windows)]
pub fn apply_default_associations(settings: &Settings) -> std::io::Result<()> {
    for extension in IMAGE_FORMATS {
        if !settings
            .enabled_formats
            .get(*extension)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }
        let user_choice = format!(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{}\UserChoice",
            extension
        );
        let claimed = std::process::Command::new("reg")
            .args(["query", &user_choice])
            .status()?
            .success();
        if !claimed {
            let class_key = format!(r"HKCU\Software\Classes\{}", extension);
            let status = std::process::Command::new("reg")
                .args(["add", &class_key, "/ve", "/d", "Widno.ImageFile", "/f"])
                .status()?;
            if !status.success() {
                return Err(std::io::Error::other(format!(
                    "reg.exe failed for {extension}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn apply_default_associations(_settings: &Settings) -> std::io::Result<()> {
    Ok(())
}
