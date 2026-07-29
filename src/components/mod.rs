mod app;
mod fight_display;
mod player_stats_display;
mod spell_select;

pub use app::App;
pub use fight_display::FightDisplay;
pub use spell_select::SpellSelect;

use ratatui::{Frame, layout::Rect};

pub trait Component {
    fn init(&mut self);
    async fn handle_events(&mut self);
    fn update(&mut self);
    fn render(&mut self, f: &mut Frame, rect: Rect);
}
