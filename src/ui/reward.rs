//! The post-fight reward screen: gold, and one spell from three.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::game::reward::OFFER_COUNT;
use crate::game::spell::SPELL_SLOTS;
use crate::ui::widgets::{focus_style, spell_card};

pub fn render(f: &mut Frame, app: &App) {
    let Some(state) = &app.reward else { return };

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Fill(1),
    ])
    .areas(f.area());

    f.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "The way is clear.",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::from(vec![
                Span::styled(
                    format!("+{} gold", state.reward.gold),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!("   (purse: {})", app.run.player.gold)),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Victory")
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        header,
    );

    // Offers, laid out in fixed-width slots so two offers do not stretch to
    // fill the row.
    let mut widths: Vec<Constraint> = vec![Constraint::Max(22); state.reward.offers.len()];
    widths.push(Constraint::Max(14)); // Skip
    widths.push(Constraint::Fill(1));
    let slots = Layout::horizontal(widths).split(body);

    for (i, spell) in state.reward.offers.iter().enumerate() {
        f.render_widget(
            spell_card(spell, format!(" {} ", spell.name), state.cursor == i, false),
            slots[i],
        );
    }

    let skip_index = state.skip_index();
    f.render_widget(
        Paragraph::new(vec![
            Line::raw(""),
            Line::styled("Take nothing", Style::default().fg(Color::DarkGray)),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Skip ")
                .border_style(focus_style(state.cursor == skip_index)),
        ),
        slots[skip_index],
    );

    let hint = if state.reward.offers.is_empty() {
        "You already know every spell.    Enter to move on."
    } else if app.run.player.spells.len() < SPELL_SLOTS {
        "←→ choose    Enter to take    (or pick Skip)"
    } else {
        "←→ choose    Enter to take -- your slots are full, so you will pick one to replace"
    };

    f.render_widget(
        Paragraph::new(vec![
            Line::styled(hint, Style::default().fg(Color::DarkGray)),
            Line::raw(""),
            Line::styled(
                format!(
                    "Spell slots: {}/{}",
                    app.run.player.spells.len(),
                    SPELL_SLOTS
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        footer,
    );

    debug_assert!(state.reward.offers.len() <= OFFER_COUNT);
}
