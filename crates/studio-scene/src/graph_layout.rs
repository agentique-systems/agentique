use crate::{
    LayoutEngine, LayoutInput, LayoutMemory, LayoutResult, Point, Rect, Size, spatial::RectIndex,
};
use agq_kernel::ElementId;
use std::collections::{BTreeMap, BTreeSet};

/// Deterministic layered topology layout. Directed cycles are compact components;
/// condensed acyclic dependencies establish ranks. Owners remain semantic DTOs.
#[derive(Clone, Copy, Debug)]
pub struct GraphLayout {
    pub node_size: Size,
    pub rank_gap: f32,
    pub node_gap: f32,
}
impl Default for GraphLayout {
    fn default() -> Self {
        Self {
            node_size: Size::new(232.0, 118.0),
            rank_gap: 108.0,
            node_gap: 42.0,
        }
    }
}
impl LayoutEngine for GraphLayout {
    fn layout(&self, input: &LayoutInput, previous: Option<&LayoutMemory>) -> LayoutResult {
        let ids: BTreeSet<_> = input.nodes.iter().map(|n| n.id).collect();
        let mut outgoing: BTreeMap<ElementId, Vec<ElementId>> =
            ids.iter().map(|id| (*id, Vec::new())).collect();
        let mut incoming = outgoing.clone();
        for (a, b) in &input.edges {
            if a != b && ids.contains(a) && ids.contains(b) {
                outgoing.get_mut(a).expect("known endpoint").push(*b);
                incoming.get_mut(b).expect("known endpoint").push(*a);
            }
        }
        for edges in outgoing.values_mut().chain(incoming.values_mut()) {
            edges.sort();
            edges.dedup();
        }
        let mut visited = BTreeSet::new();
        let mut finish = Vec::new();
        for id in &ids {
            if visited.contains(id) {
                continue;
            }
            let mut stack = vec![(*id, false)];
            while let Some((id, done)) = stack.pop() {
                if done {
                    finish.push(id);
                    continue;
                }
                if !visited.insert(id) {
                    continue;
                }
                stack.push((id, true));
                for next in outgoing[&id].iter().rev() {
                    if !visited.contains(next) {
                        stack.push((*next, false));
                    }
                }
            }
        }
        let mut component = BTreeMap::new();
        let mut components = Vec::<Vec<ElementId>>::new();
        for id in finish.into_iter().rev() {
            if component.contains_key(&id) {
                continue;
            }
            let index = components.len();
            let mut members = Vec::new();
            let mut stack = vec![id];
            while let Some(id) = stack.pop() {
                if component.contains_key(&id) {
                    continue;
                }
                component.insert(id, index);
                members.push(id);
                stack.extend(incoming[&id].iter().copied());
            }
            members.sort();
            components.push(members);
        }
        let mut successors = vec![BTreeSet::new(); components.len()];
        let mut degrees = vec![0; components.len()];
        for (a, targets) in &outgoing {
            for b in targets {
                let ca = component[a];
                let cb = component[b];
                if ca != cb && successors[ca].insert(cb) {
                    degrees[cb] += 1;
                }
            }
        }
        let mut ready: BTreeSet<_> = degrees
            .iter()
            .enumerate()
            .filter(|(_, d)| **d == 0)
            .map(|(i, _)| i)
            .collect();
        let mut ranks = vec![0; components.len()];
        while let Some(index) = ready.pop_first() {
            for next in &successors[index] {
                ranks[*next] = ranks[*next].max(ranks[index] + 1);
                degrees[*next] -= 1;
                if degrees[*next] == 0 {
                    ready.insert(*next);
                }
            }
        }
        let mut by_rank: BTreeMap<usize, Vec<ElementId>> = BTreeMap::new();
        for (i, members) in components.iter().enumerate() {
            by_rank.entry(ranks[i]).or_default().extend(members);
        }
        for members in by_rank.values_mut() {
            members.sort();
        }
        // Two deterministic barycenter sweeps reduce avoidable crossings in
        // small engineering graphs. Large cyclic components keep their stable
        // identity order; topology edits never override restored positions.
        for adjacency in [&outgoing, &incoming] {
            let order: BTreeMap<_, _> = by_rank
                .values()
                .flat_map(|members| members.iter().enumerate().map(|(i, id)| (*id, i as f32)))
                .collect();
            for members in by_rank.values_mut().filter(|members| members.len() <= 64) {
                let score = |id: &ElementId| {
                    let neighbors: Vec<_> = adjacency[id]
                        .iter()
                        .filter(|other| ranks[component[other]] != ranks[component[id]])
                        .collect();
                    if neighbors.is_empty() {
                        order[id]
                    } else {
                        neighbors.iter().map(|id| order[id]).sum::<f32>() / neighbors.len() as f32
                    }
                };
                members.sort_by(|a, b| score(a).total_cmp(&score(b)).then_with(|| a.cmp(b)));
            }
        }
        let mut result = LayoutResult::default();
        let mut occupied = RectIndex::new(320.0);
        if let Some(previous) = previous {
            for id in &ids {
                if let Some(old) = previous.bounds.get(id).filter(|r| r.finite()) {
                    let r = Rect::new(
                        old.min.x,
                        old.min.y,
                        self.node_size.width,
                        self.node_size.height,
                    );
                    if occupied.query(r.inflate(14.0)).is_empty() {
                        occupied.insert(r, *id);
                        result.bounds.insert(*id, r);
                    }
                }
            }
        }
        let mut x = 0.0;
        for members in by_rank.values_mut() {
            // Dense cycles form a compact field instead of an unbounded column.
            let columns = if members.len() > 16 {
                (members.len() as f32).sqrt().ceil() as usize
            } else if members.len() > 4 {
                2
            } else {
                1
            };
            let mut slot = 0;
            for id in members.iter() {
                if result.bounds.contains_key(id) {
                    continue;
                }
                loop {
                    let position = Point::new(
                        x + (slot % columns) as f32 * (self.node_size.width + self.node_gap),
                        (slot / columns) as f32 * (self.node_size.height + self.node_gap),
                    );
                    slot += 1;
                    let r = Rect::new(
                        position.x,
                        position.y,
                        self.node_size.width,
                        self.node_size.height,
                    );
                    if occupied.query(r.inflate(14.0)).is_empty() {
                        occupied.insert(r, *id);
                        result.bounds.insert(*id, r);
                        break;
                    }
                }
            }
            x += columns as f32 * (self.node_size.width + self.node_gap) + self.rank_gap;
        }
        result
    }
}
