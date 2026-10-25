#![cfg_attr(windows, windows_subsystem = "windows")]

use std::{
    fs,
    path::PathBuf,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use eframe::{
    egui::{self, ColorImage, Key, TextureHandle, TextureOptions, Vec2},
    App, CreationContext, Frame,
};
use widno::{
    archive, media,
    viewer::{
        is_supported_image_path, load_image_async_with_password, load_pdf_page_async, next_index,
        LoadedImage, ViewerError,
    },
};

struct WidnoApp {
    image: Option<LoadedImage>,
    texture: Option<TextureHandle>,
    loading: Option<Receiver<Result<LoadedImage, ViewerError>>>,
    pending_path: Option<PathBuf>,
    password: String,
    error: Option<String>,
    siblings: Vec<PathBuf>,
    current_index: usize,
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
    last_drag_delta: Vec2,
    chrome_visible: bool,
    last_input: Instant,
}

impl WidnoApp {
    fn new(cc: &CreationContext<'_>) -> Self {
        let mut app = Self {
            image: None,
            texture: None,
            loading: None,
            pending_path: None,
            password: String::new(),
            error: None,
            siblings: Vec::new(),
            current_index: 0,
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
            last_drag_delta: Vec2::ZERO,
            chrome_visible: true,
            last_input: Instant::now(),
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
        self.pan = Vec2::ZERO;
        self.last_drag_delta = Vec2::ZERO;
        self.pending_path = Some(path.clone());
        self.loading = Some(load_image_async_with_password(path, password));
        self.last_input = Instant::now();
    }

    fn request_pdf_page(&mut self, path: PathBuf, page: usize) {
        self.error = None;
        self.pending_path = Some(path.clone());
        self.loading = Some(load_pdf_page_async(path, page));
        self.last_input = Instant::now();
    }

    fn poll_image(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.loading else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(image)) => {
                let rgba = image.image.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.texture = Some(ctx.load_texture(
                    image.source.to_string_lossy(),
                    color_image,
                    TextureOptions::LINEAR,
                ));
                self.image = Some(image);
                self.loading = None;
                self.pending_path = None;
                self.fit_to_window = true;
                if let Some(image) = &self.image {
                    if image.source.is_file() {
                        self.exif = media::read_exif(&image.source).unwrap_or_default();
                    }
                }
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
        ctx.input(|input| {
            if input.key_pressed(Key::Escape) {
                std::process::exit(0);
            }
            if input.key_pressed(Key::F11) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                    !input.viewport().fullscreen.unwrap_or(false),
                ));
                changed = true;
            }
            if input.modifiers.ctrl && input.key_pressed(Key::Num0) {
                self.fit_to_window = true;
                self.zoom = 1.0;
                changed = true;
            }
            if input.modifiers.ctrl
                && (input.key_pressed(Key::Plus) || input.key_pressed(Key::Equals))
            {
                self.fit_to_window = false;
                self.zoom = (self.zoom * 1.2).min(32.0);
                changed = true;
            }
            if input.modifiers.ctrl && input.key_pressed(Key::Minus) {
                self.fit_to_window = false;
                self.zoom = (self.zoom / 1.2).max(0.05);
                changed = true;
            }
            if input.modifiers.ctrl && input.key_pressed(Key::S) {
                save_requested = true;
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
            if input.raw_scroll_delta.y.abs() > f32::EPSILON {
                self.fit_to_window = false;
                self.zoom =
                    (self.zoom * (1.0 + input.raw_scroll_delta.y * 0.001)).clamp(0.05, 32.0);
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

    fn metadata_text(&self) -> String {
        let dimensions = self
            .image
            .as_ref()
            .map(|image| format!("{}×{}", image.image.width(), image.image.height()))
            .unwrap_or_else(|| "—".to_string());
        let position = if self.pdf_page_count > 0 {
            format!("{}/{}", self.pdf_page + 1, self.pdf_page_count)
        } else if self.siblings.is_empty() {
            "—/—".to_string()
        } else {
            format!("{}/{}", self.current_index + 1, self.siblings.len())
        };
        format!(
            "{}  |  {}  |  {}  |  {}  |  zoom {:.0}%",
            self.metadata_name,
            dimensions,
            format_bytes(self.metadata_size),
            position,
            self.zoom * 100.0
        )
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

impl App for WidnoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.poll_image(ctx);
        self.handle_input(ctx);

        let dropped_files = ctx.input(|input| input.raw.dropped_files.clone());
        if let Some(file) = dropped_files.first().and_then(|file| file.path.clone()) {
            self.request_image(file);
        }

        let mut retry_path = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                ui.colored_label(egui::Color32::LIGHT_GREEN, self.metadata_text());
                if self.chrome_visible {
                    ui.horizontal(|ui| {
                        ui.label(
                            self.image
                                .as_ref()
                                .map(|image| image.source.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Widno".to_string()),
                        );
                        if ui.button("Fit").clicked() {
                            self.fit_to_window = true;
                            self.last_input = Instant::now();
                        }
                        if ui.button("100%").clicked() {
                            self.fit_to_window = false;
                            self.zoom = 1.0;
                            self.last_input = Instant::now();
                        }
                    });
                }
                if self.loading.is_some() {
                    ui.colored_label(egui::Color32::YELLOW, "Загрузка…");
                }

                let available = ui.available_size();
                if let Some(texture) = &self.texture {
                    if let Some(size) = self.image_size(available) {
                        let (rect, response) =
                            ui.allocate_exact_size(available, egui::Sense::drag());
                        if response.dragged() {
                            let drag_delta = response.drag_delta();
                            self.pan += drag_delta - self.last_drag_delta;
                            self.last_drag_delta = drag_delta;
                            self.fit_to_window = false;
                        } else {
                            self.last_drag_delta = Vec2::ZERO;
                        }
                        let center = rect.center() + self.pan;
                        let image_rect = egui::Rect::from_center_size(center, size);
                        ui.painter().image(
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
            .with_fullscreen(true)
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
