use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread;

use super::ImageData;

type LoadCallback = Box<dyn FnOnce(Result<ImageData, String>) + Send + 'static>;

struct DecodeTask {
    path: PathBuf,
}

/// In-memory texture cache and asynchronous image streaming loader.
///
/// Deduplicates image decoding and GPU texture allocations across render frames.
pub struct ImageCache {
    entries: RwLock<HashMap<PathBuf, ImageData>>,
    pending: Arc<Mutex<HashMap<PathBuf, Vec<LoadCallback>>>>,
    tx: Mutex<Option<Sender<DecodeTask>>>,
}

static GLOBAL_CACHE: OnceLock<ImageCache> = OnceLock::new();

impl ImageCache {
    /// Returns the global shared `ImageCache` instance.
    pub fn global() -> &'static Self {
        GLOBAL_CACHE.get_or_init(|| Self::new())
    }

    /// Creates a new, isolated `ImageCache` instance.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            pending: Arc::new(Mutex::new(HashMap::new())),
            tx: Mutex::new(None),
        }
    }

    /// Looks up a cached `ImageData` by file path.
    pub fn get<P: AsRef<Path>>(&self, path: P) -> Option<ImageData> {
        self.entries.read().ok()?.get(path.as_ref()).cloned()
    }

    /// Inserts a decoded `ImageData` into the cache under the specified path.
    pub fn insert<P: Into<PathBuf>>(&self, path: P, data: ImageData) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(path.into(), data);
        }
    }

    /// Returns a cached `ImageData` if available, or synchronously loads and caches it from disk.
    pub fn get_or_load<P: AsRef<Path>>(&self, path: P) -> Result<ImageData, String> {
        let p = path.as_ref();
        if let Some(cached) = self.get(p) {
            return Ok(cached);
        }

        let data = ImageData::from_file_uncached(p)?;
        self.insert(p.to_path_buf(), data.clone());
        Ok(data)
    }

    /// Checks if a file is currently being decoded asynchronously in the background.
    pub fn is_loading<P: AsRef<Path>>(&self, path: P) -> bool {
        self.pending
            .lock()
            .map(|p| p.contains_key(path.as_ref()))
            .unwrap_or(false)
    }

    /// Requests background streaming and decoding for an image file without blocking the UI thread.
    ///
    /// If the image is already cached, `on_complete` is invoked immediately on the calling thread.
    /// Otherwise, decoding executes in a background thread and invokes `on_complete` upon finish.
    pub fn load_async<P: AsRef<Path>, F>(&self, path: P, on_complete: Option<F>)
    where
        F: FnOnce(Result<ImageData, String>) + Send + 'static,
    {
        let path_buf = path.as_ref().to_path_buf();

        if let Some(cached) = self.get(&path_buf) {
            if let Some(cb) = on_complete {
                cb(Ok(cached));
            }
            return;
        }

        let mut pending = self.pending.lock().unwrap();
        if let Some(callbacks) = pending.get_mut(&path_buf) {
            if let Some(cb) = on_complete {
                callbacks.push(Box::new(cb));
            }
            return;
        }

        let mut callbacks: Vec<LoadCallback> = Vec::new();
        if let Some(cb) = on_complete {
            callbacks.push(Box::new(cb));
        }

        pending.insert(path_buf.clone(), callbacks);
        drop(pending);

        self.ensure_worker_started();
        if let Ok(guard) = self.tx.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(DecodeTask { path: path_buf });
            }
        }
    }

    /// Clears all cached images from memory.
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }

    fn ensure_worker_started(&self) {
        let mut guard = self.tx.lock().unwrap();
        if guard.is_some() {
            return;
        }

        let (tx, rx): (Sender<DecodeTask>, Receiver<DecodeTask>) = channel();
        *guard = Some(tx);

        let pending_map = Arc::clone(&self.pending);

        thread::Builder::new()
            .name("mtk-image-loader".into())
            .spawn(move || {
                while let Ok(task) = rx.recv() {
                    let path = task.path;
                    let result = ImageData::from_file_uncached(&path);

                    if let Ok(ref data) = result {
                        ImageCache::global().insert(path.clone(), data.clone());
                    }

                    let callbacks = {
                        let mut map = pending_map.lock().unwrap();
                        map.remove(&path).unwrap_or_default()
                    };

                    for cb in callbacks {
                        cb(result.clone());
                    }
                }
            })
            .expect("Failed to spawn mtk-image-loader background thread");
    }
}
