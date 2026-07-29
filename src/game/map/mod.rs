//! The run map: a branching directed graph of nodes, climbed bottom to top.

pub mod generate;

pub use generate::generate;

pub type NodeId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Fight,
    Event,
    Shop,
    Boss,
}

impl NodeKind {
    /// Single-character map glyph.
    pub fn glyph(self) -> char {
        match self {
            NodeKind::Fight => '⚔',
            NodeKind::Event => '?',
            NodeKind::Shop => '$',
            NodeKind::Boss => '☠',
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            NodeKind::Fight => "Fight",
            NodeKind::Event => "Event",
            NodeKind::Shop => "Shop",
            NodeKind::Boss => "Boss",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub row: usize,
    /// Position within its row, used only for drawing.
    pub col: usize,
    /// Nodes reachable from here, all in the next row up.
    pub next: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct Map {
    pub nodes: Vec<Node>,
    /// Node ids grouped by row, bottom row first.
    pub rows: Vec<Vec<NodeId>>,
}

impl Map {
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn start(&self) -> NodeId {
        self.rows[0][0]
    }

    pub fn boss(&self) -> NodeId {
        *self
            .rows
            .last()
            .expect("map has rows")
            .first()
            .expect("row has a node")
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Difficulty budget for encounters at this depth.
    pub fn budget_for_row(&self, row: usize) -> u8 {
        (2 + row * 2).min(u8::MAX as usize) as u8
    }
}
