//! Accessibility tree builder.
//!
//! Converts an ECS DOM tree into an AccessKit `TreeUpdate` by walking
//! the DOM in pre-order and creating AccessKit nodes for each element.

use accesskit::{Node, NodeId, Role, Tree, TreeId, TreeUpdate};
use elidex_ecs::Entity;
use elidex_ecs::{Attributes, EcsDom, TextContent, MAX_ANCESTOR_DEPTH};
use elidex_plugin::LayoutBox;

use crate::names::compute_accessible_name;
use crate::roles::{aria_role_from_str, heading_level, tag_to_role};

/// Convert a `hecs::Entity` to an AccessKit `NodeId`.
///
/// Uses the entity's bits as a `u64` identifier.
#[must_use]
pub fn entity_to_node_id(entity: Entity) -> NodeId {
    NodeId(entity.to_bits().get())
}

/// Sentinel node ID for the accessibility tree root.
///
/// Zero is safe because hecs entities are `NonZeroU64`, so no entity
/// can produce `NodeId(0)`.
const TREE_ROOT_ID: u64 = 0;

/// Maximum recursion depth for DOM tree walks (imported from `elidex-ecs`).
const MAX_WALK_DEPTH: usize = MAX_ANCESTOR_DEPTH;

/// Build a complete `TreeUpdate` from the ECS DOM.
///
/// Walks the DOM starting from `document` in pre-order, skipping
/// elements with `aria-hidden="true"`.
///
/// `focus_entity` is the currently focused entity (if any), used to
/// set the `focus` field on the `TreeUpdate`.
#[must_use]
pub fn build_tree_update(
    dom: &EcsDom,
    document: Entity,
    focus_entity: Option<Entity>,
) -> TreeUpdate {
    let root_node_id = NodeId(TREE_ROOT_ID);
    let mut nodes: Vec<(NodeId, Node)> = Vec::new();

    // Collect top-level children of the document (composed for shadow DOM).
    let mut top_children = Vec::new();
    for c in dom.composed_children(document) {
        if !is_hidden(dom, c) {
            walk_dom(dom, c, &mut nodes, 0, false);
            top_children.push(entity_to_node_id(c));
        }
    }

    // Create the root document node.
    let mut root_node = Node::new(Role::Document);
    root_node.set_children(top_children);
    root_node.set_label("document");
    nodes.push((root_node_id, root_node));

    let focus = focus_entity.map_or(root_node_id, entity_to_node_id);

    TreeUpdate {
        nodes,
        tree: Some(Tree::new(root_node_id)),
        tree_id: TreeId::ROOT,
        focus,
    }
}

/// Recursively walk the DOM and build AccessKit nodes.
///
/// `ancestor_hidden` is `true` if any ancestor (including across shadow
/// boundaries) has `aria-hidden="true"`. This ensures shadow trees of
/// hidden hosts are excluded from the accessibility tree.
fn walk_dom(
    dom: &EcsDom,
    entity: Entity,
    nodes: &mut Vec<(NodeId, Node)>,
    depth: usize,
    ancestor_hidden: bool,
) {
    if depth > MAX_WALK_DEPTH {
        return;
    }
    if ancestor_hidden {
        return;
    }
    let node_id = entity_to_node_id(entity);
    let world = dom.world();

    // Check if this is a text node.
    if let Ok(text) = world.get::<&TextContent>(entity) {
        let trimmed = text.0.trim();
        if !trimmed.is_empty() {
            let mut node = Node::new(Role::TextRun);
            node.set_label(trimmed.to_string());
            nodes.push((node_id, node));
        }
        return;
    }

    // Element node — keep the Ref alive to avoid allocating a String.
    let tag_component = world.get::<&elidex_ecs::TagType>(entity).ok();
    let tag = tag_component.as_ref().map_or("", |t| t.0.as_str());

    let role = determine_role(dom, entity, tag);
    let mut node = Node::new(role);

    // Set heading level if applicable.
    if let Some(level) = heading_level(tag) {
        node.set_level(level);
    }

    // Set accessible name.
    if let Some(name) = compute_accessible_name(dom, entity) {
        node.set_label(name);
    }

    // Set bounds from LayoutBox if available.
    if let Ok(lb) = world.get::<&LayoutBox>(entity) {
        let border = lb.border_box();
        node.set_bounds(accesskit::Rect {
            x0: f64::from(border.x),
            y0: f64::from(border.y),
            x1: f64::from(border.x + border.width),
            y1: f64::from(border.y + border.height),
        });
    }

    // Collect children (composed for shadow DOM support).
    // Propagate hidden state: if this entity is hidden, all descendants
    // (including shadow tree children) are excluded from the a11y tree.
    let self_hidden = is_hidden(dom, entity);
    let mut children = Vec::new();
    for c in dom.composed_children(entity) {
        if !is_hidden(dom, c) {
            walk_dom(dom, c, nodes, depth + 1, self_hidden);
            children.push(entity_to_node_id(c));
        }
    }
    if !children.is_empty() {
        node.set_children(children);
    }

    nodes.push((node_id, node));
}

/// Determine the AccessKit Role for an element.
///
/// Checks for ARIA `role` attribute override, context-dependent roles
/// per HTML-AAM, then falls back to the tag-based mapping.
fn determine_role(dom: &EcsDom, entity: Entity, tag: &str) -> Role {
    if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
        // Check for explicit ARIA role.
        if let Some(role_str) = attrs.get("role") {
            if let Some(role) = aria_role_from_str(role_str) {
                return role;
            }
        }

        // img with empty alt is presentational (GenericContainer = ARIA none/presentation).
        // WAI-ARIA 1.2 §7.3: presentational role conflict resolution — if the element
        // has global ARIA naming attributes (aria-label, aria-labelledby), the
        // presentational role is overridden and the native Image role is used.
        if tag == "img" {
            if let Some(alt) = attrs.get("alt") {
                if alt.is_empty()
                    && !attrs.contains("aria-label")
                    && !attrs.contains("aria-labelledby")
                {
                    return Role::GenericContainer;
                }
            }
        }

        // a without href is generic.
        if tag == "a" && !attrs.contains("href") {
            return Role::GenericContainer;
        }
    }

    // Context-dependent roles per HTML-AAM.
    match tag {
        // <header>: Banner when scoped to body, GenericContainer inside sectioning content.
        "header" => {
            if is_sectioning_content_descendant(dom, entity) {
                return Role::GenericContainer;
            }
            return Role::Banner;
        }
        // <footer>: ContentInfo when scoped to body, GenericContainer inside sectioning content.
        "footer" => {
            if is_sectioning_content_descendant(dom, entity) {
                return Role::GenericContainer;
            }
            return Role::ContentInfo;
        }
        // <section>: Region only if it has an accessible name, otherwise GenericContainer.
        "section" => {
            if has_accessible_name(dom, entity) {
                return Role::Region;
            }
            return Role::GenericContainer;
        }
        // <form>: Form only if it has an accessible name, otherwise GenericContainer.
        "form" => {
            if has_accessible_name(dom, entity) {
                return Role::Form;
            }
            return Role::GenericContainer;
        }
        _ => {}
    }

    tag_to_role(tag)
}

/// Check if any ancestor is a sectioning content element (article, aside, main, nav, section).
///
/// L9: Crosses shadow boundaries by following `ShadowRoot.host` when the
/// parent walk reaches a shadow root, ensuring correct context detection
/// for elements inside shadow trees.
///
/// Depth-limited to [`MAX_WALK_DEPTH`] to guard against tree corruption,
/// matching other ancestor walks in the codebase.
fn is_sectioning_content_descendant(dom: &EcsDom, entity: Entity) -> bool {
    let mut parent = dom.get_parent(entity);
    let mut depth = 0;
    while let Some(p) = parent {
        if depth > MAX_WALK_DEPTH {
            break;
        }
        // L9: If we hit a ShadowRoot, continue via its host element.
        if let Ok(sr) = dom.world().get::<&elidex_ecs::ShadowRoot>(p) {
            parent = Some(sr.host);
            depth += 1;
            continue;
        }
        let tag = dom.world().get::<&elidex_ecs::TagType>(p).ok();
        let tag_str = tag.as_ref().map_or("", |t| t.0.as_str());
        if matches!(tag_str, "article" | "aside" | "main" | "nav" | "section") {
            return true;
        }
        parent = dom.get_parent(p);
        depth += 1;
    }
    false
}

/// Check if an element has an accessible name via `aria-label`, `aria-labelledby`, or `title`.
///
/// Used for context-dependent role determination (e.g. `<section>` → Region only
/// if it has an accessible name per HTML-AAM). Checks the ACCNAME labelling
/// attributes, resolving `aria-labelledby` ID references.
fn has_accessible_name(dom: &EcsDom, entity: Entity) -> bool {
    if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
        if let Some(label) = attrs.get("aria-label") {
            if !label.trim().is_empty() {
                return true;
            }
        }
        if let Some(labelledby) = attrs.get("aria-labelledby") {
            if crate::names::resolve_labelledby(dom, labelledby).is_some() {
                return true;
            }
        }
        if let Some(title) = attrs.get("title") {
            if !title.trim().is_empty() {
                return true;
            }
        }
    }
    false
}

/// Check if an element should be hidden from the accessibility tree.
fn is_hidden(dom: &EcsDom, entity: Entity) -> bool {
    if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
        if attrs.get("aria-hidden") == Some("true") {
            return true;
        }
        // HTML `hidden` attribute makes element not rendered and thus hidden.
        if attrs.contains("hidden") {
            return true;
        }
    }

    // Check display:none from ComputedStyle.
    if let Ok(style) = dom.world().get::<&elidex_plugin::ComputedStyle>(entity) {
        if style.display == elidex_plugin::Display::None {
            return true;
        }
    }

    false
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use elidex_ecs::Attributes;

    fn setup_simple_dom() -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        (dom, root)
    }

    /// Find a node in the `TreeUpdate` by entity, panicking if not found.
    fn find_node(update: &TreeUpdate, entity: Entity) -> &(NodeId, Node) {
        let id = entity_to_node_id(entity);
        update
            .nodes
            .iter()
            .find(|(nid, _)| *nid == id)
            .expect("node not found in tree update")
    }

    #[test]
    fn empty_document() {
        let (dom, root) = setup_simple_dom();
        let update = build_tree_update(&dom, root, None);
        assert_eq!(update.nodes.len(), 1);
        assert!(update.tree.is_some());
    }

    #[test]
    fn single_element() {
        let (mut dom, root) = setup_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(root, div);

        let update = build_tree_update(&dom, root, None);
        assert_eq!(update.nodes.len(), 2);
    }

    #[test]
    fn text_node_creates_text_run() {
        let (mut dom, root) = setup_simple_dom();
        let p = dom.create_element("p", Attributes::default());
        dom.append_child(root, p);
        let text = dom.create_text("Hello world");
        dom.append_child(p, text);

        let update = build_tree_update(&dom, root, None);
        assert_eq!(update.nodes.len(), 3);
        let text_node = find_node(&update, text);
        assert_eq!(text_node.1.role(), Role::TextRun);
    }

    #[test]
    fn heading_has_level() {
        let (mut dom, root) = setup_simple_dom();
        let h2 = dom.create_element("h2", Attributes::default());
        dom.append_child(root, h2);
        let text = dom.create_text("Section Title");
        dom.append_child(h2, text);

        let update = build_tree_update(&dom, root, None);
        let h2_node = find_node(&update, h2);
        assert_eq!(h2_node.1.role(), Role::Heading);
    }

    #[test]
    fn hidden_elements_skipped() {
        let cases = [
            ("aria-hidden=\"true\"", "aria-hidden", "true"),
            ("hidden attribute", "hidden", ""),
        ];

        for (desc, attr_name, attr_value) in cases {
            let (mut dom, root) = setup_simple_dom();
            let mut attrs = Attributes::default();
            attrs.set(attr_name, attr_value);
            let hidden = dom.create_element("div", attrs);
            dom.append_child(root, hidden);

            let visible = dom.create_element("p", Attributes::default());
            dom.append_child(root, visible);

            let update = build_tree_update(&dom, root, None);
            // Root + visible p only (hidden div skipped).
            assert_eq!(update.nodes.len(), 2, "case: {desc}");
        }
    }

    #[test]
    fn display_none_hides_element() {
        use elidex_plugin::{ComputedStyle, Display};

        let (mut dom, root) = setup_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(root, div);

        // Attach a ComputedStyle with display:none.
        let style = ComputedStyle {
            display: Display::None,
            ..ComputedStyle::default()
        };
        dom.world_mut().insert_one(div, style).unwrap();

        let visible = dom.create_element("p", Attributes::default());
        dom.append_child(root, visible);

        let update = build_tree_update(&dom, root, None);
        // Root + visible p only (display:none div skipped).
        assert_eq!(update.nodes.len(), 2);
    }

    type RoleTestCase = (
        &'static str,
        &'static str,
        &'static [(&'static str, &'static str)],
        Role,
    );

    #[test]
    fn role_overrides() {
        // (description, tag, attrs, expected_role)
        let cases: &[RoleTestCase] = &[
            (
                "img empty alt is presentational",
                "img",
                &[("alt", "")],
                Role::GenericContainer,
            ),
            (
                "img empty alt + aria-label overrides presentational (WAI-ARIA §7.3)",
                "img",
                &[("alt", ""), ("aria-label", "Photo")],
                Role::Image,
            ),
            (
                "explicit ARIA role overrides tag",
                "div",
                &[("role", "navigation")],
                Role::Navigation,
            ),
            (
                "a without href is generic",
                "a",
                &[],
                Role::GenericContainer,
            ),
            (
                "a with href is Link",
                "a",
                &[("href", "https://example.com")],
                Role::Link,
            ),
        ];

        for &(desc, tag, attr_pairs, expected) in cases {
            let (mut dom, root) = setup_simple_dom();
            let mut attrs = Attributes::default();
            for &(k, v) in attr_pairs {
                attrs.set(k, v);
            }
            let el = dom.create_element(tag, attrs);
            dom.append_child(root, el);

            let update = build_tree_update(&dom, root, None);
            let node = find_node(&update, el);
            assert_eq!(node.1.role(), expected, "case: {desc}");
        }
    }

    #[test]
    fn focus_entity_sets_focus() {
        let (mut dom, root) = setup_simple_dom();
        let btn = dom.create_element("button", Attributes::default());
        dom.append_child(root, btn);

        let update = build_tree_update(&dom, root, Some(btn));
        assert_eq!(update.focus, entity_to_node_id(btn));
    }

    #[test]
    fn entity_to_node_id_is_deterministic() {
        let mut dom = EcsDom::new();
        let root = dom.create_document_root();
        let id1 = entity_to_node_id(root);
        let id2 = entity_to_node_id(root);
        assert_eq!(id1, id2);
    }

    type ContextRoleTestCase = (
        &'static str,
        Option<&'static str>,
        &'static str,
        &'static [(&'static str, &'static str)],
        Role,
    );

    #[test]
    fn context_dependent_roles() {
        // (description, parent_tag, element_tag, element_attrs, expected_role)
        let cases: &[ContextRoleTestCase] = &[
            (
                "header top-level is Banner",
                Some("body"),
                "header",
                &[],
                Role::Banner,
            ),
            (
                "header in article is GenericContainer",
                Some("article"),
                "header",
                &[],
                Role::GenericContainer,
            ),
            (
                "footer top-level is ContentInfo",
                Some("body"),
                "footer",
                &[],
                Role::ContentInfo,
            ),
            (
                "section with label is Region",
                None,
                "section",
                &[("aria-label", "Main content")],
                Role::Region,
            ),
            (
                "section without label is GenericContainer",
                None,
                "section",
                &[],
                Role::GenericContainer,
            ),
            (
                "form with label is Form",
                None,
                "form",
                &[("aria-label", "Login form")],
                Role::Form,
            ),
            (
                "form without label is GenericContainer",
                None,
                "form",
                &[],
                Role::GenericContainer,
            ),
            (
                "section with title is Region",
                None,
                "section",
                &[("title", "Important section")],
                Role::Region,
            ),
            (
                "form with title is Form",
                None,
                "form",
                &[("title", "Search form")],
                Role::Form,
            ),
            (
                "section with nonexistent labelledby is GenericContainer",
                None,
                "section",
                &[("aria-labelledby", "nonexistent-id")],
                Role::GenericContainer,
            ),
        ];

        for &(desc, parent_tag, tag, attr_pairs, expected) in cases {
            let (mut dom, root) = setup_simple_dom();
            let parent = if let Some(ptag) = parent_tag {
                let p = dom.create_element(ptag, Attributes::default());
                dom.append_child(root, p);
                p
            } else {
                root
            };
            let mut attrs = Attributes::default();
            for &(k, v) in attr_pairs {
                attrs.set(k, v);
            }
            let el = dom.create_element(tag, attrs);
            dom.append_child(parent, el);

            let update = build_tree_update(&dom, root, None);
            let node = find_node(&update, el);
            assert_eq!(node.1.role(), expected, "case: {desc}");
        }
    }

    // --- L9: Composed tree walk for sectioning content detection ---

    #[test]
    fn header_in_shadow_under_section_is_generic() {
        // <section> host → shadow tree → <header> should see <section> ancestor
        // via composed tree walk, making header GenericContainer.
        let (mut dom, root) = setup_simple_dom();
        let section = dom.create_element("section", Attributes::default());
        dom.append_child(root, section);
        let sr = dom
            .attach_shadow(section, elidex_ecs::ShadowRootMode::Open)
            .unwrap();
        let header = dom.create_element("header", Attributes::default());
        dom.append_child(sr, header);

        let update = build_tree_update(&dom, root, None);
        let node = find_node(&update, header);
        assert_eq!(
            node.1.role(),
            Role::GenericContainer,
            "header in shadow under section should be GenericContainer"
        );
    }

    #[test]
    fn header_in_shadow_under_div_is_banner() {
        // <div> host → shadow tree → <header> — no sectioning ancestor,
        // so header should be Banner.
        let (mut dom, root) = setup_simple_dom();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(root, div);
        let sr = dom
            .attach_shadow(div, elidex_ecs::ShadowRootMode::Open)
            .unwrap();
        let header = dom.create_element("header", Attributes::default());
        dom.append_child(sr, header);

        let update = build_tree_update(&dom, root, None);
        let node = find_node(&update, header);
        assert_eq!(
            node.1.role(),
            Role::Banner,
            "header in shadow under div should be Banner"
        );
    }
}
