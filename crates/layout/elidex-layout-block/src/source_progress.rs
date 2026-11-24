//! Logical source coverage for one immutable formatting input (CSS Break 3 §5.1).
use elidex_ecs::Entity;
use std::{collections::HashMap, ops::Range, sync::Arc};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceProgress {
    page: Arc<PageCoverage>,
}

/// Only items advanced by this fragment are stored here. Earlier ranges stay in
/// their original page even when a later page advances the same item.
#[derive(Debug, Default, PartialEq, Eq)]
struct PageCoverage {
    previous: Option<Arc<PageCoverage>>,
    items: HashMap<(Entity, Entity), ItemCoverage>,
}

impl Drop for PageCoverage {
    fn drop(&mut self) {
        // A long block can retain a checkpoint at every sibling boundary.
        let mut previous = self.previous.take();
        while let Some(mut page) = previous.and_then(Arc::into_inner) {
            previous = page.previous.take();
        }
    }
}

impl Default for SourceProgress {
    fn default() -> Self {
        static EMPTY: std::sync::OnceLock<Arc<PageCoverage>> = std::sync::OnceLock::new();
        Self {
            page: Arc::clone(EMPTY.get_or_init(Arc::default)),
        }
    }
}

impl SourceProgress {
    fn pages(&self) -> impl Iterator<Item = &PageCoverage> {
        std::iter::successors(Some(self.page.as_ref()), |page| page.previous.as_deref())
    }

    fn update(&mut self, key: (Entity, Entity)) -> &mut ItemCoverage {
        if Arc::get_mut(&mut self.page).is_none() {
            self.page = Arc::new(PageCoverage {
                previous: Some(Arc::clone(&self.page)),
                items: HashMap::new(),
            });
        }
        let page = Arc::get_mut(&mut self.page).expect("new page is uniquely owned");
        page.items.entry(key).or_default()
    }

    /// Resolve the shared deltas once for this layout invocation. Each replay
    /// entry retains only its own page data and an Arc to the previous page.
    pub(crate) fn index(&self) -> ProgressIndex {
        let mut items = HashMap::<_, ItemCoverage>::new();
        for page in self.pages() {
            for (&key, delta) in &page.items {
                let state = items.entry(key).or_default();
                state.terminal |= delta.terminal;
                for range in &delta.ranges {
                    state.consume(range.clone());
                }
            }
        }
        ProgressIndex { items }
    }

    /// Incorporate only the source reported by the selected child fragment.
    pub fn include(&mut self, contribution: &Self) {
        for page in contribution.pages() {
            for (&(owner, item), covered) in &page.items {
                let entry = self.update((owner, item));
                entry.terminal |= covered.terminal;
                for range in &covered.ranges {
                    entry.consume(range.clone());
                }
            }
        }
    }

    pub fn consume(&mut self, owner: Entity, item: Entity, range: Range<SourcePosition>) {
        if range.is_empty() {
            return;
        }
        self.update((owner, item)).consume(range);
    }

    #[must_use]
    pub fn covers(&self, owner: Entity, item: Entity, range: Range<SourcePosition>) -> bool {
        let mut combined = ItemCoverage::default();
        for delta in self
            .pages()
            .filter_map(|page| page.items.get(&(owner, item)))
        {
            for part in &delta.ranges {
                combined.consume(part.clone());
            }
        }
        range.is_empty()
            || combined
                .ranges
                .iter()
                .any(|r| r.start <= range.start && range.end <= r.end)
    }
    #[must_use]
    pub fn contains(&self, owner: Entity, item: Entity, position: SourcePosition) -> bool {
        self.pages().any(|page| {
            page.items
                .get(&(owner, item))
                .is_some_and(|state| state.contains(position))
        })
    }

    /// Record that a child contributed a fragment while its content remains unfinished.
    pub fn record_item(&mut self, owner: Entity, item: Entity) {
        self.update((owner, item));
    }

    pub fn contributed_items(&self) -> impl Iterator<Item = (Entity, Entity)> + '_ {
        self.pages().flat_map(|page| page.items.keys().copied())
    }

    pub fn finish_item(&mut self, owner: Entity, item: Entity) {
        self.update((owner, item)).terminal = true;
    }
    #[must_use]
    pub fn is_finished(&self, owner: Entity, item: Entity) -> bool {
        self.pages().any(|page| {
            page.items
                .get(&(owner, item))
                .is_some_and(|state| state.terminal)
        })
    }
}

pub(crate) struct ProgressIndex {
    items: HashMap<(Entity, Entity), ItemCoverage>,
}
impl ProgressIndex {
    pub(crate) fn prefix_end(&self, owner: Entity, item: Entity) -> SourcePosition {
        self.items
            .get(&(owner, item))
            .and_then(|s| s.ranges.first())
            .filter(|r| r.start == SourcePosition::default())
            .map_or_else(SourcePosition::default, |r| r.end)
    }
    pub(crate) fn is_finished(&self, owner: Entity, item: Entity) -> bool {
        self.items
            .get(&(owner, item))
            .is_some_and(|state| state.terminal)
    }
    pub(crate) fn contains(&self, owner: Entity, item: Entity, position: SourcePosition) -> bool {
        self.items
            .get(&(owner, item))
            .is_some_and(|state| state.contains(position))
    }
}
impl ItemCoverage {
    fn consume(&mut self, range: Range<SourcePosition>) {
        let ranges = &mut self.ranges;
        let first = ranges.partition_point(|stored| stored.end < range.start);
        let end = ranges.partition_point(|stored| stored.start <= range.end);
        let mut merged = range;
        if first < end {
            merged.start = merged.start.min(ranges[first].start);
            merged.end = merged.end.max(ranges[end - 1].end);
        }
        ranges.splice(first..end, [merged]);
    }

    fn contains(&self, position: SourcePosition) -> bool {
        let i = self.ranges.partition_point(|r| r.end <= position);
        self.ranges.get(i).is_some_and(|r| r.contains(&position))
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
    #[test]
    fn retained_page_starts_share_one_delta_per_page() {
        let mut dom = EcsDom::new();
        let owner = dom.create_text_ownerless_fixture("");
        let mut progress = SourceProgress::default();
        let mut starts = Vec::new();
        for _ in 0..100 {
            starts.push(progress.clone());
            for _ in 0..20 {
                let item = dom.create_text_ownerless_fixture("A");
                progress.consume(
                    owner,
                    item,
                    SourcePosition::byte(0)..SourcePosition::byte(1),
                );
                progress.finish_item(owner, item);
            }
            assert!(Arc::ptr_eq(
                progress.page.previous.as_ref().unwrap(),
                &starts.last().unwrap().page
            ));
        }
        assert_eq!(progress.pages().count(), 101);
        assert_eq!(
            progress.pages().map(|page| page.items.len()).sum::<usize>(),
            2000
        );
        assert_eq!(progress.index().items.len(), 2000);
    }
}
