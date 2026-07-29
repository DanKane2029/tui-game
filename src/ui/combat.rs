//! The fight screen.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::game::combat::{Combat, Event};
use crate::game::entity::{Enemy, Player};
use crate::ui::widgets::{element_color, health_color, meter, status_color};

pub fn render(f: &mut Frame, app: &App) {
    let Some(combat) = &app.combat else { return };

    let [top, log_area, build_area, slots_area] = Layout::vertical([
        Constraint::Min(8),
        Constraint::Length(6),
        Constraint::Length(4),
        Constraint::Length(3),
    ])
    .areas(f.area());

    render_combatants(f, top, app, combat);
    render_log(f, log_area, app);
    render_build(f, build_area, app, combat);
    render_slots(f, slots_area, app, combat);
}

fn render_combatants(f: &mut Frame, area: Rect, app: &App, combat: &Combat) {
    let [player_area, enemies_area] =
        Layout::horizontal([Constraint::Percentage(26), Constraint::Fill(1)]).areas(area);

    render_player(f, player_area, &app.run.player, combat);

    let living: Vec<(usize, &Enemy)> = combat.living_enemies().collect();
    if living.is_empty() {
        return;
    }
    let widths = vec![Constraint::Ratio(1, living.len() as u32); living.len()];
    let slots = Layout::horizontal(widths).split(enemies_area);

    for ((index, enemy), rect) in living.iter().zip(slots.iter()) {
        render_enemy(f, *rect, enemy, *index == combat.target);
    }
}

fn render_player(f: &mut Frame, area: Rect, player: &Player, combat: &Combat) {
    let mut lines = vec![
        Line::from(vec![
            Span::raw("HP "),
            Span::styled(
                meter(player.hp, player.max_hp, 6),
                Style::default().fg(health_color(player.hp, player.max_hp)),
            ),
            Span::raw(format!(" {:>3}", player.hp)),
        ]),
        Line::from(vec![
            Span::raw("MP "),
            Span::styled(
                meter(u16::from(player.mana), u16::from(player.max_mana), 6),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(format!(" {:>3}", player.mana)),
        ]),
        Line::raw(""),
        Line::styled(
            format!("Round {}", combat.round),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    for active in player.statuses.iter() {
        lines.push(Line::styled(
            format!("{} ({})", active.status.name(), active.rounds),
            Style::default().fg(status_color(active.status)),
        ));
    }

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("You")),
        area,
    );
}

fn render_enemy(f: &mut Frame, area: Rect, enemy: &Enemy, targeted: bool) {
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
            meter(enemy.hp, enemy.max_hp, 5),
            Style::default().fg(health_color(enemy.hp, enemy.max_hp)),
        ),
        Span::raw(format!(" {}", enemy.hp)),
    ]));

    lines.push(Line::styled(
        enemy.intent.label(),
        Style::default().fg(Color::LightRed),
    ));

    if enemy.armor > 0 {
        lines.push(Line::styled(
            format!("Armor {}", enemy.armor),
            Style::default().fg(Color::Gray),
        ));
    }

    for active in enemy.statuses.iter() {
        lines.push(Line::styled(
            format!("{} ({})", active.status.name(), active.rounds),
            Style::default().fg(status_color(active.status)),
        ));
    }

    let border = if targeted {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title = if targeted {
        format!("▸ {}", enemy.name)
    } else {
        format!("  {}", enemy.name)
    };

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(border),
        area,
    );
}

fn render_log(f: &mut Frame, area: Rect, app: &App) {
    let visible = app.visible_log();
    let height = area.height.saturating_sub(2) as usize;
    let start = visible.len().saturating_sub(height);

    let lines: Vec<Line> = visible[start..].iter().map(describe).collect();

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Log")),
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

fn render_build(f: &mut Frame, area: Rect, app: &App, combat: &Combat) {
    let player = &app.run.player;
    let components = combat.build_spells(player);

    let mut build_line: Vec<Span> = vec![Span::raw("Build:  ")];
    if components.is_empty() {
        build_line.push(Span::styled(
            "(empty -- press 1-5 to add a spell)",
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        for (i, spell) in components.iter().enumerate() {
            if i > 0 {
                build_line.push(Span::raw(" + "));
            }
            build_line.push(Span::styled(
                format!("[{}]", spell.name),
                Style::default().fg(element_color(spell.element)),
            ));
        }
    }

    let result_line = match combat.preview(player, &app.content.rules) {
        Some(spell) => Line::from(vec![
            Span::raw("Casting: "),
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
            Span::styled(
                if spell.pierce { "  pierces" } else { "" },
                Style::default().fg(Color::LightGreen),
            ),
            Span::styled(
                if spell.statuses.is_empty() {
                    String::new()
                } else {
                    format!(
                        "  {}",
                        spell
                            .statuses
                            .iter()
                            .map(|(s, r)| format!("{} ({r})", s.name()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
                Style::default().fg(Color::Yellow),
            ),
        ]),
        None => Line::styled("Casting: --", Style::default().fg(Color::DarkGray)),
    };

    f.render_widget(
        Paragraph::new(vec![Line::from(build_line), result_line]).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Incantation  [Enter] cast  [Bksp] undo  [Tab] end turn"),
        ),
        area,
    );
}

fn render_slots(f: &mut Frame, area: Rect, app: &App, combat: &Combat) {
    let player = &app.run.player;
    let committed = combat.committed_mana(player);
    let slots = crate::game::spell::SPELL_SLOTS;
    let widths = vec![Constraint::Ratio(1, slots as u32); slots];
    let rects = Layout::horizontal(widths).split(area);

    for (i, rect) in rects.iter().enumerate() {
        match player.spell(i) {
            Some(spell) => {
                let affordable = committed.saturating_add(spell.mana_cost) <= player.mana;
                let style = if affordable {
                    Style::default().fg(element_color(spell.element))
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                f.render_widget(
                    Paragraph::new(Line::styled(
                        format!("{} · {}MP", spell.name, spell.mana_cost),
                        style,
                    ))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("{}", i + 1)),
                    ),
                    *rect,
                );
            }
            None => {
                f.render_widget(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("{}", i + 1))
                        .style(Style::default().fg(Color::DarkGray)),
                    *rect,
                );
            }
        }
    }
}
