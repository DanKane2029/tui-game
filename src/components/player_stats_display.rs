use crate::components::Component;

use ratatui::{
    text::Line,
    widgets::{List, ListDirection},
};

pub struct PlayerStatsDisplay;

impl Component for PlayerStatsDisplay {
    fn init(&mut self) {}

    async fn handle_events(&mut self) {}

    fn update(&mut self) {}

    fn render(&mut self, f: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {
        f.render_widget(
            List::default()
                .direction(ListDirection::TopToBottom)
                .items([
                    Line::from("TEST 1"),
                    Line::from("TEST 2"),
                    Line::from("TEST 3"),
                ]),
            rect,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    /// Renders a component into an in-memory buffer and snapshots the exact characters
    /// drawn. This is how layout gets verified without launching the game.
    #[test]
    fn renders_stats_list() {
        let mut terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
        let mut display = PlayerStatsDisplay;

        terminal
            .draw(|f| display.render(f, f.area()))
            .expect("draw should succeed");

        insta::assert_snapshot!(terminal.backend());
    }
}
