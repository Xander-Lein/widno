use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub const IMAGE_FORMATS: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tif", ".tiff", ".webp", ".svg", ".ico", ".cur",
    ".dib", ".j2c", ".j2k", ".jp2", ".pbm", ".pcx", ".pgm", ".ppm", ".tga", ".arw", ".cr2", ".cr3",
    ".crw", ".dng", ".mrw", ".nef", ".nrw", ".orf", ".pef", ".raf", ".rwl", ".rw2", ".sr2", ".srf",
    ".srw", ".x3f",
];
pub const ARCHIVE_FORMATS: &[&str] = &[".zip", ".rar", ".7z", ".cbz", ".cbr", ".cb7"];

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

pub fn context_extensions(settings: &Settings) -> Vec<&'static str> {
    let mut extensions = Vec::new();
    if settings.context_files {
        extensions.extend(IMAGE_FORMATS.iter().copied());
    }
    if settings.context_archives {
        extensions.extend(ARCHIVE_FORMATS.iter().copied());
    }
    extensions
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
    use std::process::Command;

    let binary = std::env::current_exe()?;
    let binary = binary.to_string_lossy();
    let prog_id_root = r"HKCU\Software\Classes\Widno.ImageFile";
    let open_command = format!(r#""{}" "%1""#, binary);
    let icon_value = format!(r#""{}",0"#, binary);
    for (key, value) in [
        (prog_id_root, "Widno image"),
        (
            r"HKCU\Software\Classes\Widno.ImageFile\DefaultIcon",
            icon_value.as_str(),
        ),
        (
            r"HKCU\Software\Classes\Widno.ImageFile\shell\open\command",
            open_command.as_str(),
        ),
    ] {
        let status = Command::new("reg")
            .args(["add", key, "/ve", "/d", value, "/f"])
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other(format!("reg.exe failed for {key}")));
        }
    }

    for extension in IMAGE_FORMATS {
        let enabled = settings
            .enabled_formats
            .get(*extension)
            .copied()
            .unwrap_or(false);
        let user_choice = format!(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{}\UserChoice",
            extension
        );
        let claimed = Command::new("reg")
            .args(["query", &user_choice])
            .status()?
            .success();
        if !claimed && enabled {
            let class_key = format!(r"HKCU\Software\Classes\{}", extension);
            let status = Command::new("reg")
                .args(["add", &class_key, "/ve", "/d", "Widno.ImageFile", "/f"])
                .status()?;
            if !status.success() {
                return Err(std::io::Error::other(format!(
                    "reg.exe failed for {extension}"
                )));
            }
        }
        if !enabled {
            let open_with = format!(r"HKCU\Software\Classes\{}\OpenWithProgids", extension);
            let _ = Command::new("reg")
                .args(["delete", &open_with, "/v", "Widno.ImageFile", "/f"])
                .status();
        }
    }

    let context_extensions = context_extensions(settings);
    for extension in IMAGE_FORMATS.iter().chain(ARCHIVE_FORMATS.iter()) {
        let verb = format!(
            r"HKCU\Software\Classes\SystemFileAssociations\{}\shell\Browse with Widno",
            extension
        );
        if context_extensions.contains(extension) {
            let command = format!(r#""{}" "%1""#, binary);
            let command_key = format!(r"{}\command", verb);
            let status = Command::new("reg")
                .args(["add", &command_key, "/ve", "/d", &command, "/f"])
                .status()?;
            if !status.success() {
                return Err(std::io::Error::other(format!("reg.exe failed for {verb}")));
            }
            let _ = Command::new("reg")
                .args(["add", &verb, "/ve", "/d", "Browse with Widno", "/f"])
                .status();
        } else {
            let _ = Command::new("reg").args(["delete", &verb, "/f"]).status();
        }
    }

    let folder_verb = r"HKCU\Software\Classes\Directory\shell\Browse with Widno";
    if settings.context_folders {
        let command_key = format!(r"{}\command", folder_verb);
        let status = Command::new("reg")
            .args(["add", &command_key, "/ve", "/d", &open_command, "/f"])
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other(
                "reg.exe failed for folder context menu",
            ));
        }
        let _ = Command::new("reg")
            .args(["add", folder_verb, "/ve", "/d", "Browse with Widno", "/f"])
            .status();
    } else {
        let _ = Command::new("reg")
            .args(["delete", folder_verb, "/f"])
            .status();
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn apply_default_associations(_settings: &Settings) -> std::io::Result<()> {
    Ok(())
}
