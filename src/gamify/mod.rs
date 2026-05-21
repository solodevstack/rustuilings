#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

mod app;
mod colors;
mod destroy;
mod mascot;
mod tabs;
mod theme;

use crate::app_state::AppState;
use app::App;
use ratatui::layout::Rect;
use ratatui::{TerminalOptions, Viewport};

pub use self::colors::{RgbSwatch, color_from_oklab};
pub use self::theme::THEME;

pub fn ratatui_render(app_state: &mut AppState) -> anyhow::Result<()> {
    let viewport = Viewport::Fixed(Rect::new(0, 0, 81, 18));
    let terminal = ratatui::init_with_options(TerminalOptions { viewport });

    // run current exercise once before entering game so output is fresh
    {
        use crate::exercise::{OUTPUT_CAPACITY, RunnableExercise};
        let mut output = Vec::with_capacity(OUTPUT_CAPACITY);
        let _ = app_state
            .current_exercise()
            .run_exercise(Some(&mut output), app_state.cmd_runner());
        app_state.set_last_output(&output);
    }

    let app_result = App::new(app_state)
        .run(terminal, app_state)   // 👈 just 2 args
        .map_err(|e| anyhow::anyhow!(e));

    ratatui::restore();
    app_result
}