//! Small reusable rendering pieces.

use ratatui::style::Color;

use crate::game::element::Element;
use crate::game::status::Status;

/// A block meter, e.g. `███░░`.
pub fn meter(current: u16, max: u16, width: usize) -> String {
    if max == 0 || width == 0 {
        return " ".repeat(width);
    }
    let filled = ((f32::from(current) / f32::from(max)) * width as f32).round() as usize;
    let filled = filled.min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// Colour is presentation, so it lives here rather than on the pure `Element`.
pub fn element_color(element: Element) -> Color {
    match element {
        Element::Flame => Color::Red,
        Element::Water => Color::Blue,
        Element::Ice => Color::Cyan,
        Element::Shock => Color::Yellow,
        Element::Earth => Color::LightGreen,
        Element::Gust => Color::White,
        Element::Toxic => Color::Magenta,
        Element::Steam => Color::Gray,
        Element::Magma => Color::LightRed,
        Element::Blight => Color::LightMagenta,
        Element::Blizzard => Color::LightCyan,
    }
}

pub fn status_color(status: Status) -> Color {
    match status {
        Status::Burned => Color::Red,
        Status::Wet => Color::Blue,
        Status::Frozen => Color::Cyan,
        Status::Blind => Color::Gray,
        Status::Poisoned => Color::Magenta,
    }
}

/// Green when healthy, yellow when hurt, red when nearly dead.
pub fn health_color(current: u16, max: u16) -> Color {
    if max == 0 {
        return Color::DarkGray;
    }
    let frac = f32::from(current) / f32::from(max);
    if frac > 0.5 {
        Color::Green
    } else if frac > 0.25 {
        Color::Yellow
    } else {
        Color::Red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_meter_is_all_filled_and_an_empty_one_is_not() {
        assert_eq!(meter(10, 10, 4), "████");
        assert_eq!(meter(0, 10, 4), "░░░░");
    }

    #[test]
    fn a_meter_is_always_exactly_the_requested_width() {
        for current in 0..=20u16 {
            assert_eq!(meter(current, 10, 6).chars().count(), 6);
        }
    }

    #[test]
    fn a_zero_max_does_not_divide_by_zero() {
        assert_eq!(meter(0, 0, 3).chars().count(), 3);
    }

    #[test]
    fn health_colour_degrades_as_health_drops() {
        assert_eq!(health_color(10, 10), Color::Green);
        assert_eq!(health_color(4, 10), Color::Yellow);
        assert_eq!(health_color(1, 10), Color::Red);
    }
}
