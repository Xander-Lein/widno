use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
};

use image::DynamicImage;

pub const DEFAULT_CAPACITY: usize = 8;
pub const DEFAULT_MAX_BYTES: usize = 512 * 1024 * 1024;

pub struct ImageCache {
    capacity: usize,
    max_bytes: usize,
    current_bytes: usize,
    entries: VecDeque<(PathBuf, Arc<DynamicImage>)>,
}

impl ImageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            max_bytes: DEFAULT_MAX_BYTES,
            current_bytes: 0,
            entries: VecDeque::new(),
        }
    }

    pub fn with_limits(capacity: usize, max_bytes: usize) -> Self {
        Self {
            capacity,
            max_bytes,
            current_bytes: 0,
            entries: VecDeque::new(),
        }
    }

    pub fn get(&mut self, path: &Path) -> Option<Arc<DynamicImage>> {
        let index = self.entries.iter().position(|(key, _)| key == path)?;
        let entry = self.entries.remove(index)?;
        self.current_bytes = self.current_bytes.saturating_sub(entry.1.as_bytes().len());
        let image = entry.1.clone();
        self.current_bytes += image.as_bytes().len();
        self.entries.push_back(entry);
        Some(image)
    }

    pub fn insert(&mut self, path: PathBuf, image: Arc<DynamicImage>) {
        if self.capacity == 0 {
            return;
        }
        if let Some(index) = self.entries.iter().position(|(key, _)| key == &path) {
            if let Some((_, previous)) = self.entries.remove(index) {
                self.current_bytes = self.current_bytes.saturating_sub(previous.as_bytes().len());
            }
        }
        let image_bytes = image.as_bytes().len();
        if image_bytes > self.max_bytes {
            return;
        }
        self.current_bytes += image_bytes;
        self.entries.push_back((path, image));
        while self.entries.len() > self.capacity || self.current_bytes > self.max_bytes {
            if let Some((_, oldest)) = self.entries.pop_front() {
                self.current_bytes = self.current_bytes.saturating_sub(oldest.as_bytes().len());
            }
        }
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }
}
