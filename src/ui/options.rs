//! The options screen. Every setting cycles with left/right, so nothing here
//! needs text entry.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::game::options::{Cycle, OptionField};

pub fn render(f: &mut Frame, app: &App) {
    let [body, footer] =
        Layout::vertical([Constraint::Length(9), Constraint::Fill(1)]).areas(f.area());

    let lines: Vec<Line> = OptionField::ALL
        .iter()
        .map(|field| {
            let selected = *field == app.ui.option_field;
            let style = if selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(vec![
                Span::styled(if selected { " ▸ " } else { "   " }, style),
                Span::styled(format!("{:<14}", field.label()), style),
                Span::styled(
                    format!("◂ {:^20} ▸", app.options.value_label(*field)),
                    if selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ])
        })
        .collect();

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Options")),
        body,
    );

    f.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "↑↓ choose setting    ←→ change it    Enter to go back",
                Style::default().fg(Color::DarkGray),
            ),
            Line::raw(""),
            Line::styled(
                "Options apply to the next run you start.",
                Style::default().fg(Color::DarkGray),
            ),
        ])
        .centered(),
        footer,
    );
}
