use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
};

use image::DynamicImage;

pub struct ImageCache {
    capacity: usize,
    entries: VecDeque<(PathBuf, Arc<DynamicImage>)>,
}

impl ImageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::new(),
        }
    }

    pub fn get(&mut self, path: &Path) -> Option<Arc<DynamicImage>> {
        let index = self.entries.iter().position(|(key, _)| key == path)?;
        let entry = self.entries.remove(index)?;
        let image = entry.1.clone();
        self.entries.push_back(entry);
        Some(image)
    }

    pub fn insert(&mut self, path: PathBuf, image: Arc<DynamicImage>) {
        if self.capacity == 0 {
            return;
        }
        if let Some(index) = self.entries.iter().position(|(key, _)| key == &path) {
            self.entries.remove(index);
        }
        self.entries.push_back((path, image));
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }
}
