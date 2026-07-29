//! The end-of-run screen.

use ratatui::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, RunOutcome};

pub fn render(f: &mut Frame, app: &App) {
    let won = app.outcome == Some(RunOutcome::Won);

    let (banner, color) = if won {
        ("THE CLIMB IS YOURS", Color::Green)
    } else {
        ("YOU FALL", Color::Red)
    };

    let lines = vec![
        Line::raw(""),
        Line::styled(
            banner,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
        Line::raw(format!("Depth reached   {}", app.run.depth() + 1)),
        Line::raw(format!("Nodes cleared   {}", app.run.visited.len())),
        Line::raw(format!("Seed            {}", app.run.seed)),
        Line::raw(""),
        Line::raw(
            app.run
                .player
                .spells
                .iter()
                .map(|s| s.name.clone())
                .collect::<Vec<_>>()
                .join("   "),
        ),
        Line::raw(""),
        Line::styled(
            "r to climb again    q to quit",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .centered()
            .block(Block::default().borders(Borders::ALL)),
        f.area(),
    );
}
