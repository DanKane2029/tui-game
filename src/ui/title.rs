//! The title screen.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, TitleEntry};

const BANNER: [&str; 6] = [
    r"  ___ _  _  ___ ___ _  _ _____ ___ _____ ___ ___  _  _ ",
    r" |_ _| \| |/ __/ _ \ \| |_   _/ _ \_   _|_ _/ _ \| \| |",
    r"  | || .` | (_| (_) | .` | | || (_) || |  | | (_) | .` |",
    r" |___|_|\_|\___\___/|_|\_| |_| \___/ |_| |___\___/|_|\_|",
    "",
    "        combine spells · climb the branching dark",
];

pub fn render(f: &mut Frame, app: &App) {
    let [banner_area, menu_area, footer] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Length(7),
        Constraint::Fill(1),
    ])
    .areas(f.area());

    f.render_widget(
        Paragraph::new(
            BANNER
                .iter()
                .map(|l| Line::styled(*l, Style::default().fg(Color::Magenta)))
                .collect::<Vec<_>>(),
        )
        .centered()
        .block(Block::default().borders(Borders::ALL)),
        banner_area,
    );

    let menu: Vec<Line> = TitleEntry::ALL
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let selected = i == app.ui.title_cursor;
            Line::styled(
                if selected {
                    format!("▸ {}", entry.label())
                } else {
                    format!("  {}", entry.label())
                },
                if selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            )
        })
        .collect();

    f.render_widget(
        Paragraph::new(menu)
            .centered()
            .block(Block::default().borders(Borders::ALL)),
        menu_area,
    );

    f.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "↑↓ choose    Enter to select    q to quit",
                Style::default().fg(Color::DarkGray),
            ),
            Line::raw(""),
            Line::styled(
                format!(
                    "{} · {} · log {}",
                    crate::game::options::Cycle::label(app.options.map_length),
                    crate::game::options::Cycle::label(app.options.difficulty),
                    crate::game::options::Cycle::label(app.options.log_speed),
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ])
        .centered(),
        footer,
    );
}
