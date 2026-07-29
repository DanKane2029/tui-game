//! The map screen: pick where to go next.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::widgets::{health_color, meter};

pub fn render(f: &mut Frame, app: &App) {
    let [map_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(4)]).areas(f.area());

    let available = app.nodes_available();
    let selected = available.get(app.ui.map_cursor).copied();

    // Top row (the boss) first, so the map reads as a climb.
    let mut lines: Vec<Line> = Vec::new();
    for row in app.run.map.rows.iter().rev() {
        let mut spans: Vec<Span> = vec![Span::raw("   ")];
        for &id in row {
            let node = app.run.map.node(id);
            let is_available = available.contains(&id);
            let is_selected = selected == Some(id);
            let is_visited = app.run.visited.contains(&id);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else if is_available {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_visited {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            spans.push(Span::styled(format!("  {}  ", node.kind.glyph()), style));
        }
        lines.push(Line::from(spans));
        lines.push(Line::raw(""));
    }

    lines.push(Line::styled(
        "  ⚔ fight    ? event    ☠ boss",
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::styled(
        "  ←/→ choose    Enter to travel    q to quit",
        Style::default().fg(Color::DarkGray),
    ));

    let title = match selected {
        Some(id) => format!("The Climb -- next: {}", app.run.map.node(id).kind.label()),
        None => "The Climb".to_string(),
    };

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title)),
        map_area,
    );

    let player = &app.run.player;
    let status = vec![
        Line::from(vec![
            Span::raw("HP "),
            Span::styled(
                meter(player.hp, player.max_hp, 10),
                Style::default().fg(health_color(player.hp, player.max_hp)),
            ),
            Span::raw(format!(" {}/{}   ", player.hp, player.max_hp)),
            Span::raw("MP "),
            Span::styled(
                meter(u16::from(player.mana), u16::from(player.max_mana), 6),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(format!(" {}/{}", player.mana, player.max_mana)),
        ]),
        Line::raw(
            player
                .spells
                .iter()
                .map(|s| format!("{} ({}MP)", s.name, s.mana_cost))
                .collect::<Vec<_>>()
                .join("   "),
        ),
    ];

    f.render_widget(
        Paragraph::new(status).block(Block::default().borders(Borders::ALL).title("Spellbook")),
        status_area,
    );
}
