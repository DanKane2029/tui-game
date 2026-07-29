//! Frame snapshots.
//!
//! `ui::render` takes `&App`, so a test can construct any game state it likes
//! and assert on the exact characters drawn -- no terminal, no PTY, no runtime.
//! This is how layout gets iterated on without launching the game.

use insta::assert_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use tui_game::action::Action;
use tui_game::app::App;
use tui_game::game::content::Content;
use tui_game::ui;

fn app(seed: u64) -> App {
    let mut app = App::new(Content::load().expect("content parses"), seed);
    app.start_run_with_seed(seed);
    app
}

/// An app sitting on the title screen, before any run has begun.
fn fresh() -> App {
    App::new(Content::load().expect("content parses"), 1)
}

fn frame(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test backend");
    terminal
        .draw(|f| ui::render(f, app))
        .expect("render succeeds");
    format!("{}", terminal.backend())
}

/// Reveal the whole combat log so snapshots do not depend on pacing.
fn settle(app: &mut App) {
    for _ in 0..64 {
        app.tick();
    }
}

#[test]
fn map_screen() {
    assert_snapshot!(frame(&app(7)));
}

#[test]
fn combat_screen_at_the_start_of_a_fight() {
    let mut app = app(7);
    app.apply(Action::Confirm);
    settle(&mut app);
    assert_snapshot!(frame(&app));
}

/// Two Embers stack into Fireball. The preview updates as the bag grows.
#[test]
fn incantation_preview_two_embers() {
    let mut app = app(7);
    app.apply(Action::Confirm);
    app.apply(Action::AddComponent(0));
    app.apply(Action::AddComponent(0));
    settle(&mut app);
    assert_snapshot!(frame(&app));
}

/// Adding Gust converts it to hit everything, at reduced power. This is the
/// headline example from DESIGN.md, rendered.
#[test]
fn incantation_preview_becomes_firestorm() {
    let mut app = app(7);
    app.apply(Action::Confirm);
    app.apply(Action::AddComponent(0));
    app.apply(Action::AddComponent(0));
    app.apply(Action::AddComponent(2));
    settle(&mut app);
    assert_snapshot!(frame(&app));
}

/// Cast the arrow-driven way: build on the spell row, move up to the action
/// row, confirm.
#[test]
fn combat_after_casting() {
    let mut app = app(7);
    app.apply(Action::Confirm);
    app.apply(Action::AddComponent(0));
    app.apply(Action::AddComponent(0));
    app.apply(Action::NavUp);
    app.apply(Action::Confirm);
    settle(&mut app);
    assert_snapshot!(frame(&app));
}

/// Focus on the enemy row, where the arrows pick a target.
#[test]
fn combat_with_the_enemy_row_focused() {
    let mut app = app(7);
    app.apply(Action::Confirm);
    app.apply(Action::NavUp);
    app.apply(Action::NavUp);
    settle(&mut app);
    assert_snapshot!(frame(&app));
}

#[test]
fn game_over_screen() {
    let mut app = app(7);
    app.apply(Action::Confirm);
    app.run.player.hp = 1;
    for _ in 0..8 {
        app.apply(Action::EndTurn);
    }
    settle(&mut app);
    assert_snapshot!(frame(&app));
}

/// Clear the current fight so the reward screen appears.
fn win_fight(app: &mut App) {
    use tui_game::app::{ACTION_CAST, Focus};
    let count = app.combat.as_ref().expect("in a fight").enemies.len();
    for enemy in &mut app.combat.as_mut().unwrap().enemies {
        enemy.hp = 1;
    }
    for i in 0..count {
        if app.combat.is_none() {
            break;
        }
        app.combat.as_mut().unwrap().target = i;
        app.run.player.mana = 99;
        app.apply(Action::AddComponent(0));
        app.ui.focus = Focus::Actions;
        app.ui.action_cursor = ACTION_CAST;
        app.apply(Action::Confirm);
    }
}

#[test]
fn reward_screen() {
    let mut app = app(7);
    app.apply(Action::Confirm);
    win_fight(&mut app);
    settle(&mut app);
    assert_snapshot!(frame(&app));
}

#[test]
fn title_screen() {
    assert_snapshot!(frame(&fresh()));
}

#[test]
fn options_screen() {
    let mut app = fresh();
    app.ui.title_cursor = 1;
    app.apply(Action::Confirm);
    assert_snapshot!(frame(&app));
}

#[test]
fn shop_screen() {
    use tui_game::game::shop;
    let mut app = app(7);
    app.run.player.gold = 80;
    let stock = shop::generate(
        &app.content.spells,
        &app.run.player.spells,
        1,
        &mut app.run.rng,
    );
    app.shop = Some(stock);
    app.screen = tui_game::app::Screen::Shop;
    assert_snapshot!(frame(&app));
}

/// The modal shown when a spell arrives with no free slot, from either the
/// reward screen or the shop.
#[test]
fn replacement_screen() {
    use tui_game::game::spell::SPELL_SLOTS;
    let mut app = app(7);
    app.apply(Action::Confirm);
    win_fight(&mut app);
    let filler = app.run.player.spells[0].clone();
    while app.run.player.spells.len() < SPELL_SLOTS {
        app.run.player.spells.push(filler.clone());
    }
    app.apply(Action::Confirm);
    assert_snapshot!(frame(&app));
}

/// Part-way up: the travelled path and the still-open branches are drawn
/// differently from the rest of the map.
#[test]
fn map_screen_part_way_through_a_run() {
    let mut app = app(7);
    app.apply(Action::Confirm);
    win_fight(&mut app);
    // Take the reward's Skip so we land back on the map.
    let skip = app.reward.as_ref().unwrap().skip_index();
    app.reward.as_mut().unwrap().cursor = skip;
    app.apply(Action::Confirm);
    assert_snapshot!(frame(&app));
}
