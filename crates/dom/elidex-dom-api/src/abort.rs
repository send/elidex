//! DOM §3.2 Interface AbortSignal — the dependent-signal graph and the two
//! algorithms that read and write it ("create a dependent abort signal",
//! "signal abort" steps 1-4).
//!
//! Engine-independent: the module is generic over the host's handle (`H`)
//! and reason (`R`) types and runs no abort step — "run the abort steps"
//! (the abort algorithms and the `abort` event) fires JS and is the host's.
//! The line is the spec's own: steps 1-4 of "signal abort" read and write
//! reasons and links; steps 5-6 run the abort steps.
//!
//! A dependent signal records the **non-dependent** sources of its inputs
//! (a dependent input contributes its own source signals, not itself), so
//! no dependent signal ever has dependent signals of its own and "signal
//! abort" fans out exactly one level.
//!
//! Slot `#11-abort-signal-any-dependent-flattening`.

use std::collections::HashSet;
use std::hash::Hash;

/// An Infra ordered set: insertion order is kept, an append of a present
/// member is a no-op.  Both halves are updated by every write, so the
/// hashed membership test keeps an append O(1).
#[derive(Debug)]
struct OrderedSet<H> {
    order: Vec<H>,
    members: HashSet<H>,
}

impl<H> Default for OrderedSet<H> {
    fn default() -> Self {
        Self {
            order: Vec::new(),
            members: HashSet::new(),
        }
    }
}

impl<H: Copy + Eq + Hash> OrderedSet<H> {
    fn append(&mut self, h: H) {
        if self.members.insert(h) {
            self.order.push(h);
        }
    }

    fn retain(&mut self, mut keep: impl FnMut(H) -> bool) {
        let members = &mut self.members;
        self.order.retain(|&h| {
            let alive = keep(h);
            if !alive {
                members.remove(&h);
            }
            alive
        });
    }

    fn take(&mut self) -> Vec<H> {
        self.members.clear();
        std::mem::take(&mut self.order)
    }
}

/// The spec's per-signal record for the dependent-signal graph: a
/// **dependent** flag, the associated **source signals** and the associated
/// **dependent signals** (weak sets of signals).
///
/// All fields are private: they are written only by this module's
/// algorithms ([`create_dependent_abort_signal`], [`signal_abort`]) and the
/// sweep prune [`Self::retain_live`], so a host cannot set `dependent` or
/// push a handle.
#[derive(Debug)]
pub struct DependentLinks<H> {
    dependent: bool,
    source_signals: OrderedSet<H>,
    dependent_signals: OrderedSet<H>,
}

impl<H> Default for DependentLinks<H> {
    fn default() -> Self {
        Self {
            dependent: false,
            source_signals: OrderedSet::default(),
            dependent_signals: OrderedSet::default(),
        }
    }
}

impl<H: Copy + Eq + Hash> DependentLinks<H> {
    /// The signal's **dependent** flag (initially false; set by "create a
    /// dependent abort signal" step 3).
    #[must_use]
    pub fn is_dependent(&self) -> bool {
        self.dependent
    }

    /// The signal's **source signals**, in append order.
    #[must_use]
    pub fn source_signals(&self) -> &[H] {
        &self.source_signals.order
    }

    /// The signal's **dependent signals**, in append order.
    #[must_use]
    pub fn dependent_signals(&self) -> &[H] {
        &self.dependent_signals.order
    }

    /// "create a dependent abort signal" step 3.
    fn set_dependent(&mut self) {
        self.dependent = true;
    }

    /// Infra "append" to the source signals set (a no-op for a member).
    fn append_source(&mut self, h: H) {
        self.source_signals.append(h);
    }

    /// Infra "append" to the dependent signals set (a no-op for a member).
    fn append_dependent(&mut self, h: H) {
        self.dependent_signals.append(h);
    }

    /// Drop every member for which `alive` is false, from both weak sets.
    ///
    /// The one prune rule: the sets are weak by spec (DOM §3.2 "weak set"),
    /// so a collected signal must vanish from them.  The host calls this
    /// from its sweep with its own liveness test; the module never learns
    /// what "alive" means.
    pub fn retain_live(&mut self, alive: impl Fn(H) -> bool) {
        self.source_signals.retain(&alive);
        self.dependent_signals.retain(&alive);
    }

    /// Empty the dependent signals set and return its members in order
    /// (the set is never read again once "signal abort" has planned them).
    fn take_dependents(&mut self) -> Vec<H> {
        self.dependent_signals.take()
    }
}

/// The host's storage for signals: one row per signal holding its abort
/// reason and its [`DependentLinks`].
pub trait AbortSignalGraph {
    /// A signal handle.  Never dereferenced by the module; the host owns
    /// what it means (and when it is recycled).
    type Handle: Copy + Eq + Hash;
    /// An abort reason value.
    type Reason: Copy;

    /// The signal's abort reason; `None` ⇔ not aborted (DOM §3.2: "aborted
    /// when its abort reason is not undefined").  Specified only for a
    /// handle with a row (`links(h).is_some()`).
    fn abort_reason(&self, h: Self::Handle) -> Option<Self::Reason>;

    /// Set the signal's abort reason.
    fn set_abort_reason(&mut self, h: Self::Handle, reason: Self::Reason);

    /// The signal's links; `None` ⇔ no such signal (the existence test).
    fn links(&self, h: Self::Handle) -> Option<&DependentLinks<Self::Handle>>;

    /// Mutable access to the signal's links; `None` ⇔ no such signal.
    fn links_mut(&mut self, h: Self::Handle) -> Option<&mut DependentLinks<Self::Handle>>;
}

/// Outcome of [`create_dependent_abort_signal`].
#[derive(Debug, PartialEq, Eq)]
pub enum DependentCreation<R> {
    /// Step 2: an input was already aborted.  The host sets the result's
    /// abort reason to this value; the result is **not** dependent and has
    /// no links.
    AlreadyAborted(R),
    /// Steps 3-4: the result is dependent and linked to its sources.
    Linked,
}

/// DOM §3.2 "create a dependent abort signal" steps 2-5.  `result` is the
/// host's step-1 object (a fresh signal with a default row); `signals` are
/// the inputs, already validated by the host as live signals.
pub fn create_dependent_abort_signal<G: AbortSignalGraph>(
    g: &mut G,
    result: G::Handle,
    signals: &[G::Handle],
) -> DependentCreation<G::Reason> {
    // Step 2: an aborted input short-circuits (the result is not dependent).
    for &signal in signals {
        if g.links(signal).is_none() {
            debug_assert!(false, "create_dependent_abort_signal: input has no row");
            continue;
        }
        if let Some(reason) = g.abort_reason(signal) {
            return DependentCreation::AlreadyAborted(reason);
        }
    }

    // Step 3.
    let Some(result_links) = g.links_mut(result) else {
        debug_assert!(false, "create_dependent_abort_signal: result has no row");
        return DependentCreation::Linked;
    };
    result_links.set_dependent();

    // Step 4.
    for &signal in signals {
        let Some(links) = g.links(signal) else {
            continue; // already reported in step 2's loop
        };
        if links.is_dependent() {
            // 4.2: a dependent input contributes its own source signals.
            let sources: Vec<G::Handle> = links.source_signals().to_vec();
            for source in sources {
                debug_assert!(
                    g.abort_reason(source).is_none()
                        && g.links(source).is_some_and(|l| !l.is_dependent()),
                    "4.2.1: a source signal is neither aborted nor dependent"
                );
                link(g, result, source);
            }
        } else {
            // 4.1.
            link(g, result, signal);
        }
    }
    DependentCreation::Linked
}

/// "Append source to result's source signals; append result to source's
/// dependent signals" (steps 4.1.1-4.1.2 / 4.2.2-4.2.3).
fn link<G: AbortSignalGraph>(g: &mut G, result: G::Handle, source: G::Handle) {
    if let Some(links) = g.links_mut(result) {
        links.append_source(source);
    }
    if let Some(links) = g.links_mut(source) {
        links.append_dependent(result);
    }
}

/// The ordered list of "run the abort steps" calls "signal abort" steps 5-6
/// leave to the host: first `signal`, then each of `dependents`.
#[derive(Debug, PartialEq, Eq)]
pub struct AbortPlan<H> {
    /// Step 5's subject.
    pub signal: H,
    /// Step 6's `dependentSignalsToAbort`, in order.
    pub dependents: Vec<H>,
}

/// DOM §3.2 "signal abort" steps 1-4.
///
/// Returns `None` when `signal` is already aborted (step 1) or is no signal
/// at all (`links(signal) == None`).  Otherwise sets the signal's abort
/// reason (step 2) and every non-aborted dependent's (step 4), empties the
/// signal's dependent set (its members are all aborted now and the set is
/// never read again) and returns the plan for steps 5-6.  `reason` is the
/// host-materialised abort reason (step 2's "otherwise" branch — a new
/// "AbortError" `DOMException` — is engine-bound).
///
/// No JS runs here: every reason is set before the host runs any abort
/// step, which is the spec's order.
pub fn signal_abort<G: AbortSignalGraph>(
    g: &mut G,
    signal: G::Handle,
    reason: G::Reason,
) -> Option<AbortPlan<G::Handle>> {
    g.links(signal)?;
    // Step 1.
    if g.abort_reason(signal).is_some() {
        return None;
    }
    // Step 2.
    g.set_abort_reason(signal, reason);
    // Steps 3-4.
    let candidates = g.links_mut(signal)?.take_dependents();
    let mut dependents = Vec::with_capacity(candidates.len());
    for dependent in candidates {
        if g.links(dependent).is_none() {
            continue; // collected; the sweep prunes, so unreachable in the VM
        }
        if g.abort_reason(dependent).is_none() {
            g.set_abort_reason(dependent, reason);
            dependents.push(dependent);
        }
    }
    Some(AbortPlan { signal, dependents })
}

// Graph fixtures name signals a, b, d1, ... after the spec's prose.
#[cfg(test)]
#[allow(clippy::many_single_char_names)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The engine-independent reading of the contract: a `HashMap` graph.
    #[derive(Default)]
    struct TestGraph {
        rows: HashMap<u32, (Option<&'static str>, DependentLinks<u32>)>,
    }

    impl TestGraph {
        fn signal(&mut self, h: u32) -> u32 {
            self.rows.insert(h, (None, DependentLinks::default()));
            h
        }
        fn aborted_signal(&mut self, h: u32, reason: &'static str) -> u32 {
            self.rows
                .insert(h, (Some(reason), DependentLinks::default()));
            h
        }
        fn l(&self, h: u32) -> &DependentLinks<u32> {
            &self.rows[&h].1
        }
    }

    impl AbortSignalGraph for TestGraph {
        type Handle = u32;
        type Reason = &'static str;
        fn abort_reason(&self, h: u32) -> Option<&'static str> {
            self.rows.get(&h).and_then(|r| r.0)
        }
        fn set_abort_reason(&mut self, h: u32, reason: &'static str) {
            self.rows.get_mut(&h).unwrap().0 = Some(reason);
        }
        fn links(&self, h: u32) -> Option<&DependentLinks<u32>> {
            self.rows.get(&h).map(|r| &r.1)
        }
        fn links_mut(&mut self, h: u32) -> Option<&mut DependentLinks<u32>> {
            self.rows.get_mut(&h).map(|r| &mut r.1)
        }
    }

    #[test]
    fn flattening_sources_are_never_dependent() {
        let mut g = TestGraph::default();
        let (a, b) = (g.signal(1), g.signal(2));
        let d1 = g.signal(3);
        assert_eq!(
            create_dependent_abort_signal(&mut g, d1, &[a]),
            DependentCreation::Linked
        );
        let d2 = g.signal(4);
        create_dependent_abort_signal(&mut g, d2, &[d1, b]);
        assert_eq!(g.l(d2).source_signals(), &[a, b]);
        assert_eq!(g.l(a).dependent_signals(), &[d1, d2]);
        assert!(g.l(d1).dependent_signals().is_empty());
        assert!(g.l(d2).is_dependent() && g.l(d1).is_dependent());
        assert!(!g.l(a).is_dependent());
    }

    #[test]
    fn already_aborted_input_short_circuits() {
        let mut g = TestGraph::default();
        let a = g.aborted_signal(1, "ra");
        let b = g.signal(2);
        let r = g.signal(3);
        assert_eq!(
            create_dependent_abort_signal(&mut g, r, &[a, b]),
            DependentCreation::AlreadyAborted("ra")
        );
        assert!(!g.l(r).is_dependent());
        assert!(g.l(r).source_signals().is_empty());
        assert!(g.l(b).dependent_signals().is_empty());
        // The empty list: step 3 with nothing to link — dependent, no sources.
        let e = g.signal(4);
        assert_eq!(
            create_dependent_abort_signal(&mut g, e, &[]),
            DependentCreation::Linked
        );
        assert!(g.l(e).is_dependent() && g.l(e).source_signals().is_empty());
    }

    #[test]
    fn set_semantics() {
        let mut g = TestGraph::default();
        let a = g.signal(1);
        let r = g.signal(2);
        create_dependent_abort_signal(&mut g, r, &[a, a]);
        assert_eq!(g.l(r).source_signals(), &[a]);
        assert_eq!(g.l(a).dependent_signals(), &[r]);
    }

    #[test]
    fn signal_abort_plans_then_empties() {
        let mut g = TestGraph::default();
        let a = g.signal(1);
        let (d1, d2) = (g.signal(2), g.signal(3));
        create_dependent_abort_signal(&mut g, d1, &[a]);
        create_dependent_abort_signal(&mut g, d2, &[a]);
        g.set_abort_reason(d2, "pre");
        let plan = signal_abort(&mut g, a, "r").unwrap();
        assert_eq!(
            plan,
            AbortPlan {
                signal: a,
                dependents: vec![d1]
            }
        );
        assert_eq!(g.abort_reason(a), Some("r"));
        assert_eq!(g.abort_reason(d1), Some("r"));
        assert_eq!(g.abort_reason(d2), Some("pre"));
        assert!(g.l(a).dependent_signals().is_empty());
        assert!(signal_abort(&mut g, a, "again").is_none());
        // A handle with no row is no signal.
        assert!(signal_abort(&mut g, 99, "x").is_none());
    }

    #[test]
    fn retain_live_prunes_both_sets() {
        let mut g = TestGraph::default();
        let (a, b) = (g.signal(1), g.signal(2));
        let d = g.signal(3);
        create_dependent_abort_signal(&mut g, d, &[a, b]);
        let e = g.signal(4);
        create_dependent_abort_signal(&mut g, e, &[a]);
        g.links_mut(d).unwrap().retain_live(|h| h != b);
        g.links_mut(a).unwrap().retain_live(|h| h != e);
        assert_eq!(g.l(d).source_signals(), &[a]);
        assert_eq!(g.l(a).dependent_signals(), &[d]);
        // Membership follows the order: a pruned member can be appended again.
        create_dependent_abort_signal(&mut g, e, &[a]);
        assert_eq!(g.l(a).dependent_signals(), &[d, e]);
    }
}
