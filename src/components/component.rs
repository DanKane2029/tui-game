use ratatui::{Frame, layout::Rect};

pub trait Component {
	fn init(&mut self);
	fn handle_events(&mut self);
	fn handle_key_events(&mut self);
	fn handle_mouse_events(&mut self);
	fn update(&mut self);
	fn render(&mut self, f: &mut Frame, rect: Rect);
}
