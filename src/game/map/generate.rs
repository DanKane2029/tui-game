//! Branching map generation.
//!
//! Every generated map must satisfy two properties, both checked by tests:
//! every node is reachable from the start, and the boss is reachable from
//! every node. A map that fails either would strand the player.

// rand 0.10 moved `random_range` and friends onto RngExt.
use rand::{Rng, RngExt};

use crate::game::map::{Map, Node, NodeId, NodeKind};

/// Generate a map with `rows` rows. Row 0 is the single starting node and the
/// top row is the single boss node.
pub fn generate(rows: usize, rng: &mut impl Rng) -> Map {
    let rows = rows.max(2);
    let mut nodes: Vec<Node> = Vec::new();
    let mut row_ids: Vec<Vec<NodeId>> = Vec::new();

    for row in 0..rows {
        let width = if row == 0 || row == rows - 1 {
            1
        } else {
            rng.random_range(2..=3)
        };

        let mut ids = Vec::with_capacity(width);
        for col in 0..width {
            let kind = if row == rows - 1 {
                NodeKind::Boss
            } else if row == 0 {
                NodeKind::Fight
            } else {
                // Weighted so fights stay the backbone of a run.
                match rng.random_range(0..100) {
                    0..22 => NodeKind::Event,
                    22..34 => NodeKind::Shop,
                    _ => NodeKind::Fight,
                }
            };

            let id = nodes.len();
            nodes.push(Node {
                id,
                kind,
                row,
                col,
                next: Vec::new(),
            });
            ids.push(id);
        }
        row_ids.push(ids);
    }

    for row in 0..rows - 1 {
        connect(&mut nodes, &row_ids[row], &row_ids[row + 1], rng);
    }

    Map {
        nodes,
        rows: row_ids,
    }
}

/// Wire one row to the next, guaranteeing both directions of connectivity:
/// every node here gets an outgoing edge, and every node above gets an
/// incoming one.
fn connect(nodes: &mut [Node], from: &[NodeId], to: &[NodeId], rng: &mut impl Rng) {
    for (i, &id) in from.iter().enumerate() {
        // Bias toward the node directly above, so paths read as lines rather
        // than as a tangle.
        let aligned = (i * to.len()) / from.len();
        let mut targets = vec![to[aligned]];

        if to.len() > 1 && rng.random_range(0..100) < 55 {
            let other = to[rng.random_range(0..to.len())];
            if !targets.contains(&other) {
                targets.push(other);
            }
        }

        targets.sort_unstable();
        nodes[id].next = targets;
    }

    // Any node above with nothing pointing at it gets an edge from a random
    // node below. Without this the map could strand a branch.
    for &target in to {
        let has_incoming = from.iter().any(|&id| nodes[id].next.contains(&target));
        if !has_incoming {
            let source = from[rng.random_range(0..from.len())];
            nodes[source].next.push(target);
            nodes[source].next.sort_unstable();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashSet;

    fn maps() -> impl Iterator<Item = Map> {
        (0..200u64).map(|seed| generate(5, &mut StdRng::seed_from_u64(seed)))
    }

    fn reachable_from_start(map: &Map) -> HashSet<NodeId> {
        let mut seen = HashSet::new();
        let mut stack = vec![map.start()];
        while let Some(id) = stack.pop() {
            if seen.insert(id) {
                stack.extend(map.node(id).next.iter().copied());
            }
        }
        seen
    }

    #[test]
    fn every_node_is_reachable_from_the_start() {
        for map in maps() {
            let seen = reachable_from_start(&map);
            assert_eq!(
                seen.len(),
                map.nodes.len(),
                "map has {} unreachable node(s)",
                map.nodes.len() - seen.len()
            );
        }
    }

    #[test]
    fn the_boss_is_reachable_from_every_node() {
        for map in maps() {
            let boss = map.boss();
            for node in &map.nodes {
                let mut seen = HashSet::new();
                let mut stack = vec![node.id];
                let mut found = false;
                while let Some(id) = stack.pop() {
                    if id == boss {
                        found = true;
                        break;
                    }
                    if seen.insert(id) {
                        stack.extend(map.node(id).next.iter().copied());
                    }
                }
                assert!(found, "node {} cannot reach the boss", node.id);
            }
        }
    }

    #[test]
    fn edges_only_ever_go_one_row_up() {
        for map in maps() {
            for node in &map.nodes {
                for &next in &node.next {
                    assert_eq!(
                        map.node(next).row,
                        node.row + 1,
                        "edge from row {} skipped to row {}",
                        node.row,
                        map.node(next).row
                    );
                }
            }
        }
    }

    #[test]
    fn the_start_and_boss_rows_hold_exactly_one_node() {
        for map in maps() {
            assert_eq!(map.rows.first().unwrap().len(), 1);
            assert_eq!(map.rows.last().unwrap().len(), 1);
            assert_eq!(map.node(map.boss()).kind, NodeKind::Boss);
        }
    }

    #[test]
    fn only_the_top_row_holds_a_boss() {
        for map in maps() {
            for node in &map.nodes {
                if node.kind == NodeKind::Boss {
                    assert_eq!(node.row, map.row_count() - 1);
                }
            }
        }
    }

    #[test]
    fn generation_is_reproducible_from_a_seed() {
        let a = generate(5, &mut StdRng::seed_from_u64(42));
        let b = generate(5, &mut StdRng::seed_from_u64(42));
        assert_eq!(a.nodes.len(), b.nodes.len());
        for (x, y) in a.nodes.iter().zip(b.nodes.iter()) {
            assert_eq!(x.kind, y.kind);
            assert_eq!(x.next, y.next);
        }
    }

    #[test]
    fn a_degenerate_row_count_still_produces_a_valid_map() {
        let map = generate(0, &mut StdRng::seed_from_u64(1));
        assert!(map.row_count() >= 2);
        assert_eq!(reachable_from_start(&map).len(), map.nodes.len());
    }
}
