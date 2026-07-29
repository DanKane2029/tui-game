//! Branching map generation.
//!
//! Every generated map must satisfy two properties, both checked by tests:
//! every node is reachable from the start, and the boss is reachable from
//! every node. A map that fails either would strand the player.

// rand 0.10 moved `random_range` and friends onto RngExt.
use rand::{Rng, RngExt};

use crate::game::map::{Map, Node, NodeId, NodeKind};

/// Generate a map with `rows` rows. The bottom row offers a choice of
/// starting nodes; the top row is the single boss node.
pub fn generate(rows: usize, rng: &mut impl Rng) -> Map {
    let rows = rows.max(2);
    let mut nodes: Vec<Node> = Vec::new();
    let mut row_ids: Vec<Vec<NodeId>> = Vec::new();

    for row in 0..rows {
        // Only the boss row is a single node. The bottom row branches too, so
        // the player chooses where to begin -- and so no node ever needs more
        // than two exits to cover the row above it.
        let width = if row == rows - 1 {
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

/// Wire one row to the next.
///
/// Built so that two invariants hold by construction rather than by repair:
/// every node above has something pointing at it, and no node below ends up
/// with more than two exits. The second keeps the drawn map readable.
fn connect(nodes: &mut [Node], from: &[NodeId], to: &[NodeId], rng: &mut impl Rng) {
    let aligned_for = |i: usize| (i * to.len()) / from.len();

    // 1. Everyone gets the node roughly above them, so paths read as lines
    //    rather than as a tangle.
    for (i, &id) in from.iter().enumerate() {
        nodes[id].next = vec![to[aligned_for(i)]];
    }

    // 2. Cover anything nobody points at, using the nearest source with room.
    for (j, &target) in to.iter().enumerate() {
        if from.iter().any(|&id| nodes[id].next.contains(&target)) {
            continue;
        }
        let best = from
            .iter()
            .enumerate()
            .filter(|&(_, &id)| nodes[id].next.len() < 2)
            .min_by_key(|(i, _)| aligned_for(*i).abs_diff(j))
            .map(|(_, &id)| id);

        if let Some(source) = best {
            nodes[source].next.push(target);
            nodes[source].next.sort_unstable();
        }
    }

    // 3. A little extra branching where there is still room. Kept sparse on
    //    purpose: the more edges, the less the player's choice matters.
    for (i, &id) in from.iter().enumerate() {
        if nodes[id].next.len() >= 2 || to.len() < 2 {
            continue;
        }
        if rng.random_range(0..100) < 38 {
            // Only ever branch to an adjacent column, so edges stay short.
            let aligned = aligned_for(i);
            let neighbour = if aligned + 1 < to.len() {
                to[aligned + 1]
            } else {
                to[aligned.saturating_sub(1)]
            };
            if !nodes[id].next.contains(&neighbour) {
                nodes[id].next.push(neighbour);
                nodes[id].next.sort_unstable();
            }
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
        let mut stack: Vec<NodeId> = map.rows[0].clone();
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
    fn the_boss_row_holds_exactly_one_node_and_the_start_row_branches() {
        for map in maps() {
            assert!(
                map.rows.first().unwrap().len() >= 2,
                "the bottom row should offer a choice of where to begin"
            );
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

    /// The point of a branching map: which node you pick must actually rule
    /// some later nodes out. If every node reached every node above it, the
    /// branching would be decorative.
    #[test]
    fn choosing_a_node_genuinely_rules_others_out() {
        let mut constrained = 0;
        let mut total = 0;
        for map in maps() {
            for row in 0..map.rows.len() - 1 {
                let above = &map.rows[row + 1];
                if above.len() < 2 {
                    continue;
                }
                for &id in &map.rows[row] {
                    total += 1;
                    if map.node(id).next.len() < above.len() {
                        constrained += 1;
                    }
                }
            }
        }
        assert!(total > 0, "no branching rows were generated at all");
        let ratio = constrained as f32 / total as f32;
        assert!(
            ratio > 0.5,
            "only {:.0}% of choices constrain the next row; branching is nearly decorative",
            ratio * 100.0
        );
    }

    #[test]
    fn no_node_ever_has_more_than_two_exits() {
        // Keeps the drawn map legible.
        for map in maps() {
            for node in &map.nodes {
                assert!(
                    node.next.len() <= 2,
                    "node {} has {} exits",
                    node.id,
                    node.next.len()
                );
            }
        }
    }
}
