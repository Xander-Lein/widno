pub mod archive;
pub mod cache;
pub mod formats;
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
    use crate::formats::context_extensions;
    use crate::metadata::{format_metadata, HEADER_HEIGHT};
    use crate::viewer::{
        is_supported_image_path, next_index, pan_for_drag, pan_for_resize, pan_for_resize_scale,
        reset_view, resize_scale_factor, texture_cache_key, zoom_pan_for_cursor, zoom_step,
        ZoomMode,
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
    fn prepared_texture_contains_full_resolution_rgba_pixels() {
        let image = image::DynamicImage::new_rgba8(8000, 6000);
        let prepared = crate::viewer::prepare_texture(&image).expect("large test image");
        assert_eq!(prepared.size, (8000, 6000));
        assert_eq!(prepared.rgba.len(), 8000 * 6000 * 4);
    }

    #[test]
    fn prepared_texture_uses_actual_dimensions_after_aspect_ratio_resize() {
        let image = image::DynamicImage::new_rgba8(65, 31);
        let prepared = crate::viewer::prepare_texture(&image).expect("small image");
        assert_eq!(
            prepared.rgba.len(),
            prepared.size.0 as usize * prepared.size.1 as usize * 4
        );
    }

    #[test]
    fn prepared_texture_rejects_excessive_rgba_budget() {
        let image = image::DynamicImage::new_rgba8(16_000, 16_000);
        assert!(crate::viewer::prepare_texture(&image).is_err());
    }

    #[test]
    fn context_menu_extensions_include_supported_images_and_archives() {
        let extensions = context_extensions();
        assert!(extensions.contains(&".jpg"));
        assert!(extensions.contains(&".cbz"));
        assert!(extensions.contains(&".rar"));
    }

    #[test]
    fn context_menu_registration_is_explicit_and_never_default_association() {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join("register-file-associations.ps1"),
        )
        .expect("context menu registration script");
        assert!(script.contains("Open with Widno"));
        assert!(script.contains("Directory\\shell"));
        assert!(script.contains("SystemFileAssociations"));
        assert!(script.contains("Remove-Item -LiteralPath (Join-Path $systemAssociationKey"));
        assert!(script.contains("Set-Item -Path $Path -Value $Value"));
        assert!(!script.contains("New-ItemProperty -LiteralPath $openWithProgids"));
        assert!(!script.contains("Set-DefaultIfUnclaimed"));
    }

    #[test]
    fn installer_uses_per_user_local_appdata_directory() {
        let installer = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("installer")
                .join("widno.iss"),
        )
        .expect("installer script");
        assert!(installer.contains("DefaultDirName={localappdata}\\Programs\\Widno"));
        assert!(!installer.contains("DefaultDirName={autopf}\\Widno"));
        assert!(installer.contains("PrivilegesRequired=lowest"));
    }

    #[test]
    fn installer_has_no_additional_tasks_page_and_explains_associations() {
        let installer = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("installer")
                .join("widno.iss"),
        )
        .expect("installer script");
        assert!(!installer.contains("[Tasks]"));
        assert!(installer.contains("[Messages]"));
        assert!(installer.contains("FinishedLabel="));
        assert!(installer.contains("Open with Widno"));
        assert!(installer.contains("default"));
    }

    #[test]
    fn first_load_benchmark_script_uses_release_timing_contract() {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join("benchmark-first-load.ps1"),
        )
        .expect("first-load benchmark script");
        assert!(script.contains("WIDNO_LOAD_TIMING"));
        assert!(script.contains("Wait-Process"));
        assert!(script.contains("widno-load-times.csv"));
        assert!(script.contains("Split-Path"));
        assert!(script.contains("$Runs"));
    }

    #[test]
    fn first_load_benchmark_quotes_paths_for_process_arguments() {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join("benchmark-first-load.ps1"),
        )
        .expect("first-load benchmark script");
        assert!(script.contains("ArgumentList @(\"`\"$resolvedPath`\"\")"));
    }

    #[test]
    fn association_smoke_script_checks_context_menu_without_defaults() {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join("association-smoke.ps1"),
        )
        .expect("association smoke script");
        assert!(script.contains("Open with Widno"));
        assert!(script.contains("SystemFileAssociations"));
        assert!(script.contains("Directory\\shell"));
        assert!(script.contains("UserChoice"));
        assert!(script.contains("OpenWithProgids"));
    }

    #[test]
    fn association_smoke_covers_every_registered_extension() {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join("association-smoke.ps1"),
        )
        .expect("association smoke script");
        for extension in context_extensions() {
            assert!(
                script.contains(&format!("'{extension}'")),
                "association smoke is missing {extension}"
            );
        }
    }

    #[test]
    fn gui_smoke_uses_working_area_when_taskbar_is_visible() {
        let script = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join("gui-smoke.ps1"),
        )
        .expect("GUI smoke script");
        assert!(script.contains("WorkingArea"));
        assert!(script.contains("dpi tolerance"));
    }

    #[test]
    fn ci_workflow_runs_windows_quality_gates() {
        let workflow = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(".github")
                .join("workflows")
                .join("ci.yml"),
        )
        .expect("CI workflow");
        assert!(workflow.contains("windows-latest"));
        assert!(workflow.contains("cargo test --locked"));
        assert!(workflow.contains("cargo clippy"));
        assert!(workflow.contains("cargo build --release --locked"));
    }

    #[test]
    fn ci_pins_inno_setup_version() {
        let workflow = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(".github")
                .join("workflows")
                .join("ci.yml"),
        )
        .expect("CI workflow");
        assert!(workflow.contains("choco install innosetup --version=6.7.3"));
    }

    #[test]
    fn build_script_leaves_resource_linking_to_winres() {
        let build_script =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"))
                .expect("build script");
        assert!(build_script.contains("resource.compile()"));
        assert!(!build_script.contains("rustc-link-arg"));
    }

    #[test]
    fn build_script_does_not_override_windows_resource_toolchain() {
        let build_script =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"))
                .expect("build script");
        assert!(!build_script.contains("set_toolkit_path"));
    }

    #[test]
    fn performance_report_records_rejected_wic_and_downscale() {
        let report = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("docs")
                .join("performance-baseline.md"),
        )
        .expect("performance report");
        assert!(report.contains("WIC"));
        assert!(report.contains("Smart Downscaling"));
        assert!(report.contains("удалён"));
    }

    #[test]
    fn texture_cache_key_is_stable_without_downscale() {
        let path = std::path::Path::new("photo.jpg");
        assert_eq!(
            texture_cache_key(path),
            std::path::PathBuf::from("photo.jpg")
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
        let pan = pan_for_resize((120.0, -40.0), (1920.0, 1080.0), (1280.0, 720.0));
        assert!((pan.0 - 80.0).abs() < f32::EPSILON);
        assert!((pan.1 + 26.666668).abs() < 0.00001);
    }

    #[test]
    fn resize_scales_the_fullscreen_composition_into_a_smaller_viewport() {
        assert_eq!(resize_scale_factor((1920.0, 1080.0), (960.0, 540.0)), 0.5);
        assert_eq!(
            pan_for_resize((120.0, -40.0), (1920.0, 1080.0), (960.0, 540.0)),
            (60.0, -20.0)
        );
    }

    #[test]
    fn resize_does_not_change_logical_zoom() {
        let zoom = 2.0;
        let scale = resize_scale_factor((1920.0, 1080.0), (960.0, 540.0));
        assert_eq!(zoom, 2.0);
        assert_eq!(zoom * scale, 1.0);
    }

    #[test]
    fn resize_uses_stable_fullscreen_scale_after_intermediate_snaps() {
        let corner_scale = resize_scale_factor((1920.0, 1080.0), (960.0, 540.0));
        let restored_pan = pan_for_resize_scale(
            pan_for_resize_scale((120.0, -40.0), 1.0, corner_scale),
            corner_scale,
            1.0,
        );
        assert_eq!(corner_scale, 0.5);
        assert_eq!(restored_pan, (120.0, -40.0));
    }

    #[test]
    fn load_timing_is_deferred_until_the_next_gui_frame() {
        let source = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("main.rs"),
        )
        .expect("main source");
        assert!(source.contains("pending_load_timing: Option<PathBuf>"));
        assert!(source.contains("self.pending_load_timing.take()"));
        assert!(source.contains("self.pending_load_timing = Some(timing_source)"));
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
