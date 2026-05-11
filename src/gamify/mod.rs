
#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

mod app;
mod colors;
mod destroy;
mod tabs;
mod theme;
mod mascot;


use crate::{
    app_state::AppState,
};

use app::App;
use ratatui::layout::Rect;
use ratatui::{TerminalOptions, Viewport};



pub use self::colors::{RgbSwatch, color_from_oklab};
pub use self::theme::THEME;


pub fn ratatui_render(app_state :&mut  AppState) -> anyhow::Result<()> {
    let viewport = Viewport::Fixed(Rect::new(0, 0, 81, 18));
    let terminal = ratatui::init_with_options(TerminalOptions { viewport });
    let app_result = App::new(app_state).run(terminal)
        .map_err(|e| anyhow::anyhow!(e));
    ratatui::restore();
    app_result
}