//! The event screen: a prompt, some choices, and what came of it.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let Some(active) = &app.event else { return };

    let [prompt_area, body_area] =
        Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]).areas(crate::ui::stage(f));

    f.render_widget(
        Paragraph::new(active.event.prompt.clone())
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(active.event.name.clone()),
            ),
        prompt_area,
    );

    let lines: Vec<Line> = match &active.result {
        // A choice has been made: show the outcome and wait for a dismiss.
        Some(text) => vec![
            Line::styled(
                text.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::styled("Enter to continue", Style::default().fg(Color::DarkGray)),
        ],
        None => {
            let mut lines: Vec<Line> = active
                .event
                .choices
                .iter()
                .enumerate()
                .map(|(i, choice)| {
                    let selected = i == app.ui.choice_cursor;
                    Line::from(vec![
                        Span::styled(
                            if selected { " ▸ " } else { "   " },
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(
                            choice.text.clone(),
                            if selected {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            },
                        ),
                    ])
                })
                .collect();
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                "  ↑/↓ choose    Enter to commit",
                Style::default().fg(Color::DarkGray),
            ));
            lines
        }
    };

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL)),
        body_area,
    );
}
