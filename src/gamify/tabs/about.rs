use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap};

use crate::gamify::mascot::{MascotEyeColor, RatatuiMascot};
use crate::gamify::{RgbSwatch, THEME};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AboutTab {
    pub row_index:   usize,
    pub score:       u32,
    pub last_output: String,
}

impl AboutTab {
    pub fn prev_row(&mut self) {
        self.row_index = self.row_index.saturating_sub(1);
    }

    pub fn next_row(&mut self) {
        self.row_index = self.row_index.saturating_add(1);
    }
}

impl Widget for AboutTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        RgbSwatch.render(area, buf);
        let layout = Layout::horizontal([Constraint::Length(34), Constraint::Min(0)]);
        let [logo_area, description] = area.layout(&layout);

        render_crate_description(description, buf, &self.last_output, self.row_index);

        let eye_state = if self.row_index % 2 == 0 {
            MascotEyeColor::Default
        } else {
            MascotEyeColor::Red
        };

        RatatuiMascot::new()
            .with_score(self.score)
            .set_eye(eye_state)
            .render(
                logo_area.inner(Margin {
                    vertical: 0,
                    horizontal: 2,
                }),
                buf,
            );
    }
}

fn render_crate_description(
    area: Rect,
    buf: &mut Buffer,
    output: &str,
    row_index: usize,
) {
    let area = area.inner(Margin {
        vertical: 4,
        horizontal: 2,
    });
    Clear.render(area, buf);
    Block::new().style(THEME.content).render(area, buf);
    let area = area.inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let (text, title, text_style) = if output.is_empty() {
        (
            "- cooking up terminal user interfaces -\n\n\
             Ratatui is a Rust crate that provides widgets \
             (e.g. Paragraph, Table) and draws them to the \
             screen efficiently every frame."
                .to_string(),
            " Ratatui ",
            THEME.description,
        )
    } else {
        (
            output.to_string(),
            " Exercise Output ",
            Style::default().fg(Color::Red),
        )
    };

    Paragraph::new(text.as_str())
        .style(text_style)
        .block(
            Block::new()
                .title(title)
                .title_alignment(Alignment::Center)
                .borders(Borders::TOP)
                .border_style(THEME.description_title)
                .padding(Padding::new(0, 0, 0, 0)),
        )
        .wrap(Wrap { trim: true })
        .scroll((row_index as u16, 0))
        .render(area, buf);
}