//! Ratatui mascot widget with score-driven terminal color changes.
//!
//! The mascot takes 32x16 cells and is rendered using half block characters.
//! Terminal screen colors (TERM, TERM_BORDER, TERM_CURSOR) change as score increases.
//!
//! # Usage
//! ```rust
//! RatatuiMascot::new()
//!     .with_score(app_state.game_score())
//!     .set_eye(MascotEyeColor::Default)
//!     .render(area, buf);
//! ```

use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

const RATATUI_MASCOT: &str = indoc::indoc! {"

                   hhh
                 hhhhhh
                hhhhhhh
               hhhhhhhh
              hhhhhhhhh
             hhhhhhhhhh
            hhhhhhhhhhhh
            hhhhhhhhhhhhh
            hhhhhhhhhhhhh     ██████
             hhhhhhhhhhh    ████████
                  hhhhh ███████████
                   hhh ██ee████████
                    h █████████████
                ████ █████████████
               █████████████████
               ████████████████
               ████████████████
                ███ ██████████
              ▒▒    █████████
             ▒░░▒   █████████
            ▒░░░░▒ ██████████
           ▒░░▓░░░▒ █████████
          ▒░░▓▓░░░░▒ ████████
         ▒░░░░░░░░░░▒ ██████████
        ▒░░░░░░░░░░░░▒ ██████████
       ▒░░░░░░░▓▓░░░░░▒ █████████
      ▒░░░░░░░░░▓▓░░░░░▒ ████  ███
     ▒░░░░░░░░░░░░░░░░░░▒ ██   ███
    ▒░░░░░░░░░░░░░░░░░░░░▒ █   ███
    ▒░░░░░░░░░░░░░░░░░░░░░▒   ███
     ▒░░░░░░░░░░░░░░░░░░░░░▒ ███
      ▒░░░░░░░░░░░░░░░░░░░░░▒ █"
};

// ---------------------------------------------------------------------------
// Character constants — each char in the mascot string maps to a color
// ---------------------------------------------------------------------------
const EMPTY:       char = ' ';
const RAT:         char = '█';
const HAT:         char = 'h';
const EYE:         char = 'e';
const TERM:        char = '░'; // terminal screen fill    — changes with score
const TERM_BORDER: char = '▒'; // terminal screen border  — changes with score
const TERM_CURSOR: char = '▓'; // terminal cursor         — changes with score

// ---------------------------------------------------------------------------
// Score tiers — mirrors CrabTier in mascot.rs
// ---------------------------------------------------------------------------

/// Score tier that drives terminal color selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScoreTier {
    Hatchling,  // 0–19
    Clicker,    // 20–39
    Coder,      // 40–59
    Hacker,     // 60–79
    Rustacean,  // 80+
}

impl ScoreTier {
    fn from_score(score: u32) -> Self {
        match score {
            0..=19  => Self::Hatchling,
            20..=39 => Self::Clicker,
            40..=59 => Self::Coder,
            60..=79 => Self::Hacker,
            _       => Self::Rustacean,
        }
    }

    /// Returns (term_color, term_border_color, term_cursor_color)
    fn terminal_colors(self) -> (Color, Color, Color) {
        match self {
            // default dark terminal — no progress yet
            Self::Hatchling => (
                Color::Indexed(232), // vampire black
                Color::Indexed(237), // gray
                Color::Indexed(248), // dark gray
            ),
            // blue tint — Clicker
            Self::Clicker => (
                Color::Indexed(17),  // dark blue
                Color::Indexed(25),  // blue
                Color::Indexed(39),  // bright blue
            ),
            // teal tint — Coder
            Self::Coder => (
                Color::Indexed(22),  // dark green
                Color::Indexed(29),  // teal
                Color::Indexed(43),  // bright teal
            ),
            // purple — Hacker
            Self::Hacker => (
                Color::Indexed(53),  // dark purple
                Color::Indexed(61),  // purple
                Color::Indexed(99),  // bright purple
            ),
            // rust red — Rustacean
            Self::Rustacean => (
                Color::Indexed(52),  // dark red
                Color::Indexed(88),  // rust red
                Color::Indexed(160), // bright red
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Eye state
// ---------------------------------------------------------------------------

/// Controls the mascot's eye color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MascotEyeColor {
    /// Default dark eye.
    #[default]
    Default,
    /// Red blinking eye — shown on odd row_index.
    Red,
}

// ---------------------------------------------------------------------------
// RatatuiMascot widget
// ---------------------------------------------------------------------------

/// Ratatui mascot widget with score-driven terminal colors.
///
/// Build with the builder pattern:
/// ```rust
/// RatatuiMascot::new()
///     .with_score(score)
///     .set_eye(MascotEyeColor::Default)
///     .render(area, buf);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatatuiMascot {
    /// Current game score — drives terminal color tier.
    pub score:         u32,
    eye_state:         MascotEyeColor,
    // Rat body
    rat_color:         Color,
    // Eye colors
    rat_eye_color:     Color,
    rat_eye_blink:     Color,
    // Hat
    hat_color:         Color,
    // Terminal screen — all three change with score
    term_color:        Color,
    term_border_color: Color,
    term_cursor_color: Color,
}

impl Default for RatatuiMascot {
    fn default() -> Self {
        // Terminal colors start at Hatchling (score 0).
        let (term, border, cursor) = ScoreTier::Hatchling.terminal_colors();
        Self {
            score:             0,
            eye_state:         MascotEyeColor::Default,
            rat_color:         Color::Indexed(252), // light gray  #d0d0d0
            hat_color:         Color::Indexed(231), // white       #ffffff
            rat_eye_color:     Color::Indexed(236), // dark charcoal #303030
            rat_eye_blink:     Color::Indexed(196), // red         #ff0000
            term_color:        term,
            term_border_color: border,
            term_cursor_color: cursor,
        }
    }
}

impl RatatuiMascot {
    /// Create a new mascot with default colors (score = 0).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the score and re-derive terminal colors from the matching tier.
    ///
    /// | Score  | Tier       | Terminal color |
    /// |--------|------------|----------------|
    /// | 0–19   | Hatchling  | Dark black     |
    /// | 20–39  | Clicker    | Blue           |
    /// | 40–59  | Coder      | Teal           |
    /// | 60–79  | Hacker     | Purple         |
    /// | 80+    | Rustacean  | Rust red       |
    #[must_use]
    pub fn with_score(self, score: u32) -> Self {
        let (term, border, cursor) = ScoreTier::from_score(score).terminal_colors();
        Self {
            score,
            term_color:        term,
            term_border_color: border,
            term_cursor_color: cursor,
            ..self
        }
    }

    /// Set the eye state (open / blinking).
    #[must_use]
    pub const fn set_eye(self, rat_eye: MascotEyeColor) -> Self {
        Self { eye_state: rat_eye, ..self }
    }

    /// Map a mascot character to its rendered color.
    const fn color_for(&self, c: char) -> Option<Color> {
        match c {
            RAT         => Some(self.rat_color),
            HAT         => Some(self.hat_color),
            EYE         => Some(match self.eye_state {
                MascotEyeColor::Default => self.rat_eye_color,
                MascotEyeColor::Red     => self.rat_eye_blink,
            }),
            TERM        => Some(self.term_color),
            TERM_CURSOR => Some(self.term_cursor_color),
            TERM_BORDER => Some(self.term_border_color),
            _           => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Widget implementation
// ---------------------------------------------------------------------------

impl Widget for RatatuiMascot {
    /// Render the mascot using half-block characters.
    /// The logo string is processed two lines at a time — each pair of rows
    /// becomes one row of terminal cells using ▀ / ▄ / █ block characters.
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }

        for (y, (line1, line2)) in RATATUI_MASCOT.lines().tuples().enumerate() {
            for (x, (ch1, ch2)) in line1.chars().zip(line2.chars()).enumerate() {
                let x = area.left() + x as u16;
                let y = area.top() + y as u16;

                if x >= area.right() || y >= area.bottom() {
                    continue;
                }

                let cell = &mut buf[(x, y)];

                let (fg, bg) = match (ch1, ch2) {
                    (EMPTY, EMPTY) => (None, None),
                    (c, EMPTY) | (EMPTY, c) => (self.color_for(c), None),
                    (TERM, TERM_BORDER) => (self.color_for(TERM_BORDER), self.color_for(TERM)),
                    (TERM, c) | (c, TERM) => (self.color_for(c), self.color_for(TERM)),
                    (c1, c2) => (self.color_for(c1), self.color_for(c2)),
                };

                let symbol = match (ch1, ch2) {
                    (EMPTY, EMPTY)     => None,
                    (TERM, TERM)       => Some(EMPTY),
                    (_, EMPTY | TERM)  => Some('▀'),
                    (EMPTY | TERM, _)  => Some('▄'),
                    (c, d) if c == d   => Some('█'),
                    (_, _)             => Some('▀'),
                };

                if let Some(fg) = fg { cell.fg = fg; }
                if let Some(bg) = bg { cell.bg = bg; }
                if let Some(symb) = symbol { cell.set_char(symb); }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constructor & defaults ───────────────────────────────────────────────

    #[test]
    fn new_mascot_has_default_eye() {
        let mascot = RatatuiMascot::new();
        assert_eq!(mascot.eye_state, MascotEyeColor::Default);
    }

    #[test]
    fn new_mascot_has_zero_score() {
        let mascot = RatatuiMascot::new();
        assert_eq!(mascot.score, 0);
    }

    #[test]
    fn new_mascot_uses_hatchling_terminal_colors() {
        let mascot    = RatatuiMascot::new();
        let (t, b, c) = ScoreTier::Hatchling.terminal_colors();
        assert_eq!(mascot.term_color,        t);
        assert_eq!(mascot.term_border_color, b);
        assert_eq!(mascot.term_cursor_color, c);
    }

    // ── set_eye ──────────────────────────────────────────────────────────────

    #[test]
    fn set_eye_changes_eye_state() {
        let mascot = RatatuiMascot::new().set_eye(MascotEyeColor::Red);
        assert_eq!(mascot.eye_state, MascotEyeColor::Red);
    }

    #[test]
    fn set_eye_does_not_change_score() {
        let mascot = RatatuiMascot::new()
            .with_score(50)
            .set_eye(MascotEyeColor::Red);
        assert_eq!(mascot.score, 50);
    }

    // ── with_score ───────────────────────────────────────────────────────────

    #[test]
    fn with_score_stores_score() {
        let mascot = RatatuiMascot::new().with_score(42);
        assert_eq!(mascot.score, 42);
    }

    #[test]
    fn with_score_does_not_change_eye_state() {
        let mascot = RatatuiMascot::new()
            .set_eye(MascotEyeColor::Red)
            .with_score(80);
        assert_eq!(mascot.eye_state, MascotEyeColor::Red);
    }

    #[test]
    fn with_score_does_not_change_rat_color() {
        let base   = RatatuiMascot::new();
        let scored = RatatuiMascot::new().with_score(99);
        assert_eq!(base.rat_color, scored.rat_color);
    }

    // ── ScoreTier::from_score ────────────────────────────────────────────────

    #[test]
    fn score_tier_hatchling() {
        assert_eq!(ScoreTier::from_score(0),  ScoreTier::Hatchling);
        assert_eq!(ScoreTier::from_score(19), ScoreTier::Hatchling);
    }

    #[test]
    fn score_tier_clicker() {
        assert_eq!(ScoreTier::from_score(20), ScoreTier::Clicker);
        assert_eq!(ScoreTier::from_score(39), ScoreTier::Clicker);
    }

    #[test]
    fn score_tier_coder() {
        assert_eq!(ScoreTier::from_score(40), ScoreTier::Coder);
        assert_eq!(ScoreTier::from_score(59), ScoreTier::Coder);
    }

    #[test]
    fn score_tier_hacker() {
        assert_eq!(ScoreTier::from_score(60), ScoreTier::Hacker);
        assert_eq!(ScoreTier::from_score(79), ScoreTier::Hacker);
    }

    #[test]
    fn score_tier_rustacean() {
        assert_eq!(ScoreTier::from_score(80),  ScoreTier::Rustacean);
        assert_eq!(ScoreTier::from_score(100), ScoreTier::Rustacean);
    }

    // ── Terminal colors change with score ────────────────────────────────────

    #[test]
    fn terminal_colors_differ_between_tiers() {
        let hatchling = RatatuiMascot::new().with_score(0);
        let rustacean = RatatuiMascot::new().with_score(80);
        assert_ne!(hatchling.term_color,        rustacean.term_color);
        assert_ne!(hatchling.term_border_color, rustacean.term_border_color);
        assert_ne!(hatchling.term_cursor_color, rustacean.term_cursor_color);
    }

    #[test]
    fn clicker_terminal_color_is_blue() {
        let mascot = RatatuiMascot::new().with_score(20);
        assert_eq!(mascot.term_border_color, Color::Indexed(25)); // blue
    }

    #[test]
    fn coder_terminal_color_is_teal() {
        let mascot = RatatuiMascot::new().with_score(40);
        assert_eq!(mascot.term_border_color, Color::Indexed(29)); // teal
    }

    #[test]
    fn hacker_terminal_color_is_purple() {
        let mascot = RatatuiMascot::new().with_score(60);
        assert_eq!(mascot.term_border_color, Color::Indexed(61)); // purple
    }

    #[test]
    fn rustacean_terminal_color_is_red() {
        let mascot = RatatuiMascot::new().with_score(80);
        assert_eq!(mascot.term_border_color, Color::Indexed(88)); // rust red
    }

    // ── render — buffer tests ────────────────────────────────────────────────

    #[test]
    fn set_eye_color_renders_correctly() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 32, 16));
        let mascot  = RatatuiMascot::new().set_eye(MascotEyeColor::Red);
        mascot.render(buf.area, &mut buf);
        assert_eq!(mascot.eye_state, MascotEyeColor::Red);
        assert_eq!(buf[(21, 5)].bg, Color::Indexed(196));
    }

    #[test]
    fn render_mascot_default_eye_bg() {
        let mascot  = RatatuiMascot::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 32, 16));
        mascot.render(buf.area, &mut buf);
        assert_eq!(buf[(21, 5)].bg, Color::Indexed(236));
    }

    #[test]
    fn render_mascot_correct_size() {
        let mascot  = RatatuiMascot::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 32, 16));
        mascot.render(buf.area, &mut buf);
        assert_eq!(buf.area.as_size(), (32, 16).into());
    }

    #[test]
    fn render_mascot_correct_content() {
        let mascot  = RatatuiMascot::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 32, 16));
        mascot.render(buf.area, &mut buf);
        assert_eq!(
            buf.content
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>(),
            Buffer::with_lines([
                "             ▄▄███              ",
                "           ▄███████             ",
                "         ▄█████████             ",
                "        ████████████            ",
                "        ▀███████████▀   ▄▄██████",
                "              ▀███▀▄█▀▀████████ ",
                "            ▄▄▄▄▀▄████████████  ",
                "           ████████████████     ",
                "           ▀███▀██████████      ",
                "         ▄▀▀▄   █████████       ",
                "       ▄▀ ▄  ▀▄▀█████████       ",
                "     ▄▀  ▀▀    ▀▄▀███████       ",
                "   ▄▀      ▄▄    ▀▄▀█████████   ",
                " ▄▀         ▀▀     ▀▄▀██▀  ███  ",
                "█                    ▀▄▀  ▄██   ",
                " ▀▄                    ▀▄▀█     ",
            ])
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>()
        );
    }

    #[test]
    fn render_in_minimal_buffer_does_not_panic() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        RatatuiMascot::new().render(buf.area, &mut buf);
        assert_eq!(buf, Buffer::with_lines([" "]));
    }

    #[test]
    fn render_in_zero_size_buffer_does_not_panic() {
        let mut buf = Buffer::empty(Rect::ZERO);
        RatatuiMascot::new().render(buf.area, &mut buf);
    }

    #[test]
    fn render_with_score_does_not_panic() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 32, 16));
        RatatuiMascot::new()
            .with_score(100)
            .set_eye(MascotEyeColor::Red)
            .render(buf.area, &mut buf);
    }
}