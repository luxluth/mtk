use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use winit::window::Window;

/// A request dispatched to background worker threads to rasterize an SVG.
pub struct SvgRasterRequest {
    pub node: crate::Node,
    pub svg_data: crate::image::SvgData,
    pub target_w: u32,
    pub target_h: u32,
    pub fit: crate::image::ObjectFit,
    pub generation: u64,
}

/// A completed SVG rasterization result ready for GPU texture upload.
pub struct SvgRasterResponse {
    pub node: crate::Node,
    pub svg_id: u64,
    pub target_w: u32,
    pub target_h: u32,
    pub fit: crate::image::ObjectFit,
    pub generation: u64,
    pub pixmap: Option<resvg::tiny_skia::Pixmap>,
}

struct SvgPoolShared {
    queue: HashMap<crate::Node, SvgRasterRequest>,
    shutdown: bool,
    response_tx: Sender<SvgRasterResponse>,
    window: Arc<Window>,
}

/// Background thread pool for non-blocking asynchronous SVG rasterization.
pub struct SvgRasterPool {
    shared: Arc<(Mutex<SvgPoolShared>, Condvar)>,
    response_rx: Receiver<SvgRasterResponse>,
    threads: Vec<thread::JoinHandle<()>>,
}

fn worker_loop(shared: Arc<(Mutex<SvgPoolShared>, Condvar)>) {
    loop {
        let (request, response_tx, window) = {
            let (lock, cvar) = &*shared;
            let mut guard = lock.lock().unwrap();
            while guard.queue.is_empty() && !guard.shutdown {
                guard = cvar.wait(guard).unwrap();
            }
            if guard.shutdown && guard.queue.is_empty() {
                break;
            }
            let key = match guard.queue.keys().next().copied() {
                Some(k) => k,
                None => continue,
            };
            let req = guard.queue.remove(&key).unwrap();
            let response_tx = guard.response_tx.clone();
            let window = Arc::clone(&guard.window);
            (req, response_tx, window)
        };

        let pixmap =
            request
                .svg_data
                .render_to_pixmap(request.target_w, request.target_h, request.fit);

        let response = SvgRasterResponse {
            node: request.node,
            svg_id: request.svg_data.id,
            target_w: request.target_w,
            target_h: request.target_h,
            fit: request.fit,
            generation: request.generation,
            pixmap,
        };

        if response_tx.send(response).is_ok() {
            window.request_redraw();
        }
    }
}

impl SvgRasterPool {
    /// Creates a new background worker pool bounded to CPU parallelism.
    pub fn new(window: Arc<Window>) -> Self {
        let (response_tx, response_rx) = channel();
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            .min(4)
            .max(1);

        let shared = Arc::new((
            Mutex::new(SvgPoolShared {
                queue: HashMap::new(),
                shutdown: false,
                response_tx,
                window,
            }),
            Condvar::new(),
        ));

        let mut threads = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            let shared_clone = Arc::clone(&shared);
            let handle = thread::Builder::new()
                .name(format!("mtk-svg-raster-{i}"))
                .spawn(move || worker_loop(shared_clone))
                .expect("Failed to spawn SVG raster worker thread");
            threads.push(handle);
        }

        Self {
            shared,
            response_rx,
            threads,
        }
    }

    /// Schedules an SVG rasterization request, overwriting any superseded pending request for this node.
    pub fn schedule(&self, request: SvgRasterRequest) {
        let (lock, cvar) = &*self.shared;
        let mut guard = lock.lock().unwrap();
        guard.queue.insert(request.node, request);
        cvar.notify_one();
    }

    /// Tries to receive completed rasterization responses without blocking.
    pub fn try_recv(&self) -> Option<SvgRasterResponse> {
        self.response_rx.try_recv().ok()
    }
}

impl Drop for SvgRasterPool {
    fn drop(&mut self) {
        {
            let (lock, cvar) = &*self.shared;
            let mut guard = lock.lock().unwrap();
            guard.shutdown = true;
            guard.queue.clear();
            cvar.notify_all();
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}
