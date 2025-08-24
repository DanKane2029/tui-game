mod app;
mod spell_select;
mod fight_display;
mod player_stats_display;

pub use app::App;
pub use spell_select::SpellSelect;
pub use fight_display::FightDisplay;

use ratatui::{Frame, layout::Rect};

pub trait Component {
	fn init(&mut self);
	async fn handle_events(&mut self);
	fn update(&mut self);
	fn render(&mut self, f: &mut Frame, rect: Rect);
}

pub enum Components {
	App(App),
	SpellSelect(SpellSelect)
}
