use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread;

use super::ImageData;

type LoadCallback = Box<dyn FnOnce(Result<ImageData, String>) + Send + 'static>;

/// Unique cache key identifying an image path and its target thumbnail bounds.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub path: PathBuf,
    pub max_dim: Option<(u32, u32)>,
}

impl From<PathBuf> for CacheKey {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            max_dim: None,
        }
    }
}

impl From<&Path> for CacheKey {
    fn from(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            max_dim: None,
        }
    }
}

struct CacheEntry {
    data: ImageData,
    byte_size: usize,
    last_accessed: u64,
}

struct DecodeTask {
    key: CacheKey,
}

/// In-memory byte-bounded LRU texture cache and background streaming loader.
///
/// Features:
/// - **Byte-bounded memory limit**: Enforces a strict upper ceiling (default: 64 MB).
/// - **Least Recently Used (LRU) eviction**: Drops old textures when memory limit is reached.
/// - **Background decoding & downscaling**: Reads, decodes, and scales images on worker threads.
pub struct ImageCache {
    entries: RwLock<HashMap<CacheKey, CacheEntry>>,
    access_counter: AtomicU64,
    current_bytes: AtomicUsize,
    max_bytes: AtomicUsize,
    pending: Arc<Mutex<HashMap<CacheKey, Vec<LoadCallback>>>>,
    tx: Mutex<Option<Sender<DecodeTask>>>,
}

static GLOBAL_CACHE: OnceLock<ImageCache> = OnceLock::new();

/// Default cache memory budget (64 Megabytes).
pub const DEFAULT_MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;

impl ImageCache {
    /// Returns the global shared `ImageCache` instance.
    pub fn global() -> &'static Self {
        GLOBAL_CACHE.get_or_init(Self::new)
    }

    /// Creates a new, isolated `ImageCache` instance with default memory budget (64 MB).
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            access_counter: AtomicU64::new(1),
            current_bytes: AtomicUsize::new(0),
            max_bytes: AtomicUsize::new(DEFAULT_MAX_CACHE_BYTES),
            pending: Arc::new(Mutex::new(HashMap::new())),
            tx: Mutex::new(None),
        }
    }

    /// Sets the maximum memory capacity in bytes for the image cache.
    pub fn set_max_bytes(&self, max_bytes: usize) {
        self.max_bytes.store(max_bytes, Ordering::Relaxed);
        self.evict_to_fit(0);
    }

    /// Returns the maximum memory capacity in bytes.
    pub fn max_bytes(&self) -> usize {
        self.max_bytes.load(Ordering::Relaxed)
    }

    /// Returns the current total memory consumption in bytes of cached pixel buffers.
    pub fn current_bytes(&self) -> usize {
        self.current_bytes.load(Ordering::Relaxed)
    }

    /// Looks up a cached `ImageData` by key (path and target bounds).
    pub fn get_keyed(&self, key: &CacheKey) -> Option<ImageData> {
        let mut entries = self.entries.write().ok()?;
        if let Some(entry) = entries.get_mut(key) {
            let access_id = self.access_counter.fetch_add(1, Ordering::Relaxed);
            entry.last_accessed = access_id;
            return Some(entry.data.clone());
        }

        // Fallback: If unscaled original is in cache, return it
        if key.max_dim.is_some() {
            let unscaled_key = CacheKey {
                path: key.path.clone(),
                max_dim: None,
            };
            if let Some(entry) = entries.get_mut(&unscaled_key) {
                let access_id = self.access_counter.fetch_add(1, Ordering::Relaxed);
                entry.last_accessed = access_id;
                return Some(entry.data.clone());
            }
        }

        None
    }

    /// Looks up a cached `ImageData` by file path.
    pub fn get<P: AsRef<Path>>(&self, path: P) -> Option<ImageData> {
        self.get_keyed(&CacheKey::from(path.as_ref()))
    }

    /// Inserts a decoded `ImageData` into the cache under the specified key, performing LRU eviction if needed.
    pub fn insert_keyed(&self, key: CacheKey, data: ImageData) {
        let byte_size = data.pixels.len() + std::mem::size_of::<CacheEntry>();
        self.evict_to_fit(byte_size);

        if let Ok(mut entries) = self.entries.write() {
            let access_id = self.access_counter.fetch_add(1, Ordering::Relaxed);
            if let Some(old) = entries.insert(
                key,
                CacheEntry {
                    data,
                    byte_size,
                    last_accessed: access_id,
                },
            ) {
                self.current_bytes
                    .fetch_sub(old.byte_size, Ordering::Relaxed);
            }
            self.current_bytes.fetch_add(byte_size, Ordering::Relaxed);
        }
    }

    /// Inserts a decoded `ImageData` into the cache under the specified path.
    pub fn insert<P: Into<PathBuf>>(&self, path: P, data: ImageData) {
        self.insert_keyed(CacheKey::from(path.into()), data);
    }

    /// Returns a cached `ImageData` if available, or synchronously loads and caches it from disk.
    pub fn get_or_load_keyed(&self, key: &CacheKey) -> Result<ImageData, String> {
        if let Some(cached) = self.get_keyed(key) {
            return Ok(cached);
        }

        let data = ImageData::from_file_uncached_scaled(&key.path, key.max_dim)?;
        self.insert_keyed(key.clone(), data.clone());
        Ok(data)
    }

    /// Returns a cached `ImageData` if available, or synchronously loads and caches it from disk.
    pub fn get_or_load<P: AsRef<Path>>(&self, path: P) -> Result<ImageData, String> {
        self.get_or_load_keyed(&CacheKey::from(path.as_ref()))
    }

    /// Checks if a key is currently being decoded asynchronously in the background.
    pub fn is_loading_keyed(&self, key: &CacheKey) -> bool {
        self.pending
            .lock()
            .map(|p| p.contains_key(key))
            .unwrap_or(false)
    }

    /// Checks if a path is currently being decoded asynchronously in the background.
    pub fn is_loading<P: AsRef<Path>>(&self, path: P) -> bool {
        self.is_loading_keyed(&CacheKey::from(path.as_ref()))
    }

    /// Requests background streaming and decoding for an image with optional dynamic downscaling.
    pub fn load_async_keyed<F>(&self, key: CacheKey, on_complete: Option<F>)
    where
        F: FnOnce(Result<ImageData, String>) + Send + 'static,
    {
        if let Some(cached) = self.get_keyed(&key) {
            if let Some(cb) = on_complete {
                cb(Ok(cached));
            }
            return;
        }

        let mut pending = self.pending.lock().unwrap();
        if let Some(callbacks) = pending.get_mut(&key) {
            if let Some(cb) = on_complete {
                callbacks.push(Box::new(cb));
            }
            return;
        }

        let mut callbacks: Vec<LoadCallback> = Vec::new();
        if let Some(cb) = on_complete {
            callbacks.push(Box::new(cb));
        }

        pending.insert(key.clone(), callbacks);
        drop(pending);

        self.ensure_worker_started();
        if let Ok(guard) = self.tx.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(DecodeTask { key });
            }
        }
    }

    /// Requests background streaming and decoding for an image file path without blocking the UI thread.
    pub fn load_async<P: AsRef<Path>, F>(&self, path: P, on_complete: Option<F>)
    where
        F: FnOnce(Result<ImageData, String>) + Send + 'static,
    {
        self.load_async_keyed(CacheKey::from(path.as_ref()), on_complete);
    }

    /// Clears all cached images from memory.
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
            self.current_bytes.store(0, Ordering::Relaxed);
        }
    }

    /// Evicts least recently used entries to fit incoming `needed_bytes` within `max_bytes`.
    fn evict_to_fit(&self, needed_bytes: usize) {
        let max = self.max_bytes.load(Ordering::Relaxed);
        let mut current = self.current_bytes.load(Ordering::Relaxed);

        if current + needed_bytes <= max {
            return;
        }

        if let Ok(mut entries) = self.entries.write() {
            while current + needed_bytes > max && !entries.is_empty() {
                // Find oldest entry (smallest last_accessed)
                let oldest_key = entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_accessed)
                    .map(|(k, _)| k.clone());

                if let Some(k) = oldest_key {
                    if let Some(removed) = entries.remove(&k) {
                        current = self
                            .current_bytes
                            .fetch_sub(removed.byte_size, Ordering::Relaxed)
                            - removed.byte_size;
                    }
                } else {
                    break;
                }
            }
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
                    let key = task.key;
                    let result = ImageData::from_file_uncached_scaled(&key.path, key.max_dim);

                    if let Ok(ref data) = result {
                        ImageCache::global().insert_keyed(key.clone(), data.clone());
                    }

                    let callbacks = {
                        let mut map = pending_map.lock().unwrap();
                        map.remove(&key).unwrap_or_default()
                    };

                    for cb in callbacks {
                        cb(result.clone());
                    }
                }
            })
            .expect("Failed to spawn mtk-image-loader background thread");
    }
}

/// Performs high-performance bilinear downsampling on an RGBA8 buffer to fit within `(max_w, max_h)`.
pub fn downscale_rgba8(
    src_w: u32,
    src_h: u32,
    src_pixels: &[u8],
    max_w: u32,
    max_h: u32,
) -> (u32, u32, Vec<u8>) {
    if src_w == 0 || src_h == 0 || max_w == 0 || max_h == 0 {
        return (src_w, src_h, src_pixels.to_vec());
    }

    if src_w <= max_w && src_h <= max_h {
        return (src_w, src_h, src_pixels.to_vec());
    }

    let scale = (max_w as f32 / src_w as f32)
        .min(max_h as f32 / src_h as f32)
        .min(1.0);
    let dst_w = (src_w as f32 * scale).round().max(1.0) as u32;
    let dst_h = (src_h as f32 * scale).round().max(1.0) as u32;

    if dst_w >= src_w && dst_h >= src_h {
        return (src_w, src_h, src_pixels.to_vec());
    }

    let mut dst = vec![0u8; (dst_w as usize) * (dst_h as usize) * 4];
    let x_ratio = src_w as f32 / dst_w as f32;
    let y_ratio = src_h as f32 / dst_h as f32;

    for dy in 0..dst_h {
        let sy = (dy as f32 + 0.5) * y_ratio - 0.5;
        let y0 = (sy.floor().max(0.0) as u32).min(src_h - 1);
        let y1 = (sy.ceil().max(0.0) as u32).min(src_h - 1);
        let wy = (sy - y0 as f32).clamp(0.0, 1.0);

        let row_offset = (dy as usize) * (dst_w as usize) * 4;

        for dx in 0..dst_w {
            let sx = (dx as f32 + 0.5) * x_ratio - 0.5;
            let x0 = (sx.floor().max(0.0) as u32).min(src_w - 1);
            let x1 = (sx.ceil().max(0.0) as u32).min(src_w - 1);
            let wx = (sx - x0 as f32).clamp(0.0, 1.0);

            let p00_idx = ((y0 as usize) * (src_w as usize) + (x0 as usize)) * 4;
            let p10_idx = ((y0 as usize) * (src_w as usize) + (x1 as usize)) * 4;
            let p01_idx = ((y1 as usize) * (src_w as usize) + (x0 as usize)) * 4;
            let p11_idx = ((y1 as usize) * (src_w as usize) + (x1 as usize)) * 4;

            let dst_idx = row_offset + (dx as usize) * 4;

            let w00 = (1.0 - wx) * (1.0 - wy);
            let w10 = wx * (1.0 - wy);
            let w01 = (1.0 - wx) * wy;
            let w11 = wx * wy;

            for c in 0..4 {
                let v = (src_pixels[p00_idx + c] as f32) * w00
                    + (src_pixels[p10_idx + c] as f32) * w10
                    + (src_pixels[p01_idx + c] as f32) * w01
                    + (src_pixels[p11_idx + c] as f32) * w11;
                dst[dst_idx + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    (dst_w, dst_h, dst)
}
