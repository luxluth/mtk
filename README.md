<div align="center">
  <img src="https://raw.github.com/luxluth/mtk/master/assets/mtk-logo.png" alt="MTK Logo" width="200" />

# _Muse Toolkit_

</div>

MTK is a declarative, retained-mode GUI toolkit for Rust. It pairs an Elm-inspired functional interface with a C-based Flexbox layout engine, native WGPU hardware rendering, and Parley typography.

> [!WARNING]
> _Still experimental and under active development. Not recommended for production use._

---

## Philosophy

- **Declarative Views, Retained Elements**: Write functional, declarative views (`view(&state) -> impl View<State>`). MTK retains layout nodes and updates only changed properties via `rebuild()`.
- **Primitives Over Monoliths**: Focus on flexible, unopinionated primitives rather than a rigid widget set, making it easy to build custom workstation tools, editors, and domain components.
- **Accessible Core**: The coordinator (`Context`), layout engine (`muse.h`), typography metrics, and GPU canvas passes are public so developers can build on top of MTK without fighting the framework.
- **Source Introspection**: Every widget tracks its call site with `#[track_caller]` for live in-tree debugging and inspection.

---

## Core Technologies

MTK brings together proven technologies from across the systems and graphics ecosystem:

- **Layout**: [`muse.h`](crates/mtk/src/c/muse.h) — C-based Flexbox engine with incremental layout passes, aspect-ratio resolution, and scroll clamping.
- **Graphics**: [WGPU](https://wgpu.rs/) — Cross-platform GPU rendering targeting Vulkan, Metal, and DirectX 12.
- **Typography**: [Parley](https://github.com/linebender/parley), [Swash](https://github.com/dfrg/swash) — Multi-font styling, dynamic font fallback, OpenType ligatures, and inline span geometry.
- **Windowing**: [winit](https://github.com/rust-windowing/winit) — Native window creation, DPI scaling, and event handling.
- **Images**: [image](https://github.com/image-rs/image), [resvg](https://github.com/RazrFalcon/resvg) — Asynchronous streaming loader (`mtk-image-loader`) with byte-bounded LRU caching.
- **Inspector**: [Ratatui](https://github.com/ratatui/ratatui) — Built-in terminal UI layout inspector.
- **Clipboard**: [arboard](https://github.com/1Password/arboard) — Cross-platform clipboard read and write support.

---

## Quick Start Example

Below is a complete, minimal example that creates a window displaying text:

```rust,no_run
use mtk::clr;
use mtk::style::{Size, Style};
use mtk::ui::ViewStyleExt;
use mtk::ui::widgets::text;
use mtk::windowing::{Window, WindowAttributes};

fn main() {
    let mut window = Window::with(
        (),
        |_state, _msg: ()| {},
        |_state| {
            text("Hello, MTK!").style(
                Style::new()
                    .bg_color(clr!(white))
                    .padding(10.)
                    .width(Size::Percent(1.))
                    .height(Size::Percent(1.)),
            )
        },
    );

    let attrs = WindowAttributes::new()
        .with_title("Hello MTK")
        .with_size((400, 200).into());

    window.present_with(attrs);
}
```

---

## State Management and Lenses

MTK uses unidirectional data flow with typed messages. To decompose large application states into reusable sub-views without cloning, MTK provides **Lenses** and the **Adapter Pattern**:

```rust,no_run
use mtk::Lens;
use mtk::ui::widgets::*;
use mtk::ui::{adapt, ViewAdaptExt, View};

#[derive(Lens)]
struct AppState {
    username: String,
    bio: String,
}

enum AppMsg {
    SetUsername(String),
    SetBio(String),
}

fn view(state: &AppState) -> impl View<AppState, Message = AppMsg> {
    column((
        text("Profile Editor"),
        // Adapt via the standalone helper and derived lens:
        adapt(input_text(), AppState::username, AppMsg::SetUsername),
        // Or adapt via method chaining extension trait:
        text_area().adapt(AppState::bio, AppMsg::SetBio),
    ))
}
```

---

## Examples

To run the included widget gallery:

```bash
cargo run --example widget_gallery
```

---

## Acknowledgments

MTK is inspired by [Xilem](https://github.com/linebender/xilem) and borrows architectural concepts for reactive, declarative GUI representation and state adaptation in Rust.
