//! DOM→cascade stylesheet re-collection (S5-6 §4.2) — the DOM-as-truth
//! replacement for the shell's stylesheet shadow-sync: assembles the
//! cascade-input stylesheets every frame from the live DOM's `<style>` /
//! loaded `<link rel="stylesheet">` owners, re-parsing only changed owners
//! via the per-owner [`CollectedStylesheet`] version-stamp component.
//!
//! Split from [`cssom_sheet`](crate::cssom_sheet) at touch-time (the
//! CLAUDE.md 1000-line discipline — the re-collection is a distinct
//! cohesion unit from the CSSOM API surface): `cssom_sheet` keeps the
//! session sheet cache, the rule handlers, and the `document.styleSheets`
//! walker; this module consumes its shared owner-source primitives
//! (`parse_owner_source` / `sheet_version` / `collect_stylesheet_owners`).

use std::sync::Arc;

use elidex_css::Stylesheet;
use elidex_ecs::{EcsDom, Entity};

use crate::cssom_sheet::{collect_stylesheet_owners, parse_owner_source, sheet_version};

/// Per-owner parsed-stylesheet cache backing
/// [`collect_document_stylesheets`] — derived data stamped on the owner
/// entity as an ECS component (CLAUDE.md side-store rule: per-entity,
/// `Send + Sync`, not a per-VM identity handle ⇒ component, not a
/// side map).
///
/// `version` is the owner's `sheet_version` at parse time; comparing it
/// against the live signal detects source divergence in O(1) without
/// materialising the source (the same discipline as the CSSOM cache in
/// this module's doc). The component is written through the raw,
/// non-instrumented `world_mut().insert_one` seam (the derived-data
/// precedent: the `elidex-style` computed-style / pseudo-element writes),
/// so the stamp write bumps NO version counter — the re-collection cannot
/// self-trigger (E11 false-positive direction). The component is
/// reclaimed at document teardown (`despawn_subtree`); a mid-life removal
/// is a detach, so a script-removed owner keeps its stamp but drops out
/// of the collection via the connectedness (tree-walk) filter.
pub struct CollectedStylesheet {
    /// The parsed stylesheet as of `version`, shared (`Arc`) with the
    /// cascade-input `Vec` that [`collect_document_stylesheets`] returns —
    /// a cache hit is a pointer bump, never a deep `Stylesheet` clone.
    pub parsed: Arc<Stylesheet>,
    /// The owner's `sheet_version` at parse time.
    pub version: u64,
}

/// Assemble the document's cascade-input stylesheets from the DOM —
/// the DOM-as-truth replacement for the shell's stylesheet shadow-sync
/// (S5-6 §4.2): a document-order, connectedness-filtered tree walk from
/// `document` enumerating every stylesheet owner (`<style>` + loaded
/// `<link rel="stylesheet">` — the same CSSOM §6.8 owner set as
/// `document.styleSheets`), re-parsing via [`elidex_css::parse_stylesheet`]
/// ONLY the owners whose `sheet_version` diverged from their
/// [`CollectedStylesheet`] stamp.
///
/// Designed to be called every frame: the no-change path is O(#owners)
/// version compares plus `Arc` bumps plus the walk — the returned sheets
/// are `Arc`-shared with the per-owner stamps, so assembly is pointer
/// copies and the S5-6b caller can take a fresh `Vec` on every `re_render`
/// without deep-cloning a stylesheet. Removal needs no dedicated signal —
/// a script-removed owner is detached, not despawned
/// (`EcsDom::remove_child` only detaches; the sole `despawn_subtree`
/// caller is document teardown), so it simply stops being reachable from
/// `document` and drops out of the walk while remaining a live entity.
#[must_use]
pub fn collect_document_stylesheets(document: Entity, dom: &mut EcsDom) -> Vec<Arc<Stylesheet>> {
    let owners = collect_stylesheet_owners(document, dom);
    let mut sheets = Vec::with_capacity(owners.len());
    for owner in owners {
        let version = sheet_version(owner, dom);
        let cached = dom
            .world()
            .get::<&CollectedStylesheet>(owner)
            .ok()
            .filter(|c| c.version == version)
            .map(|c| Arc::clone(&c.parsed));
        if let Some(parsed) = cached {
            sheets.push(parsed);
            continue;
        }
        // Miss: one parse (the shared owner-source branch), then two `Arc`
        // bumps — one for the cascade input, one for the stamp.
        let parsed = Arc::new(parse_owner_source(owner, dom));
        sheets.push(Arc::clone(&parsed));
        // Raw non-instrumented stamp write (see [`CollectedStylesheet`]):
        // routing this through an instrumented mutation path would bump the
        // very versions the collect compares (and the document-root change
        // signal) on every re-parse — self-triggering re-parse + spurious
        // renders (E11 false-positive direction).
        let _ = dom
            .world_mut()
            .insert_one(owner, CollectedStylesheet { parsed, version });
    }
    sheets
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use elidex_css::{parse_stylesheet, Origin, Stylesheet};
    use elidex_ecs::{Attributes, EcsDom, Entity, LinkStylesheet};
    use elidex_script_session::SessionCore;

    use super::{collect_document_stylesheets, CollectedStylesheet};
    use crate::cssom_sheet::{flush_sheet_mutation, sheet_version, sync_and_get_state};

    fn dom_with_root() -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        (dom, root)
    }

    /// Append a connected `<style>` with the given CSS text.
    fn append_style(dom: &mut EcsDom, parent: Entity, css: &str) -> Entity {
        let style = dom.create_element("style", Attributes::default());
        let text = dom.create_text(css);
        assert!(dom.append_child(style, text));
        assert!(dom.append_child(parent, style));
        style
    }

    /// Append a connected `<link rel="stylesheet">` whose sheet has
    /// "loaded" (a `LinkStylesheet` component, as the loader attaches it).
    fn append_loaded_link(dom: &mut EcsDom, parent: Entity, css: &str) -> Entity {
        let link = dom.create_element("link", Attributes::default());
        assert!(dom.append_child(parent, link));
        let _ = dom.world_mut().insert_one(
            link,
            LinkStylesheet {
                source: css.to_string(),
                href: "https://example.test/sheet.css".to_string(),
                version: 1,
            },
        );
        link
    }

    /// Make a re-parse observable: swap each owner's cached `parsed` for an
    /// empty sentinel `Arc` while keeping the version stamp. A cache hit
    /// returns the sentinel; a re-parse would return the real rules again.
    fn sentinel_swap_cached(dom: &mut EcsDom, owners: &[Entity]) {
        for &owner in owners {
            let version = dom
                .world()
                .get::<&CollectedStylesheet>(owner)
                .expect("stamped by a prior collect")
                .version;
            let _ = dom.world_mut().insert_one(
                owner,
                CollectedStylesheet {
                    parsed: Arc::new(Stylesheet::default()),
                    version,
                },
            );
        }
    }

    /// Simulate a script CSSOM mutation on `owner`: fill the session cache,
    /// replace the parsed sheet, then flush it back to the owner source —
    /// the same sync → mutate → flush sequence the `InsertRule` handler runs.
    fn write_back(owner: Entity, css: &str, session: &mut SessionCore, dom: &mut EcsDom) {
        let state = sync_and_get_state(owner, session, dom);
        state.parsed = parse_stylesheet(css, Origin::Author);
        flush_sheet_mutation(owner, session, dom);
    }

    #[test]
    fn collect_picks_up_written_back_style() {
        let (mut dom, root) = dom_with_root();
        let style = append_style(&mut dom, root, "div { color: red }");
        let mut session = SessionCore::new();

        let sheets = collect_document_stylesheets(root, &mut dom);
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].rules.len(), 1);

        write_back(
            style,
            "div { color: red } p { color: blue }",
            &mut session,
            &mut dom,
        );
        let sheets = collect_document_stylesheets(root, &mut dom);
        assert_eq!(sheets.len(), 1);
        assert_eq!(
            sheets[0].rules.len(),
            2,
            "the <style> write-back must be re-collected into cascade input"
        );
    }

    #[test]
    fn collect_picks_up_written_back_link() {
        let (mut dom, root) = dom_with_root();
        let link = append_loaded_link(&mut dom, root, "div { color: red }");
        let mut session = SessionCore::new();

        let sheets = collect_document_stylesheets(root, &mut dom);
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].rules.len(), 1);

        write_back(
            link,
            "div { color: red } p { color: blue }",
            &mut session,
            &mut dom,
        );
        let sheets = collect_document_stylesheets(root, &mut dom);
        assert_eq!(sheets.len(), 1);
        assert_eq!(
            sheets[0].rules.len(),
            2,
            "the LinkStylesheet write-back must be re-collected into cascade input"
        );
    }

    /// Idle-frame pin (E11 false-positive direction): a no-change collect
    /// does zero re-parse and zero stamp-key movement, and the stamp write
    /// itself (the raw `insert_one`) moves NO tree version — the collect
    /// cannot feed the very change signal that would re-trigger it.
    #[test]
    fn idle_collect_zero_reparse_zero_stamp_movement() {
        let (mut dom, root) = dom_with_root();
        let s1 = append_style(&mut dom, root, "div { color: red }");
        let s2 = append_loaded_link(&mut dom, root, "p { color: blue }");

        let root_before = dom.inclusive_descendants_version(root);
        let first = collect_document_stylesheets(root, &mut dom);
        assert_eq!(first.len(), 2);
        assert_eq!(
            dom.inclusive_descendants_version(root),
            root_before,
            "the stamp write is a raw component insert and must not bump the root version"
        );

        sentinel_swap_cached(&mut dom, &[s1, s2]);
        let idle = collect_document_stylesheets(root, &mut dom);
        assert_eq!(idle.len(), 2);
        assert!(
            idle.iter().all(|s| s.rules.is_empty()),
            "an idle collect must hit the cache (zero re-parse), got: {idle:?}"
        );
        for owner in [s1, s2] {
            assert_eq!(
                dom.world()
                    .get::<&CollectedStylesheet>(owner)
                    .expect("stamp survives an idle collect")
                    .version,
                sheet_version(owner, &dom),
                "zero stamp-key movement on the idle path"
            );
        }
    }

    /// Only the changed owner re-parses; the untouched owner stays cached.
    #[test]
    fn collect_reparses_only_the_changed_owner() {
        let (mut dom, root) = dom_with_root();
        let changed = append_style(&mut dom, root, "div { color: red }");
        let untouched = append_style(&mut dom, root, "p { color: blue }");

        let _ = collect_document_stylesheets(root, &mut dom);
        // Sentinel-swap both caches so a re-parse is observable per owner.
        sentinel_swap_cached(&mut dom, &[changed, untouched]);

        // Edit the first <style>'s text through the instrumented mutation
        // path (a script `textContent` write shape): its subtree version
        // moves, the sibling's does not.
        let text = dom.create_text("div { color: green } span { color: red }");
        assert!(dom.append_child(changed, text));

        let sheets = collect_document_stylesheets(root, &mut dom);
        assert_eq!(sheets.len(), 2);
        assert!(
            !sheets[0].rules.is_empty(),
            "the changed owner must re-parse"
        );
        assert!(
            sheets[1].rules.is_empty(),
            "the untouched owner must stay cached (sentinel returned, no re-parse)"
        );
    }

    /// Removal drop-out (I1): a script-removed `<style>` is a DETACH, not a
    /// despawn — the entity stays live and keeps its stamp, but the
    /// connectedness filter (the tree walk from `document`) drops it from
    /// the cascade input.
    #[test]
    fn detached_style_drops_out_of_collection() {
        let (mut dom, root) = dom_with_root();
        let removed = append_style(&mut dom, root, "div { color: red }");
        let _kept = append_style(&mut dom, root, "p { color: blue } em { color: blue }");

        assert_eq!(collect_document_stylesheets(root, &mut dom).len(), 2);

        assert!(dom.remove_child(root, removed));
        let sheets = collect_document_stylesheets(root, &mut dom);
        assert_eq!(sheets.len(), 1, "the detached owner drops out");
        // The survivor is `_kept`'s two-rule sheet, not the removed one-rule one.
        assert_eq!(sheets[0].rules.len(), 2);
        assert!(
            dom.world().get::<&CollectedStylesheet>(removed).is_ok(),
            "detach keeps the entity (and its stamp) alive — reclaimed only at teardown"
        );
    }

    /// I2 option-A pin (E11 false-negative direction): the `<link>` arm of
    /// `flush_sheet_mutation` must move the document-root
    /// `inclusive_descendants_version` (the change signal an async turn's
    /// render gating reads) WITHOUT dirtying the owner's re-collection
    /// compare key (the `LinkStylesheet` counter).
    #[test]
    fn link_flush_bumps_root_version_without_dirtying_compare() {
        let (mut dom, root) = dom_with_root();
        let link = append_loaded_link(&mut dom, root, "div { color: red }");
        let mut session = SessionCore::new();

        let _ = collect_document_stylesheets(root, &mut dom);
        let key_before = sheet_version(link, &dom);
        let root_before = dom.inclusive_descendants_version(root);

        write_back(
            link,
            "div { color: red } p { color: blue }",
            &mut session,
            &mut dom,
        );

        assert!(
            dom.inclusive_descendants_version(root) > root_before,
            "the link-arm write-back must move the root version delta"
        );
        assert_eq!(
            sheet_version(link, &dom),
            key_before + 1,
            "the compare key moves ONLY by the LinkStylesheet counter's own \
             increment — the added rev_version bump must not dirty it"
        );
    }
}
