use std::{io::Write, path::PathBuf};

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use sha2::{Digest, Sha256};
use widno::{archive, viewer};
use zip::write::SimpleFileOptions;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn imported_rar_fixture_is_byte_identical_and_readable() {
    let path = fixture_root().join("archives").join("фотки.rar");
    let bytes = std::fs::read(&path).expect("imported RAR fixture");
    let hash = format!("{:X}", Sha256::digest(&bytes));
    assert_eq!(
        hash,
        "3F6FA5C5A87AF605865C4276BABC7AE6754BF5B1394433608B62ABC520A23C38"
    );

    let entries = archive::list_entries(&path, None).expect("list RAR entries");
    let files: Vec<_> = entries
        .into_iter()
        .filter(|entry| entry.size.unwrap_or(0) > 0)
        .collect();
    assert_eq!(files.len(), 8);
    assert!(files.iter().any(|entry| entry.name.ends_with(".jpg")));
}

#[test]
fn loads_an_image_from_any_supported_archive_index() {
    let path = fixture_root().join("archives").join("фотки.rar");
    let entries = archive::image_entries(&path, None).expect("list archive images");
    assert!(entries.len() > 1);

    let (entry, image) =
        archive::load_image_entry_at(&path, 1, None).expect("load second archive image");
    assert_eq!(entry.name, entries[1].name);
    assert!(image.width() > 0);
    assert!(image.height() > 0);
}

#[test]
fn common_image_formats_round_trip_through_viewer_loader() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let image = RgbaImage::from_pixel(4, 3, Rgba([24, 80, 160, 255]));
    let formats = [
        ("jpeg", ImageFormat::Jpeg),
        ("png", ImageFormat::Png),
        ("gif", ImageFormat::Gif),
        ("bmp", ImageFormat::Bmp),
        ("tiff", ImageFormat::Tiff),
        ("webp", ImageFormat::WebP),
    ];

    for (extension, format) in formats {
        let path = directory.path().join(format!("fixture.{extension}"));
        DynamicImage::ImageRgba8(image.clone())
            .save_with_format(&path, format)
            .expect("write image fixture");
        let loaded = viewer::load_image(&path).expect("load image fixture");
        assert_eq!(loaded.image.dimensions(), (4, 3), "{extension}");
    }
}

#[test]
fn loader_uses_image_content_for_mislabeled_files() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("image.jpg");
    let image = RgbaImage::from_pixel(4, 3, Rgba([24, 80, 160, 255]));
    image
        .save_with_format(&path, ImageFormat::Png)
        .expect("write PNG with a JPEG extension");

    let loaded = viewer::load_image(&path).expect("sniff image content");
    assert_eq!(loaded.image.dimensions(), (4, 3));
}

#[test]
fn password_protected_7z_requires_the_correct_password() {
    let archive_path = fixture_root().join("archives").join("protected.7z");

    assert!(archive::list_entries(&archive_path, Some("wrong")).is_err());
    let entries =
        archive::list_entries(&archive_path, Some("widno-test")).expect("open protected 7z");
    assert_eq!(entries.len(), 1);
    assert_eq!(
        archive::read_entry(&archive_path, &entries[0].name, Some("widno-test"))
            .expect("read protected entry"),
        b"Widno password fixture"
    );
}

#[test]
fn reading_from_an_oversized_zip_is_rejected_before_entry_access() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let archive_path = directory.path().join("too-many-entries.zip");
    let file = std::fs::File::create(&archive_path).expect("create ZIP");
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("target.png", SimpleFileOptions::default())
        .expect("create target entry");
    writer
        .write_all(b"not an image")
        .expect("write target entry");
    for index in 0..100_000 {
        writer
            .start_file(format!("extra-{index}.txt"), SimpleFileOptions::default())
            .expect("create extra entry");
    }
    writer.finish().expect("finish ZIP");

    let error = archive::read_entry(&archive_path, "target.png", None)
        .expect_err("oversized ZIP must be rejected");
    assert!(error.to_string().contains("too many entries"));
}
