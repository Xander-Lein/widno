pub mod archive;
pub mod cache;
pub mod media;
pub mod metadata;
pub mod settings;
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
    use crate::settings::{context_extensions, Settings};
    use crate::viewer::{
        is_supported_image_path, next_index, pan_for_drag, pan_for_resize, reset_view,
        prefetch_texture_dimensions, smart_texture_dimensions, texture_cache_key,
        zoom_pan_for_cursor, zoom_step, ZoomMode,
    };
    use crate::window::{
        is_fullscreen, mark_native_maximized, native_snap_style, snap_action, snap_rect, Rect,
        SnapAction, SnapDirection, WS_CAPTION, WS_MAXIMIZE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
        WS_SYSMENU, WS_THICKFRAME,
    };
    use crate::{archive::ArchiveKind, media::is_raw_path};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[test]
    fn recognizes_common_image_extensions_case_insensitively() {
        assert!(is_supported_image_path(Path::new("photo.JPEG")));
        assert!(is_supported_image_path(Path::new("scan.tiff")));
        assert!(is_supported_image_path(Path::new("camera.CR3")));
        assert!(is_supported_image_path(Path::new("icon.svg")));
        assert!(!is_supported_image_path(Path::new("notes.txt")));
    }

    #[test]
    fn renders_static_svg_without_external_resources() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("icon.svg");
        std::fs::write(
            &path,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4"><rect width="8" height="4" fill="red"/></svg>"#,
        )
        .expect("write svg");
        let image = crate::media::svg_preview(&path).expect("render svg");
        assert_eq!(image.dimensions(), (8, 4));
    }

    #[test]
    fn smart_downscaling_caps_texture_to_two_viewport_dimensions() {
        assert_eq!(
            smart_texture_dimensions((8000, 6000), (1920, 1080)),
            (3840, 2880)
        );
        assert_eq!(
            smart_texture_dimensions((1200, 800), (1920, 1080)),
            (1200, 800)
        );
    }

    #[test]
    fn prefetch_texture_dimensions_match_the_viewport_budget() {
        assert_eq!(
            prefetch_texture_dimensions((8000, 6000), (1920, 1080)),
            (3840, 2880)
        );
    }

    #[test]
    fn settings_default_allows_images_and_archives_and_round_trips() {
        let settings = Settings::default();
        assert!(settings.enabled_formats[".svg"]);
        assert!(settings.enabled_formats[".nrw"]);
        assert!(!settings.enabled_formats.contains_key(".rar"));
        assert!(settings.context_files);
        assert!(settings.context_archives);
        assert!(settings.context_folders);
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("settings.json");
        settings.save(&path).expect("save settings");
        let loaded: Settings =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read settings"))
                .expect("parse settings");
        assert_eq!(loaded.enabled_formats, settings.enabled_formats);
    }

    #[test]
    fn context_extensions_follow_settings_categories() {
        let settings = Settings {
            context_files: false,
            context_archives: true,
            ..Settings::default()
        };
        let extensions = context_extensions(&settings);
        assert!(extensions.contains(&".cbz"));
        assert!(!extensions.contains(&".jpg"));
    }

    #[test]
    fn texture_cache_key_changes_with_viewport() {
        let path = std::path::Path::new("photo.jpg");
        assert_ne!(
            texture_cache_key(path, (1920, 1080)),
            texture_cache_key(path, (1280, 720))
        );
    }

    #[test]
    fn load_latency_is_reported_in_milliseconds() {
        let start = Instant::now();
        let end = start + Duration::from_millis(125);
        assert_eq!(crate::viewer::load_elapsed_ms(start, end), 125);
    }

    #[test]
    fn navigation_wraps_around_folder_boundaries() {
        assert_eq!(next_index(0, 3, -1), Some(2));
        assert_eq!(next_index(2, 3, 1), Some(0));
        assert_eq!(next_index(0, 0, 1), None);
    }

    #[test]
    fn formats_metadata_without_a_filename() {
        assert_eq!(
            format_metadata((1920, 1080), 1536, "2/7", 1.0),
            "1920×1080  |  1.5 KB  |  2/7  |  zoom 100%"
        );
        assert!(format_metadata((1, 1), 1_000_000, "1/1", 1.0).contains("1.0 MB"));
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
            SnapAction::Rect(Rect::new(0, 540, 1920, 1080))
        );
        assert_eq!(
            snap_action(Rect::new(0, 0, 960, 540), work, SnapDirection::Up),
            SnapAction::Rect(work)
        );
        assert_eq!(snap_action(work, work, SnapDirection::Up), SnapAction::Noop);
        assert_eq!(
            snap_action(Rect::new(0, 0, 960, 1080), work, SnapDirection::Left),
            SnapAction::Noop
        );
    }

    #[test]
    fn fullscreen_is_exactly_the_work_area() {
        let work = Rect::new(0, 0, 1920, 1080);
        assert!(is_fullscreen(work, work));
        assert!(!is_fullscreen(Rect::new(0, 0, 960, 1080), work));
    }

    #[test]
    fn native_snap_style_keeps_window_borderless_but_snap_capable() {
        let style = native_snap_style(WS_CAPTION | WS_SYSMENU);
        assert_eq!(style & (WS_CAPTION | WS_SYSMENU), 0);
        assert_ne!(style & WS_THICKFRAME, 0);
        assert_ne!(style & WS_MINIMIZEBOX, 0);
        assert_ne!(style & WS_MAXIMIZEBOX, 0);
    }

    #[test]
    fn native_maximized_state_is_explicitly_marked() {
        assert_ne!(mark_native_maximized(0) & WS_MAXIMIZE, 0);
    }

    #[test]
    fn window_snap_moves_between_corners_without_expanding_to_a_half() {
        let work = Rect::new(0, 0, 1920, 1080);
        let top_half = Rect::new(0, 0, 1920, 540);
        let bottom_half = Rect::new(0, 540, 1920, 1080);
        assert_eq!(
            snap_rect(top_half, work, SnapDirection::Left),
            Rect::new(0, 0, 960, 540)
        );
        assert_eq!(
            snap_rect(top_half, work, SnapDirection::Right),
            Rect::new(960, 0, 1920, 540)
        );
        assert_eq!(snap_rect(top_half, work, SnapDirection::Up), work);
        assert_eq!(
            snap_rect(Rect::new(0, 0, 960, 540), work, SnapDirection::Right),
            top_half
        );
        assert_eq!(
            snap_rect(Rect::new(960, 0, 1920, 540), work, SnapDirection::Down),
            Rect::new(960, 0, 1920, 1080)
        );
        assert_eq!(
            snap_rect(Rect::new(0, 540, 960, 1080), work, SnapDirection::Right),
            bottom_half
        );
        assert_eq!(snap_rect(bottom_half, work, SnapDirection::Up), top_half);
        assert_eq!(
            snap_rect(bottom_half, work, SnapDirection::Left),
            Rect::new(0, 540, 960, 1080)
        );
        assert_eq!(
            snap_rect(bottom_half, work, SnapDirection::Right),
            Rect::new(960, 540, 1920, 1080)
        );
        assert_eq!(
            snap_rect(Rect::new(0, 540, 960, 1080), work, SnapDirection::Up),
            Rect::new(0, 0, 960, 1080)
        );
    }

    #[test]
    fn corner_returns_to_its_side_half_with_vertical_arrow() {
        let work = Rect::new(0, 0, 1920, 1080);
        assert_eq!(
            snap_rect(Rect::new(0, 0, 960, 540), work, SnapDirection::Down),
            Rect::new(0, 0, 960, 1080)
        );
        assert_eq!(
            snap_rect(Rect::new(960, 540, 1920, 1080), work, SnapDirection::Up),
            Rect::new(960, 0, 1920, 1080)
        );
    }

    #[test]
    fn snap_never_creates_a_full_width_top_half() {
        let work = Rect::new(0, 0, 1920, 1080);
        let top_half = Rect::new(0, 0, 1920, 540);
        assert_ne!(snap_rect(work, work, SnapDirection::Up), top_half);
        assert_ne!(
            snap_rect(Rect::new(0, 0, 960, 1080), work, SnapDirection::Up),
            top_half
        );
    }

    #[test]
    fn image_cache_default_capacity_keeps_a_session_working_set() {
        assert_eq!(crate::cache::DEFAULT_CAPACITY, 8);
        assert_eq!(crate::cache::DEFAULT_MAX_BYTES, 512 * 1024 * 1024);
    }

    #[test]
    fn image_cache_evicts_least_recently_used_entries_by_bytes() {
        let mut cache = ImageCache::with_limits(8, 12);
        cache.insert(
            Path::new("first.png").to_path_buf(),
            Arc::new(image::DynamicImage::new_rgb8(2, 2)),
        );
        cache.insert(
            Path::new("second.png").to_path_buf(),
            Arc::new(image::DynamicImage::new_rgb8(2, 2)),
        );
        assert!(cache.get(Path::new("first.png")).is_none());
        assert!(cache.current_bytes() <= 12);
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
    fn resize_preserves_the_image_point_at_the_viewport_center() {
        assert_eq!(
            pan_for_resize((120.0, -40.0), (1920.0, 1080.0), (1280.0, 720.0)),
            (440.0, 140.0)
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
    fn archive_entry_names_reject_traversal_and_absolute_paths() {
        assert!(crate::archive::is_safe_entry_name("folder/photo.jpg"));
        assert!(!crate::archive::is_safe_entry_name("../photo.jpg"));
        assert!(!crate::archive::is_safe_entry_name(
            "folder/../../photo.jpg"
        ));
        assert!(!crate::archive::is_safe_entry_name(r"..\photo.jpg"));
        assert!(!crate::archive::is_safe_entry_name("/photo.jpg"));
        assert!(!crate::archive::is_safe_entry_name(r"C:\photo.jpg"));
    }

    #[test]
    fn svg_preview_rejects_oversized_render_dimensions() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("huge.svg");
        let mut svg = vec![b' '; 16 * 1024 * 1024 + 1];
        svg.extend_from_slice(
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1"/></svg>"#,
        );
        std::fs::write(&path, svg).expect("write SVG");
        assert!(crate::media::svg_preview(&path).is_err());
    }

    #[cfg(not(feature = "pdf"))]
    #[test]
    fn pdf_support_is_explicitly_disabled_in_default_build() {
        assert!(!crate::media::pdf_runtime_available());
        let error = crate::media::render_pdf_first_page(Path::new("fixture.pdf"))
            .expect_err("default build must not claim PDF support");
        assert!(error.to_string().contains("PDF support is disabled"));
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
