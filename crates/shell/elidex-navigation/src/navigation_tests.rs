//! Unit tests for `super::NavigationController` / `HistoryEntry` — split from
//! `navigation.rs` at the 1000-line touch-time boundary (CLAUDE.md); `#[path]`
//! keeps `super::*` private-field access (the source module is `super`).

use super::*;

fn url(s: &str) -> url::Url {
    url::Url::parse(s).unwrap()
}

#[test]
fn new_is_empty() {
    let nav = NavigationController::new();
    assert!(nav.is_empty());
    assert_eq!(nav.len(), 0);
    assert!(nav.current_url().is_none());
    assert!(!nav.can_go_back());
    assert!(!nav.can_go_forward());
}

#[test]
fn push_and_current() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
    assert_eq!(nav.len(), 1);
}

#[test]
fn push_multiple() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    nav.push(url("https://c.com/"));
    assert_eq!(nav.current_url().unwrap().as_str(), "https://c.com/");
    assert_eq!(nav.len(), 3);
}

#[test]
fn go_back_and_forward() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    nav.push(url("https://c.com/"));

    assert!(nav.can_go_back());
    assert_eq!(nav.go_back().unwrap().as_str(), "https://b.com/");

    assert!(nav.can_go_forward());
    assert_eq!(nav.go_forward().unwrap().as_str(), "https://c.com/");
}

#[test]
fn go_back_at_start_returns_none() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    assert!(nav.go_back().is_none());
    assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
}

#[test]
fn go_forward_at_end_returns_none() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    assert!(nav.go_forward().is_none());
}

#[test]
fn push_after_back_truncates_forward() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    nav.push(url("https://c.com/"));
    nav.go_back();
    nav.go_back();
    // Now at a.com. Push a new URL.
    nav.push(url("https://d.com/"));
    assert_eq!(nav.current_url().unwrap().as_str(), "https://d.com/");
    assert_eq!(nav.len(), 2); // a, d
    assert!(!nav.can_go_forward());
}

#[test]
fn go_with_delta() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    nav.push(url("https://c.com/"));

    assert_eq!(nav.go(-2).unwrap().as_str(), "https://a.com/");

    assert_eq!(nav.go(2).unwrap().as_str(), "https://c.com/");
}

#[test]
fn go_out_of_range() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    assert!(nav.go(-1).is_none());
    assert!(nav.go(1).is_none());
    // Current position unchanged.
    assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
}

#[test]
fn peek_does_not_move_cursor() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    nav.push(url("https://c.com/"));
    // At c.com (index 2). Peeks must NOT move the cursor.
    assert_eq!(
        nav.peek_back().map(|(i, u)| (i, u.as_str())),
        Some((1, "https://b.com/"))
    );
    assert_eq!(nav.current_url().unwrap().as_str(), "https://c.com/");
    assert_eq!(
        nav.peek_go(-2).map(|(i, u)| (i, u.as_str())),
        Some((0, "https://a.com/"))
    );
    assert_eq!(nav.current_url().unwrap().as_str(), "https://c.com/");
    // Forward is a no-op at the end.
    assert!(nav.peek_forward().is_none());
    // Out-of-range peeks return None without moving.
    assert!(nav.peek_go(5).is_none());
    assert_eq!(nav.current_url().unwrap().as_str(), "https://c.com/");
}

#[test]
fn peek_then_commit_is_atomic() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    // At b.com (index 1). Peek back but DON'T commit — cursor unmoved.
    let (target, _) = nav.peek_back().unwrap();
    assert_eq!(nav.current_url().unwrap().as_str(), "https://b.com/");
    // Commit the peeked index — now the cursor moves.
    nav.commit_index(target);
    assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
}

#[test]
fn peek_go_zero_is_current() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    assert_eq!(
        nav.peek_go(0).map(|(i, u)| (i, u.as_str())),
        Some((1, "https://b.com/"))
    );
}

#[test]
fn replace_current() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    nav.replace(url("https://b-replaced.com/"));
    assert_eq!(
        nav.current_url().unwrap().as_str(),
        "https://b-replaced.com/"
    );
    assert_eq!(nav.len(), 2);
}

#[test]
fn replace_empty_acts_as_push() {
    let mut nav = NavigationController::new();
    nav.replace(url("https://a.com/"));
    assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
    assert_eq!(nav.len(), 1);
}

#[test]
fn set_title() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.set_current_title("Page A".to_string());
    assert_eq!(nav.current_url().unwrap().as_str(), "https://a.com/");
    assert_eq!(nav.current_title(), Some("Page A"));
}

#[test]
fn go_zero_returns_current() {
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    assert_eq!(nav.go(0).unwrap().as_str(), "https://b.com/");
}

#[test]
fn current_title_empty_history() {
    let nav = NavigationController::new();
    assert_eq!(nav.current_title(), None);
}

#[test]
fn push_evicts_oldest_when_over_cap() {
    let mut nav = NavigationController::new();
    for i in 0..=MAX_HISTORY_ENTRIES {
        nav.push(url(&format!("https://page{i}.com/")));
    }
    // Should have been capped at MAX_HISTORY_ENTRIES.
    assert_eq!(nav.len(), MAX_HISTORY_ENTRIES);
    // The oldest entry (page0) should have been evicted.
    assert_eq!(nav.entries[0].url.as_str(), "https://page1.com/");
    // Current URL is the last pushed.
    assert_eq!(
        nav.current_url().unwrap().as_str(),
        &format!("https://page{MAX_HISTORY_ENTRIES}.com/")
    );
    // Index should point to the last entry.
    assert_eq!(nav.index, Some(MAX_HISTORY_ENTRIES - 1));
}

#[test]
fn push_evicts_preserves_back_navigation() {
    let mut nav = NavigationController::new();
    for i in 0..=MAX_HISTORY_ENTRIES {
        nav.push(url(&format!("https://page{i}.com/")));
    }
    // Can still go back.
    assert!(nav.can_go_back());
    let expected_idx = MAX_HISTORY_ENTRIES - 1;
    assert_eq!(
        nav.go_back().unwrap().as_str(),
        &format!("https://page{expected_idx}.com/")
    );
}

// --- Same-document classifier (WHATWG HTML §7.4.2.2 navigate step 15) ---

/// Pin url 2.x's `fragment()` distinction that the classifier's step-15
/// "url's fragment is non-null" conjunct rests on: a *removed* fragment is
/// `None` (⇒ CrossDocument), an *emptied* `#` fragment is `Some("")`
/// (⇒ SameDocument), and a present fragment is `Some("x")`. If a url-crate
/// change ever collapsed emptied and removed, the classifier correction
/// (removal ⇒ CrossDocument, emptied ⇒ SameDocument) would silently regress.
#[test]
fn url_crate_fragment_semantics_pinned() {
    assert_eq!(url("http://x/a").fragment(), None, "removed ⇒ None");
    assert_eq!(
        url("http://x/a#").fragment(),
        Some(""),
        "emptied ⇒ Some(\"\")"
    );
    assert_eq!(
        url("http://x/a#x").fragment(),
        Some("x"),
        "present ⇒ Some(\"x\")"
    );
}

/// The full same-document classification truth table (plan §4.2 / §9).
/// SameDocument IFF the URLs are equal excluding fragments AND the target
/// fragment is non-null (navigate step 15 conjuncts 3-4). The **removal**
/// (`/a#x → /a`) and **query-differ** (`/a?q=1 → /a?q=2#x`) rows are exactly
/// the ones a naive "fragments differ" predicate gets wrong, so they are
/// pinned here alongside every other case.
#[test]
fn classify_navigation_truth_table() {
    use NavClass::{CrossDocument, SameDocument};
    // (current, target, expected, label)
    let cases = [
        ("http://x/a", "http://x/a#x", SameDocument, "add fragment"),
        (
            "http://x/a#x",
            "http://x/a#y",
            SameDocument,
            "change fragment",
        ),
        (
            "http://x/a#x",
            "http://x/a",
            CrossDocument,
            "remove fragment (target frag null)",
        ),
        (
            "http://x/a#x",
            "http://x/a#",
            SameDocument,
            "empty fragment (target frag Some(\"\"))",
        ),
        (
            "http://x/a",
            "http://x/a#",
            SameDocument,
            "add empty fragment",
        ),
        (
            "http://x/a#x",
            "http://x/a#x",
            SameDocument,
            "identical incl. fragment",
        ),
        (
            "http://x/a",
            "http://x/a",
            CrossDocument,
            "identical, no fragment",
        ),
        ("http://x/a", "http://x/b", CrossDocument, "path differs"),
        (
            "http://x/a?q=1",
            "http://x/a?q=2#x",
            CrossDocument,
            "query differs (even with a fragment)",
        ),
        (
            "http://x/a",
            "https://x/a#x",
            CrossDocument,
            "scheme differs",
        ),
        ("http://x/a", "http://y/a#x", CrossDocument, "host differs"),
    ];
    for (current, target, expected, label) in cases {
        assert_eq!(
            classify_navigation(&url(current), &url(target)),
            expected,
            "classify_navigation({current}, {target}) [{label}]",
        );
    }
}

/// The helper compares normalized `url`-crate serializations, so it is
/// robust where a crude `split('#')` string compare would differ: a
/// default-port target is equal-excluding-fragments to its port-less form,
/// so it classifies SameDocument given a non-null fragment.
#[test]
fn classify_navigation_normalizes_default_port() {
    assert_eq!(
        classify_navigation(&url("http://x:80/a"), &url("http://x/a#f")),
        NavClass::SameDocument,
    );
}

// --- 5c: serialized-state + scroll storage / traversal read path ---

#[test]
fn set_current_state_and_scroll_write_current_entry_and_entry_reads_them() {
    // The pushState/replaceState drain writes the serialized state + captured
    // scroll onto the CURRENT entry; a traversal reads them back from the
    // PEEKED TARGET entry via `entry(index)` (the traversal read-source).
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push(url("https://b.com/"));
    nav.set_current_state(Some(b"{\"n\":2}".to_vec()));
    nav.set_current_scroll((10.0, 20.0));
    let e = nav.entry(1).expect("current entry exists");
    assert_eq!(
        e.classic_history_api_state.as_deref(),
        Some(b"{\"n\":2}".as_slice())
    );
    assert_eq!(e.scroll_position, Some((10.0, 20.0)));
    // The other (a) entry is untouched — default `None` state + scroll.
    let a = nav.entry(0).expect("entry 0 exists");
    assert_eq!(a.classic_history_api_state, None);
    assert_eq!(a.scroll_position, None);
    // Out-of-range index → `None` (no panic).
    assert!(nav.entry(99).is_none());
}

#[test]
fn push_starts_state_and_scroll_at_none() {
    // A fresh navigation entry carries no classic state / scroll until the
    // drain writes them — a plain nav's traversal restores `null` + no scroll.
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.set_current_state(Some(b"x".to_vec()));
    nav.set_current_scroll((1.0, 2.0));
    nav.push(url("https://b.com/"));
    let e = nav.entry(1).expect("new entry exists");
    assert_eq!(e.classic_history_api_state, None);
    assert_eq!(e.scroll_position, None);
}

#[test]
fn resolve_traversal_classifies_by_document_identity_not_url() {
    // The one engine-independent traversal decision, by DOCUMENT IDENTITY
    // (§7.4.6.1 step 14.10), NOT URL: `pushState` routing gives same-document
    // entries different URLs (must be SameDocument), and a fresh nav to an
    // existing URL gives a different document the same URL (must be Rebuild).
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/")); // doc 1, index 0
    nav.push_same_document(url("https://a.com/products")); // doc 1, index 1 (pushState routing)
    nav.set_current_state(Some(b"{\"n\":2}".to_vec()));
    nav.set_current_scroll((5.0, 6.0));
    nav.push_same_document(url("https://a.com/products/2")); // doc 1, index 2
                                                             // At index 2. A `go(0)` (target == current) is a RELOAD (Rebuild), NOT a
                                                             // same-document no-op (History.go step 4).
    assert_eq!(nav.resolve_traversal(2), TraversalKind::Rebuild);
    // back() to /products (index 1) — DIFFERENT URL, SAME document → restore.
    assert_eq!(
        nav.resolve_traversal(1),
        TraversalKind::SameDocument {
            state: Some(b"{\"n\":2}".to_vec()),
            scroll: Some((5.0, 6.0)),
        }
    );
    // back() to / (index 0) — same document, no state/scroll.
    assert_eq!(
        nav.resolve_traversal(0),
        TraversalKind::SameDocument {
            state: None,
            scroll: None,
        }
    );
}

#[test]
fn resolve_traversal_rebuilds_across_documents_and_same_url_different_doc() {
    // A cross-document navigation (different `document_sequence`) → Rebuild,
    // even when the URLs are equal-excluding-fragments — the case a URL-based
    // classifier gets WRONG (stale document). `location.replace()` and reload
    // are new-document events too.
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/")); // doc 1, index 0
    nav.push(url("https://a.com/")); // doc 2, index 1 — SAME url, fresh nav
                                     // At index 1 (doc 2). back() to index 0 (doc 1) — same URL, DIFFERENT
                                     // document → Rebuild (not a stale same-document no-rebuild).
    assert_eq!(nav.resolve_traversal(0), TraversalKind::Rebuild);

    // `location.replace()` stamps a NEW document even in place.
    let mut nav2 = NavigationController::new();
    nav2.push(url("https://a.com/")); // doc 1
    nav2.push_same_document(url("https://a.com/x")); // doc 1 (pushState)
    nav2.replace(url("https://a.com/y")); // location.replace() → doc 2 at index 1
    nav2.commit_index(0); // pretend a traversal committed to index 0
                          // From doc-1 entry 0, resolving to index 1 (now doc 2) → Rebuild.
    assert_eq!(nav2.resolve_traversal(1), TraversalKind::Rebuild);

    // reload re-stamps: after a fragment push (shared doc), reloading the base
    // makes a later back to the fragment cross-document.
    let mut nav3 = NavigationController::new();
    nav3.push(url("https://a.com/")); // doc 1, index 0
    nav3.push_same_document(url("https://a.com/#x")); // doc 1, index 1
    nav3.commit_index(0); // back to base
    nav3.restamp_current_document(); // reload the base → doc 2 at index 0
                                     // From reloaded base (doc 2, index 0), forward to #x (still doc 1) → Rebuild.
    assert_eq!(nav3.resolve_traversal(1), TraversalKind::Rebuild);
}

#[test]
fn replace_clears_prior_state_and_scroll_new_document() {
    // `location.replace()` stamps a NEW document in place → it must carry NONE
    // of the replaced (e.g. pushState'd) entry's classic state or scroll, else a
    // later reload/traversal resurrects stale state (Codex R1 F2).
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/"));
    nav.push_same_document(url("https://a.com/x")); // pushState entry
    nav.set_current_state(Some(b"{\"n\":1}".to_vec()));
    nav.set_current_scroll((10.0, 20.0));
    let seq_before = nav.entry(1).unwrap().document_sequence;
    // location.replace() → new document, state + scroll cleared.
    nav.replace(url("https://a.com/y"));
    let e = nav.entry(1).unwrap();
    assert_eq!(
        e.classic_history_api_state, None,
        "replace clears classic state"
    );
    assert_eq!(e.scroll_position, None, "replace clears scroll");
    assert_ne!(
        e.document_sequence, seq_before,
        "replace stamps a new document_sequence"
    );
    // replace_same_document (replaceState) does NOT clear — same document, the
    // caller writes the new state.
    nav.set_current_state(Some(b"keep".to_vec()));
    nav.set_current_scroll((1.0, 2.0));
    nav.replace_same_document(url("https://a.com/z"));
    let e = nav.entry(1).unwrap();
    assert_eq!(
        e.classic_history_api_state.as_deref(),
        Some(b"keep".as_slice()),
        "replace_same_document keeps state (caller overwrites)"
    );
    assert_eq!(e.scroll_position, Some((1.0, 2.0)));
}

#[test]
fn rebuild_traversal_restamps_target_so_siblings_stay_cross_document() {
    // A cross-document traversal REBUILDS the target as a fresh document, so the
    // shell re-stamps it (`restamp_current_document` after `commit_index`).
    // Without that re-stamp, the rebuilt target keeps the `document_sequence` it
    // shared with its former pushState siblings and a later traversal to such a
    // sibling mis-classifies same-document (stale document under a swapped URL).
    let mut nav = NavigationController::new();
    nav.push(url("https://a.com/")); // doc 1, index 0 (A)
    nav.push_same_document(url("https://a.com/a2")); // doc 1, index 1 (pushState sibling)
    nav.push(url("https://b.com/")); // doc 2, index 2 (cross-doc — destroys A's doc)
                                     // back() to /a2 (index 1): cross-document (D2 vs D1) → Rebuild.
    assert_eq!(nav.resolve_traversal(1), TraversalKind::Rebuild);
    // The shell rebuilds /a2 fresh, then commits + re-stamps.
    nav.commit_index(1);
    nav.restamp_current_document();
    // back() to A (index 0): A's document was destroyed → must be Rebuild. Without
    // the re-stamp above, entry[1] would still be D1 == entry[0] D1 → a wrong
    // SameDocument (stale /a2 content shown under the /a URL).
    assert_eq!(nav.resolve_traversal(0), TraversalKind::Rebuild);
}

#[test]
fn eviction_preserves_entry_state() {
    // FIFO eviction over the cap keeps each surviving entry's serialized state
    // (the state rides the entry — a single Vec — not a parallel side-store, so
    // it evicts + re-indexes atomically with the entry).
    let mut nav = NavigationController::new();
    for i in 0..=MAX_HISTORY_ENTRIES {
        nav.push(url(&format!("https://page{i}.com/")));
        nav.set_current_state(Some(format!("{{\"i\":{i}}}").into_bytes()));
    }
    assert_eq!(nav.len(), MAX_HISTORY_ENTRIES);
    // page0 evicted; the oldest survivor is page1 carrying its own state.
    let first = nav.entry(0).expect("oldest survivor exists");
    assert_eq!(first.url.as_str(), "https://page1.com/");
    assert_eq!(
        first.classic_history_api_state.as_deref(),
        Some(b"{\"i\":1}".as_slice())
    );
    // The current (last) entry keeps its state too.
    let last = nav
        .entry(MAX_HISTORY_ENTRIES - 1)
        .expect("current entry exists");
    assert_eq!(
        last.classic_history_api_state.as_deref(),
        Some(format!("{{\"i\":{MAX_HISTORY_ENTRIES}}}").as_bytes())
    );
}
