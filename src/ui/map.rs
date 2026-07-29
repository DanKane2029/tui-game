//! The map screen.
//!
//! Nodes and the paths between them are drawn onto a character canvas, then
//! converted to styled lines. Drawing the edges is what makes the branching
//! legible: without them the player cannot see that picking one node rules
//! others out.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::game::map::{Map, NodeId};
use crate::ui::widgets::{health_color, meter};

/// Rows of canvas per map row: one for the node, the rest for the path
/// climbing to the row above.
const ROW_HEIGHT: usize = 3;

/// Cap on how wide the map is drawn. Spreading a few nodes across a full
/// terminal makes the connecting paths long, shallow and hard to follow.
const MAX_MAP_WIDTH: usize = 48;

/// How a cell should be coloured. Ordered by priority -- a cell claimed by a
/// higher variant is never overwritten by a lower one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Ink {
    Faint,
    Travelled,
    Open,
    Chosen,
}

impl Ink {
    fn style(self) -> Style {
        match self {
            Ink::Faint => Style::default().fg(Color::DarkGray),
            Ink::Travelled => Style::default().fg(Color::Green),
            Ink::Open => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            Ink::Chosen => Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        }
    }
}

struct Canvas {
    cells: Vec<Vec<(char, Ink)>>,
    width: usize,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![(' ', Ink::Faint); width]; height],
            width,
        }
    }

    /// Higher-priority ink wins, so a travelled path is never scribbled over
    /// by a faint one drawn later.
    fn put(&mut self, x: isize, y: isize, ch: char, ink: Ink) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.cells.len() {
            return;
        }
        let cell = &mut self.cells[y as usize][x as usize];
        if cell.0 == ' ' || ink >= cell.1 {
            *cell = (ch, ink);
        }
    }

    /// Always wins. Used for the nodes themselves, which must sit on top of
    /// any path that runs through their cell.
    fn put_over(&mut self, x: isize, y: isize, ch: char, ink: Ink) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.cells.len() {
            return;
        }
        self.cells[y as usize][x as usize] = (ch, ink);
    }

    fn into_lines(self) -> Vec<Line<'static>> {
        self.cells
            .into_iter()
            .map(|row| {
                // Merge runs of identical ink into single spans.
                let mut spans: Vec<Span> = Vec::new();
                let mut buffer = String::new();
                let mut current: Option<Ink> = None;

                for (ch, ink) in row {
                    if current != Some(ink) {
                        if let Some(prev) = current {
                            spans.push(Span::styled(std::mem::take(&mut buffer), prev.style()));
                        }
                        current = Some(ink);
                    }
                    buffer.push(ch);
                }
                if let Some(prev) = current {
                    spans.push(Span::styled(buffer, prev.style()));
                }
                Line::from(spans)
            })
            .collect()
    }
}

/// Where a node sits on the canvas. Rows are drawn top-down with the boss
/// first, so the map reads as a climb.
fn node_position(map: &Map, id: NodeId, width: usize) -> (isize, isize) {
    let node = map.node(id);
    let row_width = map.rows[node.row].len().max(1);
    let x = ((node.col as f32 + 0.5) / row_width as f32 * width as f32).round() as isize;
    let y = ((map.row_count() - 1 - node.row) * ROW_HEIGHT) as isize;
    (x, y)
}

/// Draw the path climbing from `from` up to `to`, excluding the node rows.
///
/// Steps along whichever axis is longer, so a wide, shallow edge stays a
/// continuous line instead of a few disconnected marks.
fn draw_path(canvas: &mut Canvas, from: (isize, isize), to: (isize, isize), ink: Ink) {
    let (x1, y1) = from;
    let (x2, y2) = to;
    let dx = x2 - x1;
    let dy = y2 - y1;
    let steps = dx.abs().max(dy.abs()).max(1);

    let diagonal = match dx.signum() {
        1 => '╱',
        -1 => '╲',
        _ => '│',
    };

    // A shallow edge spends several steps on one row. Drawing those as
    // horizontals rather than repeated diagonals is what makes it read as a
    // path instead of a smear.
    let mut previous_y = y1;
    for step in 1..steps {
        let t = step as f32 / steps as f32;
        let x = (x1 as f32 + dx as f32 * t).round() as isize;
        let y = (y1 as f32 + dy as f32 * t).round() as isize;
        let ch = if y == previous_y && dx.abs() > dy.abs() {
            '─'
        } else {
            diagonal
        };
        canvas.put(x, y, ch, ink);
        previous_y = y;
    }
}

pub fn render(f: &mut Frame, app: &App) {
    let [map_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(4)]).areas(crate::ui::stage(f));

    let available = app.nodes_available();
    let selected = available.get(app.ui.map_cursor).copied();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(match selected {
            Some(id) => format!("The Climb  ▸ {}", app.run.map.node(id).kind.label()),
            None => "The Climb".to_string(),
        });
    let inner = block.inner(map_area);
    f.render_widget(block, map_area);

    render_canvas(f, inner, app, &available, selected);
    render_status(f, status_area, app);
}

fn render_canvas(
    f: &mut Frame,
    area: Rect,
    app: &App,
    available: &[NodeId],
    selected: Option<NodeId>,
) {
    let map = &app.run.map;
    let width = (area.width.saturating_sub(1) as usize).clamp(1, MAX_MAP_WIDTH);
    let height = map.row_count() * ROW_HEIGHT;
    let mut canvas = Canvas::new(width, height);

    // Edges first, so node glyphs always sit on top of them.
    for node in &map.nodes {
        let from = node_position(map, node.id, width);
        for &next in &node.next {
            let to = node_position(map, next, width);

            // An edge is lit if it is the step the player is about to take, or
            // one they already took.
            let ink = if selected == Some(next) && app.run.position == Some(node.id) {
                Ink::Chosen
            } else if available.contains(&next) && app.run.position == Some(node.id) {
                Ink::Open
            } else if app.run.visited.contains(&node.id) && app.run.visited.contains(&next) {
                Ink::Travelled
            } else {
                Ink::Faint
            };
            draw_path(&mut canvas, from, to, ink);
        }
    }

    for node in &map.nodes {
        let (x, y) = node_position(map, node.id, width);
        let ink = if selected == Some(node.id) {
            Ink::Chosen
        } else if available.contains(&node.id) {
            Ink::Open
        } else if app.run.visited.contains(&node.id) {
            Ink::Travelled
        } else {
            Ink::Faint
        };

        // Bracket the node under the cursor so it reads even without colour.
        if selected == Some(node.id) {
            canvas.put_over(x - 1, y, '[', ink);
            canvas.put_over(x + 1, y, ']', ink);
        } else if app.run.position == Some(node.id) {
            canvas.put_over(x - 1, y, '(', Ink::Travelled);
            canvas.put_over(x + 1, y, ')', Ink::Travelled);
        }
        canvas.put_over(x, y, node.kind.glyph(), ink);
    }

    // Centre the drawing in the panel rather than pinning it to the corner.
    let draw_area = Rect {
        x: area.x + (area.width.saturating_sub(width as u16)) / 2,
        y: area.y + (area.height.saturating_sub(height as u16)) / 2,
        width: (width as u16).min(area.width),
        height: (height as u16).min(area.height),
    };
    f.render_widget(Paragraph::new(canvas.into_lines()), draw_area);
}

fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let player = &app.run.player;
    let status = vec![
        Line::from(vec![
            Span::raw("HP "),
            Span::styled(
                meter(player.hp, player.max_hp, 10),
                Style::default().fg(health_color(player.hp, player.max_hp)),
            ),
            Span::raw(format!(" {}/{}   ", player.hp, player.max_hp)),
            Span::raw("MP "),
            Span::styled(
                meter(u16::from(player.mana), u16::from(player.max_mana), 6),
                Style::default().fg(Color::Blue),
            ),
            Span::raw(format!(" {}/{}   ", player.mana, player.max_mana)),
            Span::styled(
                format!("{} gold", player.gold),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("⚔ fight   ", Style::default().fg(Color::Gray)),
            Span::styled("? event   ", Style::default().fg(Color::Gray)),
            Span::styled("$ shop   ", Style::default().fg(Color::Gray)),
            Span::styled("☠ boss      ", Style::default().fg(Color::Gray)),
            Span::styled(
                "←→ choose   Enter to travel",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(status).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Spellbook")
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        area,
    );
}
