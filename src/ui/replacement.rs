//! "Which spell does this replace?"
//!
//! Shared by the reward and shop screens -- both can hand the player a spell
//! when every slot is already full.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Screen};
use crate::game::spell::SPELL_SLOTS;
use crate::ui::widgets::spell_card;

pub fn render(f: &mut Frame, app: &App) {
    let Some(pending) = &app.pending else { return };

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Fill(1),
    ])
    .areas(crate::ui::stage(f));

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Taking "),
                Span::styled(
                    pending.incoming.name.clone(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(
                    "  ({} · pow {} · {} MP)",
                    pending.incoming.element.name(),
                    pending.incoming.power,
                    pending.incoming.mana_cost
                )),
            ]),
            Line::styled(
                pending.incoming.blurb.clone(),
                Style::default().fg(Color::DarkGray),
            ),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Your spell slots are full")
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        header,
    );

    let widths = vec![Constraint::Ratio(1, SPELL_SLOTS as u32); SPELL_SLOTS];
    let slots = Layout::horizontal(widths).split(body);
    for (i, spell) in app.run.player.spells.iter().enumerate() {
        f.render_widget(
            spell_card(
                spell,
                format!(" {} {} ", i + 1, spell.name),
                pending.cursor == i,
                false,
            ),
            slots[i],
        );
    }

    // Backing out of a shop purchase forfeits gold already spent, so say so.
    let warning = if pending.return_to == Screen::Shop {
        "Backspace to cancel -- the gold is already spent and will not be refunded."
    } else {
        "Backspace to cancel and take nothing."
    };

    f.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "←→ choose which spell to discard    Enter to confirm",
                Style::default().fg(Color::DarkGray),
            ),
            Line::raw(""),
            Line::styled(warning, Style::default().fg(Color::DarkGray)),
        ])
        .block(Block::default().borders(Borders::ALL)),
        footer,
    );
}
