//! The fight screen.
//!
//! Laid out top to bottom in the order the player thinks about them: enemies,
//! what just happened, your state and what you are building, then the spells
//! you build it from. Focus moves down that stack with the arrow keys, so the
//! spatial layout and the control scheme agree.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{ACTION_CAST, ACTION_END_TURN, App, Focus};
use crate::game::combat::{Combat, Event};
use crate::game::entity::{Enemy, Player};
use crate::game::spell::SPELL_SLOTS;
use crate::ui::widgets::{
    element_color, focus_style, health_color, meter, spell_card, status_color,
};

pub fn render(f: &mut Frame, app: &App) {
    let Some(combat) = &app.combat else { return };

    // Spare height goes to the enemy row, which has art to show. The log is
    // capped: letting it absorb the slack means a very tall terminal -- or a
    // maximised browser window -- turns the screen into mostly empty log.
    let [enemies_area, log_area, mid_area, spells_area] = Layout::vertical([
        Constraint::Min(9),
        Constraint::Max(10),
        Constraint::Length(5),
        Constraint::Length(8),
    ])
    .areas(crate::ui::stage(f));

    render_enemies(f, enemies_area, app, combat);
    render_log(f, log_area, app);

    let [stats_area, incantation_area] =
        Layout::horizontal([Constraint::Length(26), Constraint::Fill(1)]).areas(mid_area);
    render_stats(f, stats_area, &app.run.player, combat);
    render_incantation(f, incantation_area, app, combat);

    render_spells(f, spells_area, app, combat);
}

fn render_enemies(f: &mut Frame, area: Rect, app: &App, combat: &Combat) {
    let focused = app.ui.focus == Focus::Enemies;

    // One panel groups the whole row, so the enemies read as a single zone --
    // which is also what the arrow keys treat them as.
    let group = Block::default()
        .borders(Borders::ALL)
        .title("Enemies")
        .border_style(focus_style(focused));
    let inner = group.inner(area);
    f.render_widget(group, area);

    let living: Vec<(usize, &Enemy)> = combat.living_enemies().collect();
    if living.is_empty() {
        return;
    }

    // Cap each entry and let fillers on both sides absorb the slack, so one
    // enemy sits centred at a sensible size rather than stretching across the
    // screen or hugging the left edge.
    let mut widths: Vec<Constraint> = vec![Constraint::Fill(1)];
    widths.extend(vec![Constraint::Max(30); living.len()]);
    widths.push(Constraint::Fill(1));
    let slots = Layout::horizontal(widths).split(inner);

    for ((index, enemy), rect) in living.iter().zip(slots.iter().skip(1)) {
        let targeted = *index == combat.target;

        let mut lines: Vec<Line> = enemy
            .art
            .iter()
            .map(|row| {
                Line::styled(
                    row.clone(),
                    Style::default().fg(element_color(enemy.element)),
                )
            })
            .collect();

        lines.push(Line::from(vec![
            Span::raw("HP "),
            Span::styled(
                meter(enemy.hp, enemy.max_hp, 6),
                Style::default().fg(health_color(enemy.hp, enemy.max_hp)),
            ),
            Span::raw(format!(" {}/{}", enemy.hp, enemy.max_hp)),
        ]));

        let mut footer = vec![Span::styled(
            enemy.intent.label(),
            Style::default().fg(Color::LightRed),
        )];
        if enemy.armor > 0 {
            footer.push(Span::styled(
                format!("  armor {}", enemy.armor),
                Style::default().fg(Color::Gray),
            ));
        }
        lines.push(Line::from(footer));

        if !enemy.statuses.is_empty() {
            lines.push(Line::from(
                enemy
                    .statuses
                    .iter()
                    .map(|a| {
                        Span::styled(
                            format!("{} ({}) ", a.status.name(), a.rounds),
                            Style::default().fg(status_color(a.status)),
                        )
                    })
                    .collect::<Vec<_>>(),
            ));
        }

        // No inner border: the group panel already frames them, and doubling
        // borders wastes two rows of height for nothing. The target marker and
        // colour carry the selection instead.
        let name_style = if targeted {
            focus_style(true)
        } else {
            Style::default().fg(Color::Gray)
        };
        lines.insert(
            0,
            Line::styled(
                if targeted {
                    format!("▸ {}", enemy.name)
                } else {
                    format!("  {}", enemy.name)
                },
                name_style,
            ),
        );

        f.render_widget(Paragraph::new(lines), *rect);
    }
}

fn render_log(f: &mut Frame, area: Rect, app: &App) {
    let visible = app.visible_log();
    let height = area.height.saturating_sub(2) as usize;
    let start = visible.len().saturating_sub(height);
    let lines: Vec<Line> = visible[start..].iter().map(describe).collect();

    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Log")
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        area,
    );
}

/// Formatting is presentation, so `Event` stays a plain fact and this decides
/// how it reads.
fn describe(event: &Event) -> Line<'static> {
    match event {
        Event::Cast { name } => Line::styled(
            format!("You cast {name}."),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Event::Damaged { target, amount } => {
            Line::raw(format!("  {target} takes {amount} damage."))
        }
        Event::StatusApplied { target, status } => Line::styled(
            format!("  {target} is {}.", status.name()),
            Style::default().fg(status_color(*status)),
        ),
        Event::Died { name } => Line::styled(
            format!("  {name} falls."),
            Style::default().fg(Color::DarkGray),
        ),
        Event::EnemyAttacked { name, amount } => Line::styled(
            format!("{name} hits you for {amount}."),
            Style::default().fg(Color::LightRed),
        ),
        Event::StatusTicked { target, amount } => Line::styled(
            format!("  {target} suffers {amount}."),
            Style::default().fg(Color::Magenta),
        ),
        Event::TurnEnded => Line::styled("-- turn --", Style::default().fg(Color::DarkGray)),
        Event::Refused(why) => Line::styled(
            format!("({why})"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
        Event::Won => Line::styled(
            "The way is clear.",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Event::Lost => Line::styled(
            "You fall.",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    }
}

fn render_stats(f: &mut Frame, area: Rect, player: &Player, combat: &Combat) {
    let mut lines = vec![
        Line::from(vec![
            Span::raw("HP "),
            Span::styled(
                meter(player.hp, player.max_hp, 6),
                Style::default().fg(health_color(player.hp, player.max_hp)),
            ),
            Span::raw(format!(" {}/{}", player.hp, player.max_hp)),
        ]),
        Line::from(vec![
            Span::raw("MP "),
            Span::styled(
                meter(u16::from(player.mana), u16::from(player.max_mana), 6),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(format!(" {}/{}", player.mana, player.max_mana)),
        ]),
    ];

    if player.statuses.is_empty() {
        lines.push(Line::styled(
            format!("Round {}", combat.round),
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        lines.push(Line::from(
            player
                .statuses
                .iter()
                .map(|a| {
                    Span::styled(
                        format!("{} ({}) ", a.status.name(), a.rounds),
                        Style::default().fg(status_color(a.status)),
                    )
                })
                .collect::<Vec<_>>(),
        ));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("You")
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        area,
    );
}

fn render_incantation(f: &mut Frame, area: Rect, app: &App, combat: &Combat) {
    let player = &app.run.player;
    let components = combat.build_spells(player);
    let focused = app.ui.focus == Focus::Actions;

    let mut build = vec![Span::raw("Build: ")];
    if components.is_empty() {
        build.push(Span::styled(
            "(empty)",
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        for (i, spell) in components.iter().enumerate() {
            if i > 0 {
                build.push(Span::raw(" + "));
            }
            build.push(Span::styled(
                format!("[{}]", spell.name),
                Style::default().fg(element_color(spell.element)),
            ));
        }
    }

    let result = match combat.preview(player, &app.content.rules) {
        Some(spell) => {
            let mut spans = vec![
                Span::styled(
                    spell.name.to_uppercase(),
                    Style::default()
                        .fg(element_color(spell.element))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(
                    "  {} dmg · {} · {} MP",
                    spell.damage,
                    spell.targeting.label(),
                    spell.mana_cost
                )),
            ];
            if spell.pierce {
                spans.push(Span::styled(
                    "  pierces",
                    Style::default().fg(Color::LightGreen),
                ));
            }
            for (status, rounds) in &spell.statuses {
                spans.push(Span::styled(
                    format!("  {} ({rounds})", status.name()),
                    Style::default().fg(status_color(*status)),
                ));
            }
            Line::from(spans)
        }
        None => Line::styled(
            "pick spells below to build a spell",
            Style::default().fg(Color::DarkGray),
        ),
    };

    let button = |label: &'static str, selected: bool| {
        let style = if focused && selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        Span::styled(format!(" {label} "), style)
    };

    let actions = Line::from(vec![
        button("Cast", app.ui.action_cursor == ACTION_CAST),
        Span::raw("  "),
        button("End Turn", app.ui.action_cursor == ACTION_END_TURN),
        Span::styled(
            "     ↑↓ move · ←→ pick · Enter do · Bksp undo",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    f.render_widget(
        Paragraph::new(vec![Line::from(build), result, actions]).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Incantation")
                .border_style(focus_style(focused)),
        ),
        area,
    );
}

fn render_spells(f: &mut Frame, area: Rect, app: &App, combat: &Combat) {
    let player = &app.run.player;
    let committed = combat.committed_mana(player);
    let focused = app.ui.focus == Focus::Spells;

    let widths = vec![Constraint::Ratio(1, SPELL_SLOTS as u32); SPELL_SLOTS];
    let rects = Layout::horizontal(widths).split(area);

    for (i, rect) in rects.iter().enumerate() {
        let Some(spell) = player.spell(i) else {
            f.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", i + 1))
                    .border_style(Style::default().fg(Color::DarkGray)),
                *rect,
            );
            continue;
        };

        let selected = focused && i == app.ui.spell_cursor;
        // Dim anything the remaining mana cannot pay for, including what is
        // already committed to the build.
        let affordable = committed.saturating_add(spell.mana_cost) <= player.mana;

        let label = if selected {
            format!("▸{} {}", i + 1, spell.name)
        } else {
            format!(" {} {}", i + 1, spell.name)
        };

        f.render_widget(spell_card(spell, label, selected, !affordable), *rect);
    }
}
