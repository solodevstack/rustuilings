
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

use std::io::stdout;

use app::App;
use anyhow::Result;  // swap color_eyre for anyhow
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::Rect;
use ratatui::{TerminalOptions, Viewport};

pub use self::colors::{RgbSwatch, color_from_oklab};
pub use self::theme::THEME;

// pub fn ratatui_render() -> anyhow::Result<()> {
//     let viewport = Viewport::Fixed(Rect::new(0, 0, 81, 18));
//     let terminal = ratatui::init_with_options(TerminalOptions { viewport });
//     execute!(stdout(), EnterAlternateScreen)?;
//     let app_result = App::default().run(terminal)
//         .map_err(|e| anyhow::anyhow!(e));  // 👈 convert Report to anyhow::Error
//     execute!(stdout(), LeaveAlternateScreen)?;
//     ratatui::restore();
//     app_result
// }
pub fn ratatui_render() -> anyhow::Result<()> {
    let viewport = Viewport::Fixed(Rect::new(0, 0, 81, 18));
    let terminal = ratatui::init_with_options(TerminalOptions { viewport });
    let app_result = App::default().run(terminal)
        .map_err(|e| anyhow::anyhow!(e));
    ratatui::restore();
    app_result
}