//! Run options, set from the title screen.
//!
//! Every option is a small closed set cycled with left/right, so the options
//! screen needs no text entry and the whole game stays on the arrow keys.

/// A setting that cycles through a fixed list of values.
pub trait Cycle: Sized + Copy + PartialEq + 'static {
    const ALL: &'static [Self];

    fn label(self) -> &'static str;

    fn index(self) -> usize {
        Self::ALL.iter().position(|v| *v == self).unwrap_or(0)
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapLength {
    Short,
    Standard,
    Long,
}

impl MapLength {
    pub fn rows(self) -> usize {
        match self {
            MapLength::Short => 3,
            MapLength::Standard => 5,
            MapLength::Long => 8,
        }
    }
}

impl Cycle for MapLength {
    const ALL: &'static [Self] = &[MapLength::Short, MapLength::Standard, MapLength::Long];

    fn label(self) -> &'static str {
        match self {
            MapLength::Short => "Short (3 rows)",
            MapLength::Standard => "Standard (5 rows)",
            MapLength::Long => "Long (8 rows)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Calm,
    Standard,
    Harsh,
}

impl Difficulty {
    /// Scales the difficulty budget an encounter is built against.
    pub fn budget_scale(self) -> f32 {
        match self {
            Difficulty::Calm => 0.7,
            Difficulty::Standard => 1.0,
            Difficulty::Harsh => 1.5,
        }
    }
}

impl Cycle for Difficulty {
    const ALL: &'static [Self] = &[Difficulty::Calm, Difficulty::Standard, Difficulty::Harsh];

    fn label(self) -> &'static str {
        match self {
            Difficulty::Calm => "Calm",
            Difficulty::Standard => "Standard",
            Difficulty::Harsh => "Harsh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSpeed {
    Instant,
    Normal,
    Slow,
}

impl LogSpeed {
    /// Loop ticks between revealing one more log entry. Zero means reveal the
    /// whole thing immediately.
    pub fn ticks_per_entry(self) -> u8 {
        match self {
            LogSpeed::Instant => 0,
            LogSpeed::Normal => 1,
            LogSpeed::Slow => 3,
        }
    }
}

impl Cycle for LogSpeed {
    const ALL: &'static [Self] = &[LogSpeed::Instant, LogSpeed::Normal, LogSpeed::Slow];

    fn label(self) -> &'static str {
        match self {
            LogSpeed::Instant => "Instant",
            LogSpeed::Normal => "Normal",
            LogSpeed::Slow => "Slow",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub map_length: MapLength,
    pub difficulty: Difficulty,
    pub log_speed: LogSpeed,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            map_length: MapLength::Standard,
            difficulty: Difficulty::Standard,
            log_speed: LogSpeed::Normal,
        }
    }
}

/// Which row of the options screen is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionField {
    MapLength,
    Difficulty,
    LogSpeed,
}

impl Cycle for OptionField {
    const ALL: &'static [Self] = &[
        OptionField::MapLength,
        OptionField::Difficulty,
        OptionField::LogSpeed,
    ];

    fn label(self) -> &'static str {
        match self {
            OptionField::MapLength => "Map length",
            OptionField::Difficulty => "Difficulty",
            OptionField::LogSpeed => "Log speed",
        }
    }
}

impl Options {
    /// Cycle one field forwards or backwards.
    pub fn adjust(&mut self, field: OptionField, forward: bool) {
        match field {
            OptionField::MapLength => {
                self.map_length = if forward {
                    self.map_length.next()
                } else {
                    self.map_length.prev()
                }
            }
            OptionField::Difficulty => {
                self.difficulty = if forward {
                    self.difficulty.next()
                } else {
                    self.difficulty.prev()
                }
            }
            OptionField::LogSpeed => {
                self.log_speed = if forward {
                    self.log_speed.next()
                } else {
                    self.log_speed.prev()
                }
            }
        }
    }

    pub fn value_label(&self, field: OptionField) -> &'static str {
        match field {
            OptionField::MapLength => self.map_length.label(),
            OptionField::Difficulty => self.difficulty.label(),
            OptionField::LogSpeed => self.log_speed.label(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycling_forwards_then_back_returns_to_the_start() {
        for value in MapLength::ALL {
            assert_eq!(value.next().prev(), *value);
        }
        for value in Difficulty::ALL {
            assert_eq!(value.next().prev(), *value);
        }
        for value in LogSpeed::ALL {
            assert_eq!(value.next().prev(), *value);
        }
    }

    #[test]
    fn cycling_through_every_value_wraps_to_where_it_began() {
        let mut value = Difficulty::Calm;
        for _ in 0..Difficulty::ALL.len() {
            value = value.next();
        }
        assert_eq!(value, Difficulty::Calm);
    }

    #[test]
    fn every_value_has_a_label_and_a_stable_index() {
        for (i, value) in MapLength::ALL.iter().enumerate() {
            assert!(!value.label().is_empty());
            assert_eq!(value.index(), i);
        }
    }

    #[test]
    fn adjusting_a_field_changes_only_that_field() {
        let mut options = Options::default();
        let before = options;
        options.adjust(OptionField::Difficulty, true);
        assert_ne!(options.difficulty, before.difficulty);
        assert_eq!(options.map_length, before.map_length);
        assert_eq!(options.log_speed, before.log_speed);
    }

    #[test]
    fn map_lengths_are_ordered_and_all_playable() {
        assert!(MapLength::Short.rows() < MapLength::Standard.rows());
        assert!(MapLength::Standard.rows() < MapLength::Long.rows());
        // Generation needs at least a start row and a boss row.
        for value in MapLength::ALL {
            assert!(value.rows() >= 2);
        }
    }

    #[test]
    fn difficulty_scales_in_the_direction_you_would_expect() {
        assert!(Difficulty::Calm.budget_scale() < Difficulty::Standard.budget_scale());
        assert!(Difficulty::Standard.budget_scale() < Difficulty::Harsh.budget_scale());
    }
}
