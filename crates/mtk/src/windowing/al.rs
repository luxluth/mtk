use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window as WWindow, WindowId},
};

use crate::{
    Context, TextStyle,
    ui::{Event, View, event::EventResult},
    windowing::renderer::Renderer,
};

pub struct Window<'r, S, V>
where
    V: View<S>,
{
    renderer: Option<Renderer<'r>>,

    window: Option<Arc<WWindow>>,
    context: Context,
    state: S,

    app_view_fn: Option<Box<dyn FnMut(&S) -> V>>,
    update_fn: Option<Box<dyn FnMut(&mut S, V::Message)>>,

    view: Option<V>,
    element: Option<V::Element>,
    attr: WindowAttributes,
    cursor_pos: (f32, f32),
    last_frame_time: Instant,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowDimension {
    pub width: u32,
    pub height: u32,
}

impl WindowDimension {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }
}

impl From<(u32, u32)> for WindowDimension {
    fn from((width, height): (u32, u32)) -> Self {
        WindowDimension::new(width, height)
    }
}

impl Into<winit::dpi::Size> for WindowDimension {
    fn into(self) -> winit::dpi::Size {
        winit::dpi::Size::Physical(PhysicalSize {
            width: self.width,
            height: self.height,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WindowAttributes {
    pub resizable: bool,
    pub transparent: bool,
    pub blur: bool,
    pub decorations: bool,
    pub size: WindowDimension,
    pub min_size: Option<WindowDimension>,
    pub max_size: Option<WindowDimension>,
    pub title: String,
    #[cfg(target_os = "linux")]
    pub app_id: String,
}

macro_rules! attr_fn {
    ($name:ident, $field:ident, $t:ty) => {
        pub fn $name(mut self: Self, value: $t) -> Self {
            self.$field = value;
            self
        }
    };
}

macro_rules! attr_fn_string {
    ($name:ident, $field:ident) => {
        pub fn $name<S: ToString>(mut self: Self, value: S) -> Self {
            self.$field = value.to_string();
            self
        }
    };
}

impl WindowAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    attr_fn_string!(with_title, title);
    attr_fn!(with_resizable, resizable, bool);
    attr_fn!(with_transparency, transparent, bool);
    attr_fn!(with_blur, blur, bool);
    attr_fn!(with_decorations, decorations, bool);
    attr_fn!(with_size, size, WindowDimension);
    attr_fn!(with_min_size, min_size, Option<WindowDimension>);
    attr_fn!(with_max_size, max_size, Option<WindowDimension>);

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    attr_fn_string!(with_app_id, app_id);
}

impl Default for WindowAttributes {
    fn default() -> Self {
        Self {
            resizable: true,
            title: "MTK".to_string(),
            size: WindowDimension::new(800, 600),
            min_size: None,
            max_size: None,

            transparent: true,
            blur: false,
            decorations: false,

            #[cfg(any(
                target_os = "linux",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            app_id: "".to_string(),
        }
    }
}

impl<'r, S, V> Window<'r, S, V>
where
    V: View<S>,
{
    pub fn with<U, F>(state: S, update_fn: U, mut view_fn: F) -> Self
    where
        U: FnMut(&mut S, V::Message) + 'static,
        F: FnMut(&S) -> V + 'static,
    {
        let mut ctx = Context::new();

        let view = view_fn(&state);
        let element = view.build(&mut ctx);

        let root_node = view.get_node(&element);
        ctx.root_attach(root_node);

        ctx.set_text_sizing_func(move |ctx, _node, text, userdata, avail_w, avail_h| {
            let default_style = TextStyle::default();
            let style = userdata
                .and_then(|u| u.downcast_ref::<TextStyle>())
                .unwrap_or(&default_style);

            let text_ctx = ctx.text_context.clone();
            crate::text::measure_text(text, style, avail_w, avail_h, &text_ctx)
        });

        Self {
            renderer: None,
            window: None,
            context: ctx,
            state,
            app_view_fn: Some(Box::new(view_fn)),
            update_fn: Some(Box::new(update_fn)),
            view: Some(view),
            attr: WindowAttributes::default(),
            element: Some(element),
            cursor_pos: (0.0, 0.0),
            last_frame_time: Instant::now(),
        }
    }

    pub fn present(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(self).unwrap();
    }

    pub fn present_with(&mut self, attr: WindowAttributes) {
        self.attr = attr;
        self.present();
    }

    fn dispatch_and_rebuild(&mut self, mtk_event: Event) {
        if let (Some(view), Some(element), Some(app_view_fn), Some(update_fn)) = (
            &self.view,
            &mut self.element,
            &mut self.app_view_fn,
            &mut self.update_fn,
        ) {
            let is_tick = matches!(mtk_event, Event::Tick { .. });

            // Pass 1 - READONLY state down
            let (result, optional_msg) =
                view.handle_event(element, &self.state, mtk_event, &mut self.context);

            let mut state_changed = false;

            // Pass 2 - we check if a logical message bubbled up to the root
            if let Some(msg) = optional_msg {
                update_fn(&mut self.state, msg);
                state_changed = true;
            }

            // Pass 3 - we rebuild only when state has been updated
            if state_changed {
                let new_view = app_view_fn(&self.state);
                new_view.rebuild(view, &mut self.context, element);
                self.view = Some(new_view);
            }

            if let Some(window) = &self.window {
                if state_changed || (!is_tick && result == EventResult::Handled) {
                    window.request_redraw();
                }
            }
        }
    }
}

impl<'r, S, V> ApplicationHandler for Window<'r, S, V>
where
    V: View<S>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attr = self.attr.clone();
        let mut window_attributes = WWindow::default_attributes()
            .with_title(attr.title)
            .with_decorations(attr.decorations)
            .with_transparent(attr.transparent)
            .with_blur(attr.blur)
            .with_resizable(attr.resizable)
            .with_inner_size(attr.size);

        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly"
        ))]
        {
            use winit::platform::wayland::WindowAttributesExtWayland;
            window_attributes = window_attributes.with_name(attr.app_id.clone(), "");
        }

        if let Some(min_size) = attr.min_size {
            window_attributes = window_attributes.with_min_inner_size(min_size);
        }

        if let Some(max_size) = attr.max_size {
            window_attributes = window_attributes.with_max_inner_size(max_size);
        }

        self.context
            .compute_layout(attr.size.height as f32, attr.size.width as f32);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_ime_allowed(true);

        self.window = Some(window.clone());
        self.context.window = Some(window.clone());

        let renderer = pollster::block_on(Renderer::new(
            event_loop.owned_display_handle(),
            window.clone(),
        ));
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let window = self.window.as_ref().unwrap().clone();
        if id != window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                // NOTE: Maybe accept a before_close_hook
                event_loop.exit();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.context.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic,
            } => {
                let mtk_event = Event::KeyboardInput {
                    event,
                    is_synthetic,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::Ime(ime) => {
                let mtk_event = Event::Ime(ime);
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                }

                if let (Some(view), Some(element)) = (&self.view, &self.element) {
                    let root = view.get_node(element);
                    root.set_dirty(&mut self.context);
                }

                self.dispatch_and_rebuild(Event::WindowResized(WindowDimension {
                    width: size.width,
                    height: size.height,
                }));
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32;
                let y = position.y as f32;
                self.cursor_pos = (x, y);
                let hit_nodes = self.context.pick(x, y);
                let mtk_event = Event::CursorMoved { x, y, hit_nodes };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::MouseInput { state, .. } => {
                let pressed = state == winit::event::ElementState::Pressed;
                let hit_nodes = self.context.pick(self.cursor_pos.0, self.cursor_pos.1);
                let mtk_event = Event::MouseInput {
                    pressed,
                    x: self.cursor_pos.0,
                    y: self.cursor_pos.1,
                    hit_nodes,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                let (dx, dy, is_touchpad) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * 20.0, y * 20.0, false),
                    MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32, true),
                };
                let hit_nodes = self.context.pick(self.cursor_pos.0, self.cursor_pos.1);
                let mtk_event = Event::MouseWheel {
                    delta_x: dx,
                    delta_y: dy,
                    is_touchpad,
                    phase,
                    hit_nodes,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame_time).as_secs_f32();
                self.last_frame_time = now;
                self.dispatch_and_rebuild(Event::Tick { dt });

                let size = window.inner_size();
                self.context
                    .compute_layout(size.width as f32, size.height as f32);

                let viewport = crate::style::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: size.width as f32,
                    h: size.height as f32,
                };
                self.context.build_render_list(viewport);

                if let Some(renderer) = &mut self.renderer {
                    let focused_caret = renderer.render(&self.context);
                    if let Some(window) = &self.window {
                        if let Some(caret) = focused_caret {
                            let position = PhysicalPosition::new(caret[0] as u32, caret[1] as u32);
                            let size = PhysicalSize::new(caret[2] as u32, caret[3] as u32);
                            window.set_ime_cursor_area(position, size);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
