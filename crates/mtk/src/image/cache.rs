use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
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

struct WorkQueue {
    tasks: Mutex<VecDeque<DecodeTask>>,
    condvar: Condvar,
    shutdown: AtomicBool,
}

impl WorkQueue {
    fn new() -> Self {
        Self {
            tasks: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            shutdown: AtomicBool::new(false),
        }
    }

    fn pop(queue: &Arc<WorkQueue>) -> Option<DecodeTask> {
        let mut tasks = queue.tasks.lock().unwrap();
        loop {
            if queue.shutdown.load(Ordering::Relaxed) {
                return None;
            }
            if let Some(task) = tasks.pop_back() {
                return Some(task);
            }
            tasks = queue.condvar.wait(tasks).unwrap();
        }
    }
}

/// In-memory byte-bounded LRU texture cache and background streaming loader.
///
/// Features:
/// - **Byte-bounded memory limit**: Enforces a strict upper ceiling (default: 256 MB).
/// - **Least Recently Used (LRU) eviction**: Drops old textures when memory limit is reached.
/// - **Multi-threaded background worker pool**: Decodes and scales images in parallel.
/// - **LIFO priority queue**: Prioritizes images currently in the viewport.
/// - **Task cancellation**: Aborts pending decodes for unmounted/scrolled-out widgets.
pub struct ImageCache {
    entries: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,
    access_counter: Arc<AtomicU64>,
    current_bytes: Arc<AtomicUsize>,
    max_bytes: Arc<AtomicUsize>,
    pending: Arc<Mutex<HashMap<CacheKey, Vec<LoadCallback>>>>,
    queue: Arc<WorkQueue>,
    workers_started: OnceLock<()>,
}

impl Drop for ImageCache {
    fn drop(&mut self) {
        self.queue.shutdown.store(true, Ordering::Relaxed);
        self.queue.condvar.notify_all();
    }
}

static GLOBAL_CACHE: OnceLock<ImageCache> = OnceLock::new();

/// Default cache memory budget (256 Megabytes).
pub const DEFAULT_MAX_CACHE_BYTES: usize = 256 * 1024 * 1024;

impl ImageCache {
    /// Returns the global shared `ImageCache` instance.
    pub fn global() -> &'static Self {
        GLOBAL_CACHE.get_or_init(Self::new)
    }

    /// Creates a new, isolated `ImageCache` instance with default memory budget (256 MB).
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_counter: Arc::new(AtomicU64::new(1)),
            current_bytes: Arc::new(AtomicUsize::new(0)),
            max_bytes: Arc::new(AtomicUsize::new(DEFAULT_MAX_CACHE_BYTES)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            queue: Arc::new(WorkQueue::new()),
            workers_started: OnceLock::new(),
        }
    }

    /// Sets the maximum memory capacity in bytes for the image cache.
    pub fn set_max_bytes(&self, max_bytes: usize) {
        self.max_bytes.store(max_bytes, Ordering::Relaxed);
        evict_to_fit_internal(&self.entries, &self.current_bytes, &self.max_bytes, 0);
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

        // Fallback 1: If unscaled original is in cache, return it
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

        // Fallback 2: If any version of this image path is in cache, return the best match
        if let Some(best_key) = entries
            .keys()
            .filter(|k| k.path == key.path)
            .cloned()
            .max_by_key(|k| k.max_dim.map(|(w, h)| w * h).unwrap_or(u32::MAX))
        {
            if let Some(entry) = entries.get_mut(&best_key) {
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
        insert_entry_internal(
            &self.entries,
            &self.access_counter,
            &self.current_bytes,
            &self.max_bytes,
            key,
            data,
        );
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

    /// Checks if ANY background decoding task is currently pending or running for this image path.
    pub fn is_loading_path<P: AsRef<Path>>(&self, path: P) -> bool {
        let p = path.as_ref();
        self.pending
            .lock()
            .map(|pending| pending.keys().any(|k| k.path == p))
            .unwrap_or(false)
    }

    /// Checks if a path is currently being decoded asynchronously in the background.
    pub fn is_loading<P: AsRef<Path>>(&self, path: P) -> bool {
        self.is_loading_path(path)
    }

    /// Cancels any pending background decoding request for the specified cache key.
    pub fn cancel_load_keyed(&self, key: &CacheKey) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(key);
        }
        if let Ok(mut tasks) = self.queue.tasks.lock() {
            tasks.retain(|t| &t.key != key);
        }
    }

    /// Cancels any pending background decoding requests for the specified file path.
    pub fn cancel_load_path<P: AsRef<Path>>(&self, path: P) {
        let p = path.as_ref();
        if let Ok(mut pending) = self.pending.lock() {
            pending.retain(|k, _| k.path != p);
        }
        if let Ok(mut tasks) = self.queue.tasks.lock() {
            tasks.retain(|t| t.key.path != p);
        }
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

        self.ensure_workers_started();
        {
            let mut tasks = self.queue.tasks.lock().unwrap();
            if !tasks.iter().any(|t| t.key == key) {
                tasks.push_back(DecodeTask { key });
                self.queue.condvar.notify_one();
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

    fn ensure_workers_started(&self) {
        self.workers_started.get_or_init(|| {
            let num_workers = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
                .clamp(2, 16);

            for i in 0..num_workers {
                let entries = Arc::clone(&self.entries);
                let access_counter = Arc::clone(&self.access_counter);
                let current_bytes = Arc::clone(&self.current_bytes);
                let max_bytes = Arc::clone(&self.max_bytes);
                let pending = Arc::clone(&self.pending);
                let queue = Arc::clone(&self.queue);

                let thread_name = format!("mtk-image-loader-{}", i);
                let _ = thread::Builder::new().name(thread_name).spawn(move || {
                    worker_loop(
                        entries,
                        access_counter,
                        current_bytes,
                        max_bytes,
                        pending,
                        queue,
                    );
                });
            }
        });
    }
}

fn evict_to_fit_internal(
    entries: &RwLock<HashMap<CacheKey, CacheEntry>>,
    current_bytes: &AtomicUsize,
    max_bytes: &AtomicUsize,
    needed_bytes: usize,
) {
    let max = max_bytes.load(Ordering::Relaxed);
    let mut current = current_bytes.load(Ordering::Relaxed);

    if current + needed_bytes <= max {
        return;
    }

    if let Ok(mut map) = entries.write() {
        while current + needed_bytes > max && !map.is_empty() {
            let oldest_key = map
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(k, _)| k.clone());

            if let Some(k) = oldest_key {
                if let Some(removed) = map.remove(&k) {
                    current = current_bytes.fetch_sub(removed.byte_size, Ordering::Relaxed)
                        - removed.byte_size;
                }
            } else {
                break;
            }
        }
    }
}

fn insert_entry_internal(
    entries: &RwLock<HashMap<CacheKey, CacheEntry>>,
    access_counter: &AtomicU64,
    current_bytes: &AtomicUsize,
    max_bytes: &AtomicUsize,
    key: CacheKey,
    data: ImageData,
) {
    let byte_size = data.pixels.len() + std::mem::size_of::<CacheEntry>();
    evict_to_fit_internal(entries, current_bytes, max_bytes, byte_size);

    if let Ok(mut map) = entries.write() {
        let access_id = access_counter.fetch_add(1, Ordering::Relaxed);
        if let Some(old) = map.insert(
            key,
            CacheEntry {
                data,
                byte_size,
                last_accessed: access_id,
            },
        ) {
            current_bytes.fetch_sub(old.byte_size, Ordering::Relaxed);
        }
        current_bytes.fetch_add(byte_size, Ordering::Relaxed);
    }
}

fn worker_loop(
    entries: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,
    access_counter: Arc<AtomicU64>,
    current_bytes: Arc<AtomicUsize>,
    max_bytes: Arc<AtomicUsize>,
    pending: Arc<Mutex<HashMap<CacheKey, Vec<LoadCallback>>>>,
    queue: Arc<WorkQueue>,
) {
    loop {
        let task = match WorkQueue::pop(&queue) {
            Some(t) => t,
            None => return,
        };

        // Check if task is still wanted (not cancelled)
        {
            let pending_guard = pending.lock().unwrap();
            if !pending_guard.contains_key(&task.key) {
                continue;
            }
        }

        let key = task.key;

        // Check if an existing version in cache can satisfy this request in-memory
        let cached_pixels = {
            let entries_guard = entries.read().unwrap();
            if let Some(entry) = entries_guard.get(&key) {
                Some(entry.data.clone())
            } else if let Some(best_key) = entries_guard
                .keys()
                .filter(|k| k.path == key.path)
                .cloned()
                .max_by_key(|k| k.max_dim.map(|(w, h)| w * h).unwrap_or(u32::MAX))
            {
                entries_guard.get(&best_key).map(|e| e.data.clone())
            } else {
                None
            }
        };

        let result = if let Some(cached) = cached_pixels {
            if let Some((max_w, max_h)) = key.max_dim {
                if cached.width > max_w || cached.height > max_h {
                    let (w, h, pixels) =
                        downscale_rgba8(cached.width, cached.height, &cached.pixels, max_w, max_h);
                    ImageData::from_rgba8(w, h, pixels)
                } else {
                    Ok(cached)
                }
            } else {
                Ok(cached)
            }
        } else {
            ImageData::from_file_uncached_scaled(&key.path, key.max_dim)
        };

        if let Ok(ref data) = result {
            insert_entry_internal(
                &entries,
                &access_counter,
                &current_bytes,
                &max_bytes,
                key.clone(),
                data.clone(),
            );
        }

        let callbacks = {
            let mut map = pending.lock().unwrap();
            map.remove(&key).unwrap_or_default()
        };

        for cb in callbacks {
            cb(result.clone());
        }
    }
}

#[derive(Clone)]
struct Filter1D {
    start: usize,
    weights: Vec<f32>,
}

fn compute_filter_1d(src_size: u32, dst_size: u32) -> Vec<Filter1D> {
    let scale = src_size as f32 / dst_size as f32;
    let mut filters = Vec::with_capacity(dst_size as usize);

    for d in 0..dst_size {
        let src_start = (d as f32) * scale;
        let src_end = ((d + 1) as f32) * scale;

        let s_min = src_start.floor() as u32;
        let s_max = (src_end.ceil() as u32).min(src_size);

        let mut weights = Vec::with_capacity((s_max - s_min) as usize);
        let mut total_w = 0.0f32;

        for s in s_min..s_max {
            let left = (s as f32).max(src_start);
            let right = ((s + 1) as f32).min(src_end);
            let w = (right - left).max(0.0);
            weights.push(w);
            total_w += w;
        }

        if total_w > 0.0 {
            let inv = 1.0 / total_w;
            for w in &mut weights {
                *w *= inv;
            }
        }

        filters.push(Filter1D {
            start: s_min as usize,
            weights,
        });
    }

    filters
}

/// Performs high-performance separable anti-aliased area-averaging downsampling on an RGBA8 buffer to fit within `(max_w, max_h)`.
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

    let filters_x = compute_filter_1d(src_w, dst_w);
    let filters_y = compute_filter_1d(src_h, dst_h);

    // Pass 1: Horizontal downscaling (src_w x src_h -> dst_w x src_h)
    let intermediate: Vec<f32> = if dst_w < src_w {
        let mut buf = vec![0.0f32; (dst_w as usize) * (src_h as usize) * 4];
        for y in 0..src_h as usize {
            let src_row = &src_pixels[y * (src_w as usize) * 4..(y + 1) * (src_w as usize) * 4];
            let dst_row = &mut buf[y * (dst_w as usize) * 4..(y + 1) * (dst_w as usize) * 4];

            for (dx, filter) in filters_x.iter().enumerate() {
                let mut r = 0.0f32;
                let mut g = 0.0f32;
                let mut b = 0.0f32;
                let mut a = 0.0f32;

                for (i, &w) in filter.weights.iter().enumerate() {
                    let p = (filter.start + i) * 4;
                    r += (src_row[p] as f32) * w;
                    g += (src_row[p + 1] as f32) * w;
                    b += (src_row[p + 2] as f32) * w;
                    a += (src_row[p + 3] as f32) * w;
                }

                let d = dx * 4;
                dst_row[d] = r;
                dst_row[d + 1] = g;
                dst_row[d + 2] = b;
                dst_row[d + 3] = a;
            }
        }
        buf
    } else {
        src_pixels.iter().map(|&b| b as f32).collect()
    };

    // Pass 2: Vertical downscaling (dst_w x src_h -> dst_w x dst_h)
    let mut dst = vec![0u8; (dst_w as usize) * (dst_h as usize) * 4];
    for (dy, filter) in filters_y.iter().enumerate() {
        let dst_row = &mut dst[dy * (dst_w as usize) * 4..(dy + 1) * (dst_w as usize) * 4];

        for dx in 0..dst_w as usize {
            let mut r = 0.0f32;
            let mut g = 0.0f32;
            let mut b = 0.0f32;
            let mut a = 0.0f32;

            for (i, &w) in filter.weights.iter().enumerate() {
                let sy = filter.start + i;
                let p = (sy * (dst_w as usize) + dx) * 4;
                r += intermediate[p] * w;
                g += intermediate[p + 1] * w;
                b += intermediate[p + 2] * w;
                a += intermediate[p + 3] * w;
            }

            let d = dx * 4;
            dst_row[d] = r.round().clamp(0.0, 255.0) as u8;
            dst_row[d + 1] = g.round().clamp(0.0, 255.0) as u8;
            dst_row[d + 2] = b.round().clamp(0.0, 255.0) as u8;
            dst_row[d + 3] = a.round().clamp(0.0, 255.0) as u8;
        }
    }

    (dst_w, dst_h, dst)
}
