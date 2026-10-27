#![cfg_attr(windows, windows_subsystem = "windows")]

use std::{
    collections::VecDeque,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use eframe::{
    egui::{self, ColorImage, Key, Pos2, TextureHandle, TextureOptions, Vec2},
    App, CreationContext, Frame,
};
use widno::{
    archive,
    cache::{ImageCache, DEFAULT_CAPACITY},
    media,
    metadata::{format_metadata, HEADER_HEIGHT},
    settings::{Settings, IMAGE_FORMATS},
    viewer::{
        is_supported_image_path, load_archive_entry_async, load_image_async_with_password,
        load_pdf_page_async, next_index, pan_for_drag, pan_for_resize, reset_view,
        load_elapsed_ms, smart_texture_dimensions, texture_cache_key, zoom_pan_for_cursor,
        zoom_step, LoadedImage, ViewerError,
    },
    window::{mark_native_maximized, native_snap_style},
};

type ExifResult = Result<Vec<(String, String)>, String>;

#[cfg(windows)]
fn focus_widno_window(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::{
        System::Threading::{AttachThreadInput, GetCurrentThreadId},
        UI::Input::KeyboardAndMouse::{SetActiveWindow, SetFocus},
        UI::WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
        },
    };

    let foreground = unsafe { GetForegroundWindow() };
    let mut foreground_pid = 0;
    let foreground_thread = if foreground.is_null() {
        0
    } else {
        unsafe { GetWindowThreadProcessId(foreground, &mut foreground_pid) }
    };
    let current_thread = unsafe { GetCurrentThreadId() };
    let attached = foreground_thread != 0 && foreground_thread != current_thread;
    if attached {
        unsafe {
            AttachThreadInput(current_thread, foreground_thread, 1);
        }
    }
    unsafe {
        BringWindowToTop(hwnd);
        SetActiveWindow(hwnd);
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);
    }
    if attached {
        unsafe {
            AttachThreadInput(current_thread, foreground_thread, 0);
        }
    }
}

#[cfg(windows)]
fn maximize_widno_window() {
    let hwnd = widno_window_handle();
    if !hwnd.is_null() {
        sync_native_maximized_placement(hwnd);
        focus_widno_window(hwnd);
    }
}

#[cfg(windows)]
fn sync_native_maximized_placement(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GetWindowPlacement, SetWindowLongPtrW, SetWindowPlacement, ShowWindow,
        GWL_STYLE, SW_MAXIMIZE, WINDOWPLACEMENT,
    };

    let mut placement = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        flags: 0,
        showCmd: SW_MAXIMIZE as u32,
        ptMinPosition: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
        ptMaxPosition: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
        rcNormalPosition: windows_sys::Win32::Foundation::RECT::default(),
    };
    unsafe {
        if GetWindowPlacement(hwnd, &mut placement) != 0 {
            placement.showCmd = SW_MAXIMIZE as u32;
            SetWindowPlacement(hwnd, &placement);
        }
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_STYLE, mark_native_maximized(style) as isize);
        ShowWindow(hwnd, SW_MAXIMIZE);
    }
}

#[cfg(windows)]
fn configure_native_snap_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    };

    let hwnd = widno_window_handle();
    if !hwnd.is_null() {
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };
        let new_style = native_snap_style(style);
        if style != new_style {
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
                );
            }
        }
        sync_native_maximized_placement(hwnd);
    }
}

#[cfg(not(windows))]
fn configure_native_snap_window() {}

#[cfg(not(windows))]
fn maximize_widno_window() {}

#[cfg(windows)]
fn widno_window_handle() -> windows_sys::Win32::Foundation::HWND {
    use windows_sys::{
        core::BOOL,
        Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId},
    };

    unsafe extern "system" fn find_window(
        hwnd: windows_sys::Win32::Foundation::HWND,
        parameter: isize,
    ) -> BOOL {
        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == std::process::id() {
            *(parameter as *mut windows_sys::Win32::Foundation::HWND) = hwnd;
            0
        } else {
            1
        }
    }

    let mut hwnd = std::ptr::null_mut();
    unsafe {
        EnumWindows(Some(find_window), &mut hwnd as *mut _ as isize);
    }
    hwnd
}

struct WidnoApp {
    image: Option<LoadedImage>,
    texture: Option<TextureHandle>,
    texture_cache: VecDeque<(PathBuf, TextureHandle)>,
    loading: Option<Receiver<Result<LoadedImage, ViewerError>>>,
    loading_started_at: Option<Instant>,
    exif_loading: Option<Receiver<ExifResult>>,
    pending_path: Option<PathBuf>,
    password: String,
    error: Option<String>,
    siblings: Vec<PathBuf>,
    current_index: usize,
    archive_path: Option<PathBuf>,
    archive_entries: Vec<archive::ArchiveEntry>,
    archive_index: usize,
    archive_password: Option<String>,
    pdf_path: Option<PathBuf>,
    pdf_page: usize,
    pdf_page_count: usize,
    metadata_name: String,
    metadata_size: u64,
    exif: Vec<(String, String)>,
    show_exif: bool,
    zoom: f32,
    fit_to_window: bool,
    pan: Vec2,
    drag_start_pan: Vec2,
    drag_start_pointer: Option<Pos2>,
    chrome_visible: bool,
    last_input: Instant,
    image_cache: ImageCache,
    cache_key: Option<PathBuf>,
    prefetch_loading: Option<Receiver<Result<LoadedImage, ViewerError>>>,
    prefetch_key: Option<PathBuf>,
    startup_maximized: bool,
    native_snap_configured: bool,
    settings: Settings,
    settings_path: PathBuf,
    settings_open: bool,
    last_viewport_size: Option<Vec2>,
}

impl WidnoApp {
    fn new(cc: &CreationContext<'_>) -> Self {
        let (settings, settings_path) = Settings::load();
        let settings_open = !settings.initialized;
        let mut app = Self {
            image: None,
            texture: None,
            texture_cache: VecDeque::new(),
            loading: None,
            loading_started_at: None,
            exif_loading: None,
            pending_path: None,
            password: String::new(),
            error: None,
            siblings: Vec::new(),
            current_index: 0,
            archive_path: None,
            archive_entries: Vec::new(),
            archive_index: 0,
            archive_password: None,
            pdf_path: None,
            pdf_page: 0,
            pdf_page_count: 0,
            metadata_name: String::new(),
            metadata_size: 0,
            exif: Vec::new(),
            show_exif: false,
            zoom: 1.0,
            fit_to_window: true,
            pan: Vec2::ZERO,
            drag_start_pan: Vec2::ZERO,
            drag_start_pointer: None,
            chrome_visible: true,
            last_input: Instant::now(),
            image_cache: ImageCache::new(DEFAULT_CAPACITY),
            cache_key: None,
            prefetch_loading: None,
            prefetch_key: None,
            startup_maximized: true,
            native_snap_configured: false,
            settings,
            settings_path,
            settings_open,
            last_viewport_size: None,
        };

        if let Some(path) = std::env::args_os().nth(1) {
            app.request_image(PathBuf::from(path));
        }

        cc.egui_ctx.set_pixels_per_point(1.0);
        app
    }

    fn request_image(&mut self, path: PathBuf) {
        self.request_image_with_password(path, None);
    }

    fn request_image_with_password(&mut self, path: PathBuf, password: Option<String>) {
        if archive::archive_kind(&path).is_some() {
            self.archive_path = Some(path.clone());
            self.archive_entries.clear();
            self.archive_index = 0;
            self.archive_password = password.clone();
        } else {
            self.archive_path = None;
            self.archive_entries.clear();
            self.archive_index = 0;
            self.archive_password = None;
        }
        self.metadata_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Widno")
            .to_string();
        self.metadata_size = fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if media::is_pdf_path(&path) {
            self.pdf_path = Some(path.clone());
            self.pdf_page = 0;
            self.pdf_page_count = media::pdf_page_count(&path).unwrap_or(0);
        } else {
            self.pdf_path = None;
            self.pdf_page = 0;
            self.pdf_page_count = 0;
        }
        if path.is_file() {
            if let Some(parent) = path.parent() {
                let mut siblings: Vec<_> = fs::read_dir(parent)
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|candidate| is_supported_image_path(candidate))
                    .collect();
                siblings.sort();
                self.current_index = siblings
                    .iter()
                    .position(|candidate| candidate == &path)
                    .unwrap_or(0);
                self.siblings = siblings;
            }
        }
        self.error = None;
        self.exif.clear();
        self.exif_loading = None;
        self.pan = Vec2::ZERO;
        self.drag_start_pointer = None;
        self.pending_path = Some(path.clone());
        self.prefetch_loading = None;
        self.prefetch_key = None;
        self.cache_key = if archive::archive_kind(&path).is_none() {
            Some(path.clone())
        } else {
            None
        };
        let image_cached = self
            .cache_key
            .as_deref()
            .and_then(|key| self.image_cache.get(key))
            .is_some();
        self.loading = if image_cached {
            None
        } else {
            Some(load_image_async_with_password(path, password))
        };
        self.loading_started_at = Some(Instant::now());
        self.last_input = Instant::now();
    }

    fn request_archive_entry(&mut self, index: usize) {
        let (Some(path), Some(entry)) = (
            self.archive_path.clone(),
            self.archive_entries.get(index).cloned(),
        ) else {
            return;
        };
        self.archive_index = index;
        self.metadata_name = entry.name.clone();
        self.metadata_size = entry.size.unwrap_or(0);
        self.error = None;
        self.exif.clear();
        self.exif_loading = None;
        self.pan = Vec2::ZERO;
        self.drag_start_pointer = None;
        self.pending_path = Some(path.clone());
        self.prefetch_loading = None;
        self.prefetch_key = None;
        self.cache_key = Some(PathBuf::from(format!("{}::{}", path.display(), entry.name)));
        let image_cached = self
            .cache_key
            .as_deref()
            .and_then(|key| self.image_cache.get(key))
            .is_some();
        self.loading = if image_cached {
            None
        } else {
            Some(load_archive_entry_async(
                path,
                entry,
                self.archive_password.clone(),
            ))
        };
        self.loading_started_at = Some(Instant::now());
        self.last_input = Instant::now();
    }

    fn request_pdf_page(&mut self, path: PathBuf, page: usize) {
        self.error = None;
        self.exif_loading = None;
        self.cache_key = None;
        self.prefetch_loading = None;
        self.prefetch_key = None;
        self.pending_path = Some(path.clone());
        self.loading = Some(load_pdf_page_async(path, page));
        self.loading_started_at = Some(Instant::now());
        self.last_input = Instant::now();
    }

    fn start_prefetch(&mut self) {
        if self.prefetch_loading.is_some() {
            return;
        }
        if let (Some(path), Some(index)) = (
            self.archive_path.clone(),
            next_index(self.archive_index, self.archive_entries.len(), 1),
        ) {
            if let Some(entry) = self.archive_entries.get(index).cloned() {
                let key = PathBuf::from(format!("{}::{}", path.display(), entry.name));
                if self.image_cache.get(&key).is_none() {
                    self.prefetch_key = Some(key);
                    self.prefetch_loading = Some(load_archive_entry_async(
                        path,
                        entry,
                        self.archive_password.clone(),
                    ));
                }
            }
            return;
        }
        if let Some(index) = next_index(self.current_index, self.siblings.len(), 1) {
            if let Some(path) = self.siblings.get(index).cloned() {
                if self.image_cache.get(&path).is_none() {
                    self.prefetch_key = Some(path.clone());
                    self.prefetch_loading = Some(load_image_async_with_password(path, None));
                }
            }
        }
    }

    fn poll_prefetch(&mut self) {
        let Some(receiver) = &self.prefetch_loading else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(image)) => {
                self.image_cache.insert(image.source, image.image);
                self.prefetch_loading = None;
                self.prefetch_key = None;
            }
            Ok(Err(_)) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.prefetch_loading = None;
                self.prefetch_key = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    fn cached_texture(&mut self, key: &PathBuf) -> Option<TextureHandle> {
        let index = self
            .texture_cache
            .iter()
            .position(|(cached_key, _)| cached_key == key)?;
        let entry = self.texture_cache.remove(index)?;
        let texture = entry.1.clone();
        self.texture_cache.push_back(entry);
        Some(texture)
    }

    fn cache_texture(&mut self, key: PathBuf, texture: TextureHandle) {
        if let Some(index) = self
            .texture_cache
            .iter()
            .position(|(cached_key, _)| cached_key == &key)
        {
            self.texture_cache.remove(index);
        }
        self.texture_cache.push_back((key, texture));
        while self.texture_cache.len() > DEFAULT_CAPACITY {
            self.texture_cache.pop_front();
        }
    }

    fn install_loaded_image(&mut self, ctx: &egui::Context, image: LoadedImage) {
        let timing_source = image.source.clone();
        let archive_entry = image.archive_entry.clone();
        let archive_entries = image.archive_entries.clone();
        let viewport = ctx.screen_rect().size();
        let viewport_key = (viewport.x.max(1.0) as u32, viewport.y.max(1.0) as u32);
        let texture_key = texture_cache_key(&image.source, viewport_key);
        let texture = if let Some(texture) = self.cached_texture(&texture_key) {
            texture
        } else {
            let native_dimensions = (image.image.width(), image.image.height());
            let texture_dimensions = smart_texture_dimensions(native_dimensions, viewport_key);
            let texture_image = if texture_dimensions != native_dimensions {
                image.image.resize(
                    texture_dimensions.0,
                    texture_dimensions.1,
                    image::imageops::FilterType::Triangle,
                )
            } else {
                image.image.as_ref().clone()
            };
            let rgba = texture_image.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let texture = ctx.load_texture(
                image.source.to_string_lossy(),
                color_image,
                TextureOptions::LINEAR,
            );
            self.cache_texture(texture_key, texture.clone());
            texture
        };
        self.texture = Some(texture);
        self.image_cache
            .insert(image.source.clone(), image.image.clone());
        self.image = Some(image);
        self.fit_to_window = true;
        if let Some(entries) = archive_entries {
            self.archive_entries = entries;
        }
        if let Some(entry) = archive_entry {
            self.metadata_name = entry.name.clone();
            self.metadata_size = entry.size.unwrap_or(0);
            self.archive_index = self
                .archive_entries
                .iter()
                .position(|candidate| candidate.name == entry.name)
                .unwrap_or(0);
        }
        self.start_exif_load();
        self.start_prefetch();
        self.record_load_timing(&timing_source);
    }

    fn record_load_timing(&mut self, source: &Path) {
        let Some(start) = self.loading_started_at.take() else {
            return;
        };
        if std::env::var_os("WIDNO_LOAD_TIMING").is_none() {
            return;
        }
        let elapsed_ms = load_elapsed_ms(start, Instant::now());
        let source_kind = if source.to_string_lossy().contains("::") {
            "archive"
        } else {
            "file"
        };
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let path = std::env::temp_dir().join("widno-load-times.csv");
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{timestamp_ms},{elapsed_ms},{source_kind}");
        }
    }

    fn start_exif_load(&mut self) {
        let Some(path) = self
            .image
            .as_ref()
            .map(|image| image.source.clone())
            .filter(|path| path.is_file())
        else {
            self.exif_loading = None;
            return;
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = media::read_exif(&path).map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
        self.exif_loading = Some(receiver);
    }

    fn poll_exif(&mut self) {
        let Some(receiver) = &self.exif_loading else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(exif)) => {
                self.exif = exif;
                self.exif_loading = None;
            }
            Ok(Err(_)) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.exif_loading = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    fn poll_image(&mut self, ctx: &egui::Context) {
        self.poll_exif();
        self.poll_prefetch();
        if let Some(key) = self.cache_key.clone() {
            if let Some(image) = self.image_cache.get(&key) {
                self.install_loaded_image(
                    ctx,
                    LoadedImage {
                        source: key,
                        image,
                        archive_entry: None,
                        archive_entries: None,
                    },
                );
                self.loading = None;
                self.pending_path = None;
                self.cache_key = None;
                return;
            }
        }

        let Some(receiver) = &self.loading else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(image)) => {
                self.install_loaded_image(ctx, image);
                self.loading = None;
                self.pending_path = None;
                self.cache_key = None;
            }
            Ok(Err(error)) => {
                self.error = Some(error.to_string());
                self.texture = None;
                self.image = None;
                self.loading = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.error = Some("image loading worker stopped unexpectedly".to_string());
                self.loading = None;
            }
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let mut changed = false;
        let mut navigation = 0;
        let mut save_requested = false;
        let viewport = ctx.screen_rect().size();
        if let Some(previous) = self.last_viewport_size {
            if !self.fit_to_window && (previous - viewport).length_sq() > f32::EPSILON {
                let pan = pan_for_resize(
                    (self.pan.x, self.pan.y),
                    (previous.x, previous.y),
                    (viewport.x, viewport.y),
                );
                self.pan = Vec2::new(pan.0, pan.1);
            }
        }
        self.last_viewport_size = Some(viewport);
        ctx.input(|input| {
            if input.key_pressed(Key::Escape) {
                std::process::exit(0);
            }
            // The viewer deliberately stays borderless-windowed so Windows snapping
            // never transitions it back to an inconsistent fullscreen state.
            if input.modifiers.ctrl && input.key_pressed(Key::Num0) {
                self.fit_to_window = true;
                let (pan, zoom) = reset_view();
                self.pan = Vec2::new(pan.0, pan.1);
                self.zoom = zoom;
                self.drag_start_pan = Vec2::ZERO;
                self.drag_start_pointer = None;
                changed = true;
            }
            if input.modifiers.ctrl
                && (input.key_pressed(Key::Plus) || input.key_pressed(Key::Equals))
            {
                let old_zoom = self.zoom;
                self.fit_to_window = false;
                self.zoom = zoom_step(self.zoom, 1);
                if let Some(cursor) = input.pointer.hover_pos() {
                    let center = input.screen_rect().center();
                    let pan = zoom_pan_for_cursor(
                        (self.pan.x, self.pan.y),
                        old_zoom,
                        self.zoom,
                        (cursor.x, cursor.y),
                        (center.x, center.y),
                    );
                    self.pan = Vec2::new(pan.0, pan.1);
                }
                changed = true;
            }
            if input.modifiers.ctrl && input.key_pressed(Key::Minus) {
                let old_zoom = self.zoom;
                self.fit_to_window = false;
                self.zoom = zoom_step(self.zoom, -1);
                if let Some(cursor) = input.pointer.hover_pos() {
                    let center = input.screen_rect().center();
                    let pan = zoom_pan_for_cursor(
                        (self.pan.x, self.pan.y),
                        old_zoom,
                        self.zoom,
                        (cursor.x, cursor.y),
                        (center.x, center.y),
                    );
                    self.pan = Vec2::new(pan.0, pan.1);
                }
                changed = true;
            }
            if input.modifiers.ctrl && input.key_pressed(Key::S) {
                save_requested = true;
            }
            if input.modifiers.ctrl && input.key_pressed(Key::P) {
                self.settings_open = true;
                changed = true;
            }
            if input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::PageDown) {
                navigation = 1;
            }
            if input.key_pressed(Key::ArrowLeft) || input.key_pressed(Key::PageUp) {
                navigation = -1;
            }
            if input.key_pressed(Key::E) {
                self.show_exif = !self.show_exif;
                changed = true;
            }
            let pinch = input.zoom_delta();
            if (pinch - 1.0).abs() > f32::EPSILON {
                let old_zoom = self.zoom;
                self.fit_to_window = false;
                self.zoom = (self.zoom * pinch).clamp(0.05, 32.0);
                if let Some(cursor) = input.pointer.hover_pos() {
                    let center = input.screen_rect().center();
                    let pan = zoom_pan_for_cursor(
                        (self.pan.x, self.pan.y),
                        old_zoom,
                        self.zoom,
                        (cursor.x, cursor.y),
                        (center.x, center.y),
                    );
                    self.pan = Vec2::new(pan.0, pan.1);
                }
                changed = true;
            }
            if self.zoom > 1.0 && input.raw_scroll_delta != Vec2::ZERO {
                self.pan += input.raw_scroll_delta;
                changed = true;
            }
            if input.key_pressed(Key::Q) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        if ctx.input(|input| input.viewport().close_requested()) {
            std::process::exit(0);
        }

        if save_requested {
            self.save_current_image();
        }

        if navigation != 0 {
            if let Some(path) = self.pdf_path.clone() {
                let next_page = if navigation < 0 {
                    self.pdf_page.saturating_sub(1)
                } else {
                    self.pdf_page.saturating_add(1)
                };
                if next_page < self.pdf_page_count {
                    self.pdf_page = next_page;
                    self.request_pdf_page(path, next_page);
                }
            } else if let Some(index) =
                next_index(self.archive_index, self.archive_entries.len(), navigation)
            {
                if self.archive_path.is_some() {
                    self.request_archive_entry(index);
                }
            } else if let Some(index) =
                next_index(self.current_index, self.siblings.len(), navigation)
            {
                if let Some(path) = self.siblings.get(index).cloned() {
                    self.current_index = index;
                    self.request_image(path);
                }
            }
        }

        if changed {
            self.last_input = Instant::now();
            self.chrome_visible = true;
        } else if self.last_input.elapsed() > Duration::from_secs(2) {
            self.chrome_visible = false;
        }
    }

    fn image_size(&self, available: Vec2) -> Option<Vec2> {
        let image = self.image.as_ref()?;
        let native = Vec2::new(image.image.width() as f32, image.image.height() as f32);
        if self.fit_to_window {
            let scale = (available.x / native.x)
                .min(available.y / native.y)
                .min(1.0);
            Some(native * scale.max(0.01))
        } else {
            Some(native * self.zoom)
        }
    }

    fn save_current_image(&self) {
        let Some(image) = &self.image else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Images",
                &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"],
            )
            .set_file_name(&self.metadata_name)
            .save_file()
        else {
            return;
        };
        let format = match path.extension().and_then(|extension| extension.to_str()) {
            Some(extension) if extension.eq_ignore_ascii_case("png") => image::ImageFormat::Png,
            Some(extension)
                if extension.eq_ignore_ascii_case("jpg")
                    || extension.eq_ignore_ascii_case("jpeg") =>
            {
                image::ImageFormat::Jpeg
            }
            Some(extension) if extension.eq_ignore_ascii_case("webp") => image::ImageFormat::WebP,
            Some(extension) if extension.eq_ignore_ascii_case("bmp") => image::ImageFormat::Bmp,
            Some(extension)
                if extension.eq_ignore_ascii_case("tif")
                    || extension.eq_ignore_ascii_case("tiff") =>
            {
                image::ImageFormat::Tiff
            }
            _ => return,
        };
        if let Err(error) = image.image.save_with_format(path, format) {
            eprintln!("save failed: {error}");
        }
    }

    fn show_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut save = false;
        egui::Window::new("Настройки Widno")
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.heading("Open files with Widno by default");
                ui.label("Выберите форматы изображений для Widno и OpenWithProgids.");
                ui.separator();
                ui.columns(3, |columns| {
                    for (index, extension) in IMAGE_FORMATS.iter().enumerate() {
                        let enabled = self
                            .settings
                            .enabled_formats
                            .entry((*extension).to_string())
                            .or_insert(true);
                        columns[index % 3].checkbox(enabled, *extension);
                    }
                });
                ui.separator();
                ui.heading("Add Widno to context menu");
                ui.checkbox(&mut self.settings.context_files, "Files");
                ui.checkbox(&mut self.settings.context_archives, "Archives");
                ui.checkbox(&mut self.settings.context_folders, "Folders");
                ui.separator();
                ui.label("Изменение приложения по умолчанию выполняется только после подтверждения Windows.");
                if ui.button("Открыть настройки приложений Windows").clicked() {
                    #[cfg(windows)]
                    {
                        let _ = std::process::Command::new("cmd")
                            .args(["/C", "start", "", "ms-settings:defaultapps"])
                            .spawn();
                    }
                }
                if ui.button("Сохранить").clicked() {
                    save = true;
                }
            });
        if save {
            self.settings.initialized = true;
            if let Err(error) = widno::settings::apply_default_associations(&self.settings) {
                self.error = Some(format!("Не удалось применить ассоциации: {error}"));
            }
            if let Err(error) = self.settings.save(&self.settings_path) {
                self.error = Some(format!("Не удалось сохранить настройки: {error}"));
            }
            self.settings_open = false;
        }
    }

    fn metadata_text(&self) -> String {
        let dimensions = self
            .image
            .as_ref()
            .map(|image| (image.image.width(), image.image.height()))
            .unwrap_or_default();
        let position = if self.pdf_page_count > 0 {
            format!("{}/{}", self.pdf_page + 1, self.pdf_page_count)
        } else if !self.archive_entries.is_empty() {
            format!("{}/{}", self.archive_index + 1, self.archive_entries.len())
        } else if self.siblings.is_empty() {
            "—/—".to_string()
        } else {
            format!("{}/{}", self.current_index + 1, self.siblings.len())
        };
        format_metadata(dimensions, self.metadata_size, &position, self.zoom)
    }
}

impl App for WidnoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.poll_image(ctx);
        if !self.native_snap_configured {
            configure_native_snap_window();
            self.native_snap_configured = true;
        }
        if self.startup_maximized {
            maximize_widno_window();
            self.startup_maximized = false;
        }
        self.handle_input(ctx);

        let dropped_files = ctx.input(|input| input.raw.dropped_files.clone());
        if let Some(file) = dropped_files.first().and_then(|file| file.path.clone()) {
            self.request_image(file);
        }

        let mut retry_path = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                let (header_rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), HEADER_HEIGHT),
                    egui::Sense::hover(),
                );
                let font = egui::FontId::proportional(12.0);
                let left_header = egui::Rect::from_min_max(
                    header_rect.min,
                    egui::pos2(header_rect.center().x, header_rect.max.y),
                );
                let right_header = egui::Rect::from_min_max(
                    egui::pos2(header_rect.center().x, header_rect.min.y),
                    header_rect.max,
                );
                ui.painter().with_clip_rect(left_header).text(
                    header_rect.left_top(),
                    egui::Align2::LEFT_TOP,
                    self.metadata_text(),
                    font.clone(),
                    egui::Color32::LIGHT_GREEN,
                );
                if self.loading.is_some() {
                    ui.painter().text(
                        header_rect.center_top(),
                        egui::Align2::CENTER_TOP,
                        "Загрузка…",
                        font.clone(),
                        egui::Color32::YELLOW,
                    );
                }
                if let Some(path) = self
                    .image
                    .as_ref()
                    .map(|image| image.source.to_string_lossy())
                {
                    ui.painter().with_clip_rect(right_header).text(
                        header_rect.right_top(),
                        egui::Align2::RIGHT_TOP,
                        path,
                        font,
                        egui::Color32::LIGHT_GREEN,
                    );
                }

                let available = ui.available_size();
                if let Some(texture) = &self.texture {
                    if let Some(size) = self.image_size(available) {
                        let (rect, response) =
                            ui.allocate_exact_size(available, egui::Sense::drag());
                        let pan_enabled =
                            self.zoom > 1.0 || ctx.input(|input| input.key_down(Key::Space));
                        if response.drag_started() {
                            self.drag_start_pan = self.pan;
                            self.drag_start_pointer = response.interact_pointer_pos();
                        }
                        if pan_enabled && response.dragged() {
                            if let (Some(start), Some(current)) =
                                (self.drag_start_pointer, response.interact_pointer_pos())
                            {
                                let pan = pan_for_drag(
                                    (self.drag_start_pan.x, self.drag_start_pan.y),
                                    (start.x, start.y),
                                    (current.x, current.y),
                                );
                                self.pan = Vec2::new(pan.0, pan.1);
                            }
                            self.fit_to_window = false;
                        } else {
                            self.drag_start_pointer = None;
                        }
                        let center = rect.center() + self.pan;
                        let image_rect = egui::Rect::from_center_size(center, size);
                        ui.painter().with_clip_rect(rect).image(
                            texture.id(),
                            image_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                } else if let Some(error) = &self.error {
                    ui.centered_and_justified(|ui| {
                        ui.vertical(|ui| {
                            ui.colored_label(egui::Color32::LIGHT_RED, error);
                            if self
                                .pending_path
                                .as_deref()
                                .and_then(archive::archive_kind)
                                .is_some()
                            {
                                ui.horizontal(|ui| {
                                    ui.label("Пароль:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .password(true),
                                    );
                                    if ui.button("Повторить").clicked() {
                                        retry_path = self.pending_path.clone();
                                    }
                                });
                            }
                        });
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            "Перетащите изображение в окно или запустите widno.exe с путём к файлу",
                        );
                    });
                }

                if self.show_exif && !self.exif.is_empty() {
                    egui::Window::new("EXIF")
                        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
                        .resizable(true)
                        .show(ctx, |ui| {
                            for (tag, value) in &self.exif {
                                ui.horizontal_wrapped(|ui| {
                                    ui.strong(tag);
                                    ui.label(value);
                                });
                            }
                        });
                }
            });

        self.show_settings(ctx);

        if let Some(path) = retry_path {
            self.request_image_with_password(path, Some(self.password.clone()));
        }

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result<()> {
    let icon = image::load_from_memory(include_bytes!("../assets/widno-icon.png"))
        .expect("embedded Widno icon")
        .to_rgba8();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("widno")
            .with_decorations(false)
            .with_resizable(true)
            .with_inner_size(Vec2::new(1100.0, 750.0))
            .with_maximized(true)
            .with_fullscreen(false)
            .with_icon(eframe::egui::IconData {
                width: icon.width(),
                height: icon.height(),
                rgba: icon.into_raw(),
            }),
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "Widno",
        options,
        Box::new(|cc| Ok(Box::new(WidnoApp::new(cc)))),
    )
}
