pub mod archive;
pub mod media;
pub mod viewer;

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Write},
        path::Path,
    };

    use image::GenericImageView;

    use crate::viewer::{is_supported_image_path, next_index, ZoomMode};
    use crate::{archive::ArchiveKind, media::is_raw_path};

    #[test]
    fn recognizes_common_image_extensions_case_insensitively() {
        assert!(is_supported_image_path(Path::new("photo.JPEG")));
        assert!(is_supported_image_path(Path::new("scan.tiff")));
        assert!(!is_supported_image_path(Path::new("notes.txt")));
    }

    #[test]
    fn navigation_wraps_around_folder_boundaries() {
        assert_eq!(next_index(0, 3, -1), Some(2));
        assert_eq!(next_index(2, 3, 1), Some(0));
        assert_eq!(next_index(0, 0, 1), None);
    }

    #[test]
    fn zoom_modes_have_stable_factors() {
        assert_eq!(ZoomMode::Actual.factor(), 1.0);
        assert_eq!(ZoomMode::Fit.factor(), 0.0);
        assert!(ZoomMode::Step(2.0).factor() > 1.0);
    }

    #[test]
    fn identifies_archive_and_raw_sources() {
        assert_eq!(
            crate::archive::archive_kind(Path::new("photos.CB7")),
            Some(ArchiveKind::SevenZip)
        );
        assert!(is_raw_path(Path::new("camera.CR3")));
    }

    #[test]
    fn loads_a_common_png_without_blocking_the_contract() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("sample.png");
        let image = image::RgbaImage::from_pixel(3, 2, image::Rgba([20, 40, 60, 255]));
        image.save(&path).expect("write PNG fixture");

        let loaded = crate::viewer::load_image(&path).expect("load PNG fixture");
        assert_eq!(loaded.image.dimensions(), (3, 2));
    }

    #[test]
    fn loads_an_image_from_a_zip_without_extracting_it() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let archive_path = directory.path().join("photos.zip");
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([120, 80, 40, 255]));

        let mut archive_file = std::fs::File::create(&archive_path).expect("create ZIP");
        let mut writer = zip::ZipWriter::new(&mut archive_file);
        writer
            .start_file("photo.png", zip::write::SimpleFileOptions::default())
            .expect("create ZIP entry");
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut png, image::ImageFormat::Png)
            .expect("encode PNG");
        writer.write_all(png.get_ref()).expect("write ZIP entry");
        writer.finish().expect("finish ZIP");

        let loaded = crate::archive::load_image_entry(&archive_path, "photo.png", None)
            .expect("load image from ZIP");
        assert_eq!(loaded.dimensions(), (2, 2));
    }
}
