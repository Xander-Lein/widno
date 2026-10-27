pub const IMAGE_FORMATS: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tif", ".tiff", ".webp", ".svg", ".ico", ".cur",
    ".dib", ".j2c", ".j2k", ".jp2", ".pbm", ".pcx", ".pgm", ".ppm", ".tga", ".arw", ".cr2", ".cr3",
    ".crw", ".dng", ".mrw", ".nef", ".nrw", ".orf", ".pef", ".raf", ".rwl", ".rw2", ".sr2", ".srf",
    ".srw", ".x3f",
];

pub const ARCHIVE_FORMATS: &[&str] = &[".zip", ".rar", ".7z", ".cbz", ".cbr", ".cb7"];

pub fn context_extensions() -> Vec<&'static str> {
    IMAGE_FORMATS
        .iter()
        .chain(ARCHIVE_FORMATS.iter())
        .copied()
        .collect()
}
