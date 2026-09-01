use crate::debugger::SourceLocation;
use crate::rgba;
use crate::ui::layer::{Layer, layer};
use crate::ui::transition::Transition;

/// Creates a new `Modal` overlay dialog surface wrapping a main view and a dialog view.
///
/// # Examples
/// ```rust,ignore
/// modal(
///     state.is_dialog_open,
///     main_content_view,
///     column((
///         text("Confirm Action"),
///         button("OK").on_click(AppMsg::Confirm),
///     )).style(Style::new().bg_color(clr!(white)).padding(20.0).corner_radius(12.0)),
/// ).on_close(|| AppMsg::CloseDialog)
/// ```
#[track_caller]
pub fn modal<MainV, DialogV, Msg>(
    is_open: bool,
    main_view: MainV,
    dialog_view: DialogV,
) -> Layer<MainV, DialogV, Msg, fn() -> Msg> {
    layer(main_view, is_open, dialog_view)
        .source_loc(Some(SourceLocation::here("Modal")))
        .layer_id(crate::layer::ActiveLayerId::Modal)
        .transition(Transition::scale())
        .dim_background(true)
        .scrim_color(rgba!(0, 0, 0, 130))
        .set_modal(true)
        .close_on_escape(true)
        .close_on_backdrop(true)
}

/// Helper extension on `Layer` to rename `on_dismiss` to `on_close`.
pub trait ModalExt<BaseV, LayerV, Msg, F> {
    /// Sets the callback triggered when the user clicks the backdrop scrim or presses `Escape`.
    fn on_close<NewF: Fn() -> Msg>(self, on_close: NewF) -> Layer<BaseV, LayerV, Msg, NewF>;
}

impl<BaseV, LayerV, Msg, F> ModalExt<BaseV, LayerV, Msg, F> for Layer<BaseV, LayerV, Msg, F> {
    fn on_close<NewF: Fn() -> Msg>(self, on_close: NewF) -> Layer<BaseV, LayerV, Msg, NewF> {
        self.on_dismiss(on_close)
    }
}
