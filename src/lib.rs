pub mod archive;
pub mod cache;
pub mod media;
pub mod metadata;
pub mod viewer;
pub mod window;

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Write},
        path::Path,
    };

    use image::GenericImageView;

    use crate::cache::ImageCache;
    use crate::metadata::{format_metadata, HEADER_HEIGHT};
    use crate::viewer::{
        is_supported_image_path, next_index, pan_for_drag, reset_view, zoom_pan_for_cursor,
        zoom_step, ZoomMode,
    };
    use crate::window::{snap_action, snap_rect, Rect, SnapAction, SnapDirection};
    use crate::{archive::ArchiveKind, media::is_raw_path};
    use std::sync::Arc;

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
    fn formats_metadata_without_a_second_source_line() {
        assert_eq!(
            format_metadata("photo.jpg", (1920, 1080), 1536, "2/7", 1.0),
            "photo.jpg  |  1920×1080  |  1.5 KB  |  2/7  |  zoom 100%"
        );
        assert!(format_metadata("large.jpg", (1, 1), 1_000_000, "1/1", 1.0).contains("1.0 MB"));
    }

    #[test]
    fn header_height_is_reserved_independently_of_loading_state() {
        assert_eq!(HEADER_HEIGHT, 20.0);
    }

    #[test]
    fn image_cache_keeps_only_the_configured_number_of_entries() {
        let mut cache = ImageCache::new(1);
        let first = Arc::new(image::DynamicImage::new_rgb8(2, 2));
        let second = Arc::new(image::DynamicImage::new_rgb8(3, 3));

        cache.insert("first.jpg".into(), first);
        cache.insert("second.jpg".into(), second.clone());

        assert!(cache.get(std::path::Path::new("first.jpg")).is_none());
        assert_eq!(
            cache
                .get(std::path::Path::new("second.jpg"))
                .expect("second image remains")
                .dimensions(),
            (3, 3)
        );
    }

    #[test]
    fn zoom_shortcuts_change_by_25_percentage_points() {
        assert_eq!(zoom_step(1.0, 1), 1.25);
        assert_eq!(zoom_step(1.25, -1), 1.0);
    }

    #[test]
    fn zoom_keeps_the_point_under_the_cursor_stationary() {
        assert_eq!(
            zoom_pan_for_cursor((10.0, 20.0), 1.0, 2.0, (100.0, 80.0), (400.0, 300.0),),
            (320.0, 260.0)
        );
    }

    #[test]
    fn zoom_modes_have_stable_factors() {
        assert_eq!(ZoomMode::Actual.factor(), 1.0);
        assert_eq!(ZoomMode::Fit.factor(), 0.0);
        assert!(ZoomMode::Step(2.0).factor() > 1.0);
    }

    #[test]
    fn window_snap_supports_halves_and_quarters() {
        let work = Rect::new(0, 0, 1920, 1080);
        assert_eq!(
            snap_rect(work, work, SnapDirection::Left),
            Rect::new(0, 0, 960, 1080)
        );
        assert_eq!(
            snap_rect(Rect::new(0, 0, 960, 1080), work, SnapDirection::Up),
            Rect::new(0, 0, 960, 540)
        );
        assert_eq!(
            snap_rect(Rect::new(960, 0, 1920, 1080), work, SnapDirection::Down),
            Rect::new(960, 540, 1920, 1080)
        );
        assert_eq!(
            snap_action(work, work, SnapDirection::Down),
            SnapAction::Minimize
        );
        assert_eq!(
            snap_action(Rect::new(0, 0, 960, 540), work, SnapDirection::Up),
            SnapAction::Rect(work)
        );
    }

    #[test]
    fn snap_restore_is_only_needed_for_a_minimized_window() {
        assert!(crate::window::should_restore_before_snap(true));
        assert!(!crate::window::should_restore_before_snap(false));
    }

    #[test]
    fn minimized_window_can_process_win_up_without_viewer_focus() {
        assert!(crate::window::should_process_snap(false, true));
        assert!(crate::window::should_process_snap(true, false));
        assert!(!crate::window::should_process_snap(false, false));
    }

    #[test]
    fn left_drag_translates_pan_from_gesture_start() {
        assert_eq!(
            pan_for_drag((10.0, 20.0), (100.0, 200.0), (140.0, 170.0)),
            (50.0, -10.0)
        );
    }

    #[test]
    fn ctrl_zero_resets_zoom_and_pan() {
        assert_eq!(reset_view(), ((0.0, 0.0), 1.0));
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
