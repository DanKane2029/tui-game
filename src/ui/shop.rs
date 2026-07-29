//! The shop screen: somewhere for gold to go.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::widgets::focus_style;

pub fn render(f: &mut Frame, app: &App) {
    let Some(shop) = &app.shop else { return };

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(4),
    ])
    .areas(f.area());

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("Purse: "),
            Span::styled(
                format!("{} gold", app.run.player.gold),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL).title("Shop")),
        header,
    );

    let mut lines: Vec<Line> = shop
        .stock
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let selected = i == app.ui.shop_cursor;
            let affordable = app.run.player.gold >= entry.price && !entry.sold;

            let name_style = if entry.sold {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else if selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if affordable {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            Line::from(vec![
                Span::styled(if selected { " ▸ " } else { "   " }, name_style),
                Span::styled(format!("{:<26}", entry.item.name()), name_style),
                Span::styled(
                    format!("{:>5} g   ", entry.price),
                    if entry.sold {
                        Style::default().fg(Color::DarkGray)
                    } else if affordable {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
                Span::styled(
                    if entry.sold {
                        "sold".to_string()
                    } else {
                        entry.item.description()
                    },
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        })
        .collect();

    // The entry past the end of the stock leaves.
    let leave_index = shop.stock.len();
    let leaving = app.ui.shop_cursor == leave_index;
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        if leaving { " ▸ Leave" } else { "   Leave" },
        if leaving {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        },
    ));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Wares")
                .border_style(focus_style(true)),
        ),
        body,
    );

    let mut footer_lines = vec![Line::styled(
        "↑↓ choose    Enter to buy    q to quit",
        Style::default().fg(Color::DarkGray),
    )];
    if let Some(message) = &app.ui.message {
        footer_lines.push(Line::styled(
            message.clone(),
            Style::default().fg(Color::Red),
        ));
    }

    f.render_widget(
        Paragraph::new(footer_lines).block(Block::default().borders(Borders::ALL)),
        footer,
    );
}
