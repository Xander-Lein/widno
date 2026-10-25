use std::{
    fs::File,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use sevenz_rust2::{ArchiveReader, Password};
use unrar_rs::RarArchive;
use zip::ZipArchive;

const MAX_ARCHIVE_ENTRIES: usize = 100_000;
const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    SevenZip,
    Rar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub name: String,
    pub size: Option<u64>,
}

pub fn archive_kind(path: &Path) -> Option<ArchiveKind> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "zip" | "cbz" => Some(ArchiveKind::Zip),
        "7z" | "cb7" => Some(ArchiveKind::SevenZip),
        "rar" | "cbr" => Some(ArchiveKind::Rar),
        _ => None,
    }
}

pub fn list_entries(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
    match archive_kind(path).context("unsupported archive extension")? {
        ArchiveKind::Zip => {
            let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
            let mut archive = ZipArchive::new(file)?;
            if archive.len() > MAX_ARCHIVE_ENTRIES {
                anyhow::bail!("archive contains too many entries");
            }
            let mut entries = Vec::with_capacity(archive.len());
            for index in 0..archive.len() {
                let entry = match password {
                    Some(password) => archive.by_index_decrypt(index, password.as_bytes())?,
                    None => archive.by_index(index)?,
                };
                entries.push(ArchiveEntry {
                    name: entry.name().to_string(),
                    size: Some(entry.size()),
                });
            }
            Ok(entries)
        }
        ArchiveKind::SevenZip => {
            let password = password.map(Password::new).unwrap_or_else(Password::empty);
            let reader = ArchiveReader::open(path, password)?;
            if reader.archive().files.len() > MAX_ARCHIVE_ENTRIES {
                anyhow::bail!("archive contains too many entries");
            }
            Ok(reader
                .archive()
                .files
                .iter()
                .map(|entry| ArchiveEntry {
                    name: entry.name.clone(),
                    size: Some(entry.size),
                })
                .collect())
        }
        ArchiveKind::Rar => {
            let file = File::open(path)?;
            let archive = match password {
                Some(password) => RarArchive::open_with_password(file, password)?,
                None => RarArchive::open(file)?,
            };
            let entries: Vec<_> = archive
                .entries()
                .map(|entry| ArchiveEntry {
                    name: entry.name,
                    size: entry.unpacked_size,
                })
                .take(MAX_ARCHIVE_ENTRIES + 1)
                .collect();
            if entries.len() > MAX_ARCHIVE_ENTRIES {
                anyhow::bail!("archive contains too many entries");
            }
            Ok(entries)
        }
    }
}

pub fn read_entry(path: &Path, entry_name: &str, password: Option<&str>) -> Result<Vec<u8>> {
    match archive_kind(path).context("unsupported archive extension")? {
        ArchiveKind::Zip => {
            let file = File::open(path)?;
            let mut archive = ZipArchive::new(file)?;
            if archive.len() > MAX_ARCHIVE_ENTRIES {
                anyhow::bail!("archive contains too many entries");
            }
            let index = (0..archive.len())
                .find(|index| {
                    archive
                        .by_index(*index)
                        .map(|entry| entry.name() == entry_name)
                        .unwrap_or(false)
                })
                .ok_or_else(|| anyhow!("archive entry not found: {entry_name}"))?;
            let mut entry = match password {
                Some(password) => archive.by_index_decrypt(index, password.as_bytes())?,
                None => archive.by_index(index)?,
            };
            let declared_size = entry.size();
            read_limited(&mut entry, declared_size)
        }
        ArchiveKind::SevenZip => {
            let password = password.map(Password::new).unwrap_or_else(Password::empty);
            let mut reader = ArchiveReader::open(path, password)?;
            let size = reader
                .archive()
                .files
                .iter()
                .find(|entry| entry.name == entry_name)
                .map(|entry| entry.size)
                .ok_or_else(|| anyhow!("archive entry not found: {entry_name}"))?;
            if size > MAX_ENTRY_BYTES {
                anyhow::bail!("archive entry exceeds the 256 MiB safety limit");
            }
            Ok(reader.read_file(entry_name)?)
        }
        ArchiveKind::Rar => {
            let file = File::open(path)?;
            let mut archive = match password {
                Some(password) => RarArchive::open_with_password(file, password)?,
                None => RarArchive::open(file)?,
            };
            let mut entry = archive.by_name(entry_name)?;
            let declared_size = entry.size().unwrap_or(0);
            read_limited(&mut entry, declared_size)
        }
    }
}

fn read_limited(reader: &mut impl Read, declared_size: u64) -> Result<Vec<u8>> {
    if declared_size > MAX_ENTRY_BYTES {
        anyhow::bail!("archive entry exceeds the 256 MiB safety limit");
    }
    let mut bytes = Vec::with_capacity(declared_size.min(MAX_ENTRY_BYTES) as usize);
    reader.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ENTRY_BYTES {
        anyhow::bail!("archive entry exceeds the 256 MiB safety limit");
    }
    Ok(bytes)
}

pub fn load_image_entry(
    archive_path: &Path,
    entry_name: &str,
    password: Option<&str>,
) -> Result<image::DynamicImage> {
    let bytes = read_entry(archive_path, entry_name, password)?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32_768);
    limits.max_image_height = Some(32_768);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader.decode()?)
}

pub fn load_image_entry_at(
    archive_path: &Path,
    index: usize,
    password: Option<&str>,
) -> Result<(ArchiveEntry, image::DynamicImage)> {
    let entry = image_entries(archive_path, password)?
        .into_iter()
        .nth(index)
        .ok_or_else(|| anyhow!("archive image index out of range: {index}"))?;
    let image = load_image_entry(archive_path, &entry.name, password)?;
    Ok((entry, image))
}

pub fn image_entries(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
    Ok(list_entries(path, password)?
        .into_iter()
        .filter(|entry| {
            let candidate = PathBuf::from(&entry.name);
            crate::viewer::is_supported_image_path(&candidate)
        })
        .collect())
}
