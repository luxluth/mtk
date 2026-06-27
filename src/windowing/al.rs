use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window as WWindow, WindowId},
};

use crate::{Context, ui::View};

pub struct Window<S, V>
where
    V: View<S>,
{
    window: Option<Arc<WWindow>>,
    context: Context,
    state: S,
    app_view_fn: Option<Box<dyn FnMut(&mut S) -> V>>,
    view: Option<V>,
    element: Option<V::Element>,
    attr: WindowAttr,
}

#[derive(Clone)]
pub struct WindowAttr {
    pub resizable: bool,
    pub transparent: bool,
    pub blur: bool,
    pub decorations: bool,
    size: (u32, u32),
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

impl WindowAttr {
    pub fn new() -> Self {
        Self::default()
    }

    attr_fn!(with_title, title, String);
    attr_fn!(with_resizable, resizable, bool);
    attr_fn!(with_transparency, transparent, bool);
    attr_fn!(with_blur, blur, bool);
    attr_fn!(with_decorations, decorations, bool);
    attr_fn!(with_size, size, (u32, u32));

    #[cfg(target_os = "linux")]
    attr_fn!(with_app_id, app_id, String);
}

impl Default for WindowAttr {
    fn default() -> Self {
        Self {
            resizable: true,
            title: "MTK".to_string(),
            size: (800, 600),

            transparent: true,
            blur: false,
            decorations: false,

            #[cfg(target_os = "linux")]
            app_id: "".to_string(),
        }
    }
}

impl<S, V> Window<S, V>
where
    V: View<S>,
{
    pub fn with<F>(mut state: S, mut view_fn: F) -> Self
    where
        F: FnMut(&mut S) -> V + 'static,
    {
        let mut ctx = Context::new();

        let view = view_fn(&mut state);
        let element = view.build(&mut ctx);

        let root_node = view.get_node(&element);
        ctx.root_attach(root_node);

        let text_ctx = ctx.text_context.clone();
        ctx.set_text_sizing_func(move |_node, text, userdata, avail_w, avail_h| {
            use crate::ui::style::TextStyle;

            let default_style = TextStyle::default();
            let style = userdata
                .and_then(|u| u.downcast_ref::<TextStyle>())
                .unwrap_or(&default_style);

            crate::text::measure_text(text, style, avail_w, avail_h, &text_ctx)
        });

        Self {
            window: None,
            context: ctx,
            state,
            app_view_fn: Some(Box::new(view_fn)),
            view: Some(view),
            attr: WindowAttr::default(),
            element: Some(element),
        }
    }

    pub fn present(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(self).unwrap();
    }

    pub fn present_with(&mut self, attr: WindowAttr) {
        self.attr = attr;
        self.present();
    }

    fn dispatch_and_rebuild(&mut self, mtk_event: crate::ui::Event) {
        if let (Some(view), Some(element), Some(app_view_fn)) =
            (&self.view, &mut self.element, &mut self.app_view_fn)
        {
            view.message(element, &mut self.state, mtk_event);
            let new_view = app_view_fn(&mut self.state);

            new_view.rebuild(view, &mut self.context, element);

            self.view = Some(new_view);

            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

impl<S, V> ApplicationHandler for Window<S, V>
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
            .with_inner_size(PhysicalSize::new(attr.size.0, attr.size.1));

        #[cfg(target_os = "linux")]
        {
            use winit::platform::wayland::WindowAttributesExtWayland;
            window_attributes = window_attributes.with_name(attr.app_id.clone(), "");
        }

        self.context
            .compute_layout(attr.size.0 as f32, attr.size.1 as f32);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let window = self.window.as_ref().unwrap();
        if id != window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                // NOTE: Maybe accept a before_close_hook
                event_loop.exit();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mtk_event = crate::ui::Event::CursorMoved {
                    x: position.x as f32,
                    y: position.y as f32,
                };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::MouseInput { state, .. } => {
                let pressed = state == winit::event::ElementState::Pressed;
                let mtk_event = crate::ui::Event::MouseInput { pressed };
                self.dispatch_and_rebuild(mtk_event);
            }
            WindowEvent::RedrawRequested => {
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
            }
            _ => {}
        }
    }
}
