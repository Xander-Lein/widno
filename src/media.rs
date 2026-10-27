use std::{fs::File, io::BufReader, path::Path};

use anyhow::{Context, Result};
use image::{DynamicImage, GrayImage, ImageBuffer, Rgba, RgbaImage};
#[cfg(feature = "pdf")]
use pdfium_render::prelude::*;

const RAW_EXTENSIONS: &[&str] = &[
    "arw", "cr2", "cr3", "crw", "dng", "mrw", "nef", "nrw", "orf", "pef", "raf", "rwl", "rw2",
    "sr2", "srf", "srw", "x3f",
];
const MAX_RAW_FILE_BYTES: u64 = 512 * 1024 * 1024;

pub fn is_raw_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| RAW_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_svg_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("svg"))
        .unwrap_or(false)
}

pub fn svg_preview(path: &Path) -> Result<DynamicImage> {
    let data = std::fs::read(path)?;
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&data, &options)?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .context("SVG dimensions are too large")?;
    resvg::render(
        &tree,
        resvg::usvg::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let image = RgbaImage::from_raw(size.width(), size.height(), pixmap.take())
        .context("SVG renderer returned an invalid image")?;
    Ok(DynamicImage::ImageRgba8(image))
}

pub fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

pub fn raw_preview(path: &Path) -> Result<DynamicImage> {
    if std::fs::metadata(path)?.len() > MAX_RAW_FILE_BYTES {
        anyhow::bail!("RAW file exceeds the 512 MiB safety limit");
    }
    let raw = rawloader::decode_file(path)
        .with_context(|| format!("decode RAW file {}", path.display()))?;
    let width = u32::try_from(raw.width).context("RAW width exceeds u32")?;
    let height = u32::try_from(raw.height).context("RAW height exceeds u32")?;
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .context("RAW dimensions overflow")?;
    if width > 32_768 || height > 32_768 || pixels > 100_000_000 {
        anyhow::bail!("RAW dimensions exceed the safety limit");
    }
    let samples: Vec<u16> = match raw.data {
        rawloader::RawImageData::Integer(samples) => samples,
        rawloader::RawImageData::Float(samples) => samples
            .into_iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * u16::MAX as f32) as u16)
            .collect(),
    };

    if samples.len() < (width * height) as usize {
        anyhow::bail!("RAW decoder returned fewer samples than its dimensions");
    }

    let max = raw
        .whitelevels
        .iter()
        .copied()
        .max()
        .unwrap_or(u16::MAX)
        .max(1);
    let mut image: RgbaImage = ImageBuffer::new(width, height);
    for (index, pixel) in image.pixels_mut().enumerate() {
        let value = ((samples[index] as u32 * 255) / max as u32).min(255) as u8;
        *pixel = Rgba([value, value, value, 255]);
    }

    Ok(DynamicImage::ImageRgba8(image))
}

pub fn read_exif(path: &Path) -> Result<Vec<(String, String)>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader)?;
    Ok(exif
        .fields()
        .map(|field| {
            (
                field.tag.to_string(),
                field.display_value().with_unit(&exif).to_string(),
            )
        })
        .collect())
}

#[cfg(feature = "pdf")]
pub fn pdf_runtime_available() -> bool {
    let directory = std::env::var_os("WIDNO_PDFIUM_PATH")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()?
                .parent()
                .map(Path::to_path_buf)
        });
    directory
        .map(|directory| Pdfium::pdfium_platform_library_name_at_path(&directory).is_file())
        .unwrap_or(false)
}

#[cfg(feature = "pdf")]
pub fn render_pdf_first_page(path: &Path) -> Result<DynamicImage> {
    render_pdf_page(path, 0)
}

#[cfg(feature = "pdf")]
pub fn pdf_page_count(path: &Path) -> Result<usize> {
    let pdfium = bind_pdfium()?;
    let document = pdfium.load_pdf_from_file(path, None)?;
    Ok(document.pages().len() as usize)
}

#[cfg(feature = "pdf")]
pub fn render_pdf_page(path: &Path, page_index: usize) -> Result<DynamicImage> {
    let pdfium = bind_pdfium()?;
    let document = pdfium.load_pdf_from_file(path, None)?;
    let page = document
        .pages()
        .get(page_index as u16)
        .context("PDF page index out of bounds")?;
    let rendered = page
        .render_with_config(
            &PdfRenderConfig::new()
                .set_target_width(2400)
                .set_maximum_height(32_768),
        )?
        .as_image();
    Ok(rendered)
}

#[cfg(feature = "pdf")]
fn bind_pdfium() -> Result<Pdfium> {
    let library_dir = std::env::var_os("WIDNO_PDFIUM_PATH")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()?
                .parent()
                .map(Path::to_path_buf)
        })
        .context("put pdfium.dll next to widno.exe or set WIDNO_PDFIUM_PATH")?;
    let library_name = Pdfium::pdfium_platform_library_name_at_path(&library_dir);
    let bindings = Pdfium::bind_to_library(library_name)?;
    Ok(Pdfium::new(bindings))
}

#[cfg(not(feature = "pdf"))]
pub fn pdf_runtime_available() -> bool {
    false
}

#[cfg(not(feature = "pdf"))]
pub fn render_pdf_first_page(_path: &Path) -> Result<DynamicImage> {
    anyhow::bail!("PDF support is disabled in this build")
}

#[cfg(not(feature = "pdf"))]
pub fn pdf_page_count(_path: &Path) -> Result<usize> {
    anyhow::bail!("PDF support is disabled in this build")
}

#[cfg(not(feature = "pdf"))]
pub fn render_pdf_page(_path: &Path, _page_index: usize) -> Result<DynamicImage> {
    anyhow::bail!("PDF support is disabled in this build")
}

pub fn grayscale_preview(image: &DynamicImage) -> GrayImage {
    image.to_luma8()
}
