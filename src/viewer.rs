use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
};

use image::{DynamicImage, ImageReader};
use thiserror::Error;

use crate::{archive, media};

const MAX_IMAGE_PIXELS: u64 = 100_000_000;

#[derive(Debug, Error)]
pub enum ViewerError {
    #[error("failed to read image `{path}`: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoomMode {
    Fit,
    Actual,
    Step(f32),
}

pub const ZOOM_STEP: f32 = 0.25;

pub fn zoom_step(current: f32, direction: i32) -> f32 {
    let current = current.clamp(0.05, 32.0);
    (current + ZOOM_STEP * direction.signum() as f32).clamp(0.05, 32.0)
}

pub fn zoom_pan_for_cursor(
    pan: (f32, f32),
    current_zoom: f32,
    new_zoom: f32,
    cursor: (f32, f32),
    viewport_center: (f32, f32),
) -> (f32, f32) {
    let ratio = new_zoom / current_zoom.max(0.0001);
    (
        (cursor.0 - viewport_center.0) * (1.0 - ratio) + pan.0 * ratio,
        (cursor.1 - viewport_center.1) * (1.0 - ratio) + pan.1 * ratio,
    )
}

pub fn pan_for_drag(
    start_pan: (f32, f32),
    pointer_start: (f32, f32),
    pointer_current: (f32, f32),
) -> (f32, f32) {
    (
        start_pan.0 + pointer_current.0 - pointer_start.0,
        start_pan.1 + pointer_current.1 - pointer_start.1,
    )
}

pub fn reset_view() -> ((f32, f32), f32) {
    ((0.0, 0.0), 1.0)
}

impl ZoomMode {
    pub fn factor(self) -> f32 {
        match self {
            Self::Fit => 0.0,
            Self::Actual => 1.0,
            Self::Step(value) => value.max(0.05),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadedImage {
    pub source: PathBuf,
    pub image: Arc<DynamicImage>,
    pub archive_entry: Option<archive::ArchiveEntry>,
    pub archive_entries: Option<Vec<archive::ArchiveEntry>>,
}

pub fn is_supported_image_path(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "bmp"
            | "cur"
            | "dib"
            | "gif"
            | "ico"
            | "j2c"
            | "j2k"
            | "jp2"
            | "jpeg"
            | "jpg"
            | "jpe"
            | "pbm"
            | "pcx"
            | "pgm"
            | "png"
            | "ppm"
            | "tga"
            | "tif"
            | "tiff"
            | "webp"
    )
}

pub fn next_index(current: usize, len: usize, direction: i32) -> Option<usize> {
    if len == 0 {
        return None;
    }

    let current = current.min(len - 1);
    Some(if direction < 0 {
        if current == 0 {
            len - 1
        } else {
            current - 1
        }
    } else if current + 1 == len {
        0
    } else {
        current + 1
    })
}

pub fn load_image(path: impl Into<PathBuf>) -> Result<LoadedImage, ViewerError> {
    let path = path.into();
    let bytes = std::fs::read(&path).map_err(|error| ViewerError::Decode {
        path: path.clone(),
        source: image::ImageError::IoError(error),
    })?;
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|source| ViewerError::Decode {
            path: path.clone(),
            source: image::ImageError::IoError(source),
        })?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32_768);
    limits.max_image_height = Some(32_768);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|source| ViewerError::Decode {
        path: path.clone(),
        source,
    })?;
    validate_image_size(&image)?;

    Ok(LoadedImage {
        source: path,
        image: Arc::new(image),
        archive_entry: None,
        archive_entries: None,
    })
}

pub fn load_path(path: impl Into<PathBuf>) -> Result<LoadedImage, ViewerError> {
    load_path_with_password(path, None)
}

pub fn load_path_with_password(
    path: impl Into<PathBuf>,
    password: Option<&str>,
) -> Result<LoadedImage, ViewerError> {
    let path = path.into();

    if archive::archive_kind(&path).is_some() {
        let entries = archive::image_entries(&path, password)
            .map_err(|error| ViewerError::Message(error.to_string()))?;
        let entry = entries
            .first()
            .ok_or_else(|| ViewerError::Message("archive contains no supported images".into()))?;
        let image = archive::load_image_entry(&path, &entry.name, password)
            .map_err(|error| ViewerError::Message(error.to_string()))?;
        validate_image_size(&image)?;
        return Ok(LoadedImage {
            source: PathBuf::from(format!("{}::{}", path.display(), entry.name)),
            image: Arc::new(image),
            archive_entry: Some(entry.clone()),
            archive_entries: Some(entries),
        });
    }

    if media::is_raw_path(&path) {
        let image =
            media::raw_preview(&path).map_err(|error| ViewerError::Message(error.to_string()))?;
        validate_image_size(&image)?;
        return Ok(LoadedImage {
            source: path,
            image: Arc::new(image),
            archive_entry: None,
            archive_entries: None,
        });
    }

    if media::is_pdf_path(&path) {
        let image = media::render_pdf_first_page(&path)
            .map_err(|error| ViewerError::Message(error.to_string()))?;
        validate_image_size(&image)?;
        return Ok(LoadedImage {
            source: path,
            image: Arc::new(image),
            archive_entry: None,
            archive_entries: None,
        });
    }

    load_image(path)
}

fn validate_image_size(image: &DynamicImage) -> Result<(), ViewerError> {
    let pixels = u64::from(image.width()) * u64::from(image.height());
    if pixels > MAX_IMAGE_PIXELS {
        return Err(ViewerError::Message(
            "image exceeds the 100 megapixel safety limit".into(),
        ));
    }
    Ok(())
}

pub fn load_image_async(path: PathBuf) -> Receiver<Result<LoadedImage, ViewerError>> {
    load_image_async_with_password(path, None)
}

pub fn load_image_async_with_password(
    path: PathBuf,
    password: Option<String>,
) -> Receiver<Result<LoadedImage, ViewerError>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(load_path_with_password(path, password.as_deref()));
    });
    receiver
}

pub fn load_archive_entry_async(
    path: PathBuf,
    entry: archive::ArchiveEntry,
    password: Option<String>,
) -> Receiver<Result<LoadedImage, ViewerError>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = archive::load_image_entry(&path, &entry.name, password.as_deref())
            .map_err(|error| ViewerError::Message(error.to_string()))
            .and_then(|image| {
                validate_image_size(&image)?;
                Ok(LoadedImage {
                    source: PathBuf::from(format!("{}::{}", path.display(), entry.name)),
                    image: Arc::new(image),
                    archive_entry: Some(entry),
                    archive_entries: None,
                })
            });
        let _ = sender.send(result);
    });
    receiver
}

pub fn load_pdf_page_async(
    path: PathBuf,
    page_index: usize,
) -> Receiver<Result<LoadedImage, ViewerError>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = media::render_pdf_page(&path, page_index)
            .map_err(|error| ViewerError::Message(error.to_string()))
            .and_then(|image| {
                validate_image_size(&image)?;
                Ok(LoadedImage {
                    source: path,
                    image: Arc::new(image),
                    archive_entry: None,
                    archive_entries: None,
                })
            });
        let _ = sender.send(result);
    });
    receiver
}
