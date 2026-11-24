//! Logical source coverage for one immutable formatting input (CSS Break 3 §5.1).
use elidex_ecs::Entity;
use std::{collections::HashMap, ops::Range};

/// A source byte boundary, optionally inside the expansion of that scalar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct SourcePosition {
    pub byte: usize,
    pub expansion: usize,
}

impl SourcePosition {
    #[must_use]
    pub fn byte(byte: usize) -> Self {
        Self { byte, expansion: 0 }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ItemCoverage {
    ranges: Vec<Range<SourcePosition>>,
    terminal: bool,
}

/// Intervals already consumed by a formatting owner, keyed by source item. The
/// terminal marker also represents completion of an empty item.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceProgress {
    items: HashMap<(Entity, Entity), ItemCoverage>,
}

impl SourceProgress {
    pub fn consume(&mut self, owner: Entity, item: Entity, range: Range<SourcePosition>) {
        if range.is_empty() {
            return;
        }
        let ranges = &mut self.items.entry((owner, item)).or_default().ranges;
        let first = ranges.partition_point(|stored| stored.end < range.start);
        let end = ranges.partition_point(|stored| stored.start <= range.end);
        let mut merged = range;
        if first < end {
            merged.start = merged.start.min(ranges[first].start);
            merged.end = merged.end.max(ranges[end - 1].end);
        }
        ranges.splice(first..end, [merged]);
    }

    #[must_use]
    pub fn covers(&self, owner: Entity, item: Entity, range: Range<SourcePosition>) -> bool {
        range.is_empty()
            || self.items.get(&(owner, item)).is_some_and(|coverage| {
                let i = coverage.ranges.partition_point(|r| r.end <= range.start);
                coverage
                    .ranges
                    .get(i)
                    .is_some_and(|r| r.start <= range.start && range.end <= r.end)
            })
    }
    #[must_use]
    pub fn contains(&self, owner: Entity, item: Entity, position: SourcePosition) -> bool {
        self.items.get(&(owner, item)).is_some_and(|coverage| {
            let i = coverage.ranges.partition_point(|r| r.end <= position);
            coverage
                .ranges
                .get(i)
                .is_some_and(|r| r.contains(&position))
        })
    }

    pub fn finish_item(&mut self, owner: Entity, item: Entity) {
        self.items.entry((owner, item)).or_default().terminal = true;
    }
    #[must_use]
    pub fn is_finished(&self, owner: Entity, item: Entity) -> bool {
        self.items
            .get(&(owner, item))
            .is_some_and(|coverage| coverage.terminal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_ecs::EcsDom;

    #[test]
    fn disjoint_ranges_merge_without_finishing_sibling_or_other_owner() {
        let mut dom = EcsDom::new();
        let a = dom.create_text_ownerless_fixture("abcdef");
        let b = dom.create_text_ownerless_fixture("");
        let mut progress = SourceProgress::default();
        let p = SourcePosition::byte;
        progress.consume(a, a, p(4)..p(6));
        progress.finish_item(a, b);
        progress.consume(a, a, p(0)..p(2));
        assert!(!progress.covers(a, a, p(0)..p(6)));
        progress.consume(a, a, p(2)..p(4));
        assert!(progress.covers(a, a, p(0)..p(6)));
        assert!(!progress.is_finished(a, a));
        assert!(progress.is_finished(a, b));
        assert!(!progress.is_finished(b, b));
        assert!(!progress.contains(b, a, p(0)));
    }
}
