# MTK — Muse UI Toolkit

MTK is a responsive, reactive GUI framework built for Rust, inspired by [Xilem](https://github.com/linebender/xilem). It is powered by a high-performance C layout engine (`muse.h`), native WGPU hardware rendering, fluid layout flexing, and Parley typography.

![Todo App Example](https://raw.github.com/luxluth/mtk/master/assets/todoapp.png)

> [!WARNING]
> _Still experimental. Not for production use_

---

## Quick Start Example

Below is a simple, complete example that launches a window displaying text:

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

## Key Capabilities

- **Declarative View Hierarchy**: Compose user interfaces cleanly using reactive view trees (`View`).
- **Lenses and State Adaptation**: Decouple stateful sub-widgets from root application state using zero-cost `Lens` accessors.
- **Unidirectional Data Flow**: State updates pass through pure `update` functions driven by typed messages.
- **Rich UI Widgets**: Includes `container`, `column`, `row`, `text`, `scroll_view`, `input_text`, and `text_area`.
- **Seamless Intrinsic Scrolling**: Automatic content bounds calculation, smooth kinetic scrolling, and overlay shadow scrollbars.
- **Persistent System Clipboard**: Cross-platform system clipboard integration (`ClipboardData`, `ctx.clipboard_copy`, `ctx.clipboard_get`).

---

## State Management, Lenses and The Adapter Pattern

MTK uses **Lenses** and the **Adapter Pattern** to focus large global application states into isolated, reusable sub-views while keeping state flow 100% type-safe.

### 1. What is a `Lens`?

A `Lens<Outer, Inner>` provides a functional view abstraction for focusing into a sub-field of a parent state without mutating or cloning the parent structure.

- Derive lenses automatically using `#[derive(Lens)]` on your application state struct:
  ```rust,ignore
  #[derive(Lens)]
  struct AppState {
      search_query: String,
      bio: String,
  }
  ```
- Closures matching `fn(&Outer) -> &Inner` also automatically implement `Lens<Outer, Inner>`.

### 2. Adapting Views with `adapt` and `.adapt()`

Leaf components like `input_text()` or `text_area()` expect a specific slice of state (e.g. `String`) and emit specific messages. The `adapt()` helper or `.adapt()` extension method maps:

1. **State Down**: Projects the parent state (`Outer`) down to the sub-view (`Inner`) using the lens.
2. **Messages Up**: Maps local child messages into parent application domain messages (`OuterMsg`).

```rust,no_run
use mtk::Lens;
use mtk::ui::widgets::*;
use mtk::ui::{adapt, ViewAdaptExt};

#[derive(Lens)]
struct AppState {
    username: String,
    bio: String,
}

enum AppMsg {
    SetUsername(String),
    SetBio(String),
}

fn view(state: &AppState) -> impl mtk::ui::View<AppState, Message = AppMsg> {
    column((
        text("Profile Editor"),
        // Adapt via standalone helper and derived lens:
        adapt(input_text(), AppState::username, AppMsg::SetUsername),
        // Or adapt via method chaining extension trait:
        text_area().adapt(AppState::bio, AppMsg::SetBio),
    ))
}
```

---

## Core Architecture and Execution Model

The lifetime of a frame inside MTK follows a structured multi-pass execution model managed by `Context`:

1. **Tree Construction**: Widgets create and attach layout primitives (`Node`) to the context.
2. **Layout Pass (`compute_layout`)**: A multi-pass algorithm calculates intrinsic text sizes, flex dimensions, percentages, and absolute bounds across the tree.
3. **Render List Generation (`build_render_list`)**: The layout tree is flattened into a Z-indexed array of draw commands (`RenderCommand`), applying scissor clipping rectangles for scroll views and containers.
4. **Event Dispatch and Picking (`pick`)**: Coordinate hit-testing determines mouse target nodes and routes keyboard/focus events.

---

## Examples

To run the included MTK demos:

```bash
cargo run --example scroll_demo
```

---

## Acknowledgments

MTK is inspired by [Xilem](https://github.com/linebender/xilem) and borrows architectural concepts for reactive, declarative GUI representation and state adaptation in Rust.
