//! `textContent` (NodeKind-aware) and `nodeValue` handlers.

use elidex_ecs::{CommentData, EcsDom, Entity, NodeKind, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{apply_replace_all, DomApiError, DomApiHandler, SessionCore};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Collect concatenated text content from an entity and its descendants.
fn descendant_text_content(entity: Entity, dom: &EcsDom) -> String {
    crate::element::collect_text_content(entity, dom)
}

// ---------------------------------------------------------------------------
// 7. GetTextContentNodeKind
// ---------------------------------------------------------------------------

/// `node.textContent` getter — NodeKind-aware behavior.
pub struct GetTextContentNodeKind;

impl DomApiHandler for GetTextContentNodeKind {
    fn method_name(&self) -> &str {
        "textContent.get"
    }

    fn invoke(
        &self,
        this: Entity,
        _args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        match dom.node_kind(this) {
            Some(NodeKind::Document | NodeKind::DocumentType) => Ok(JsValue::Null),
            Some(NodeKind::Text | NodeKind::CdataSection) => {
                let text = dom
                    .world()
                    .get::<&TextContent>(this)
                    .map(|t| t.0.clone())
                    .unwrap_or_default();
                Ok(JsValue::String(text))
            }
            Some(NodeKind::Comment) => {
                let data = dom
                    .world()
                    .get::<&CommentData>(this)
                    .map(|c| c.0.clone())
                    .unwrap_or_default();
                Ok(JsValue::String(data))
            }
            _ => {
                let text = descendant_text_content(this, dom);
                Ok(JsValue::String(text))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8. SetTextContentNodeKind
// ---------------------------------------------------------------------------

/// `node.textContent` setter — NodeKind-aware behavior.
pub struct SetTextContentNodeKind;

impl DomApiHandler for SetTextContentNodeKind {
    fn method_name(&self) -> &str {
        "textContent.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let text = crate::util::require_string_arg(args, 0)?;

        match dom.node_kind(this) {
            Some(NodeKind::Document | NodeKind::DocumentType) => Ok(JsValue::Undefined),
            Some(NodeKind::Text | NodeKind::CdataSection) => {
                // `set_text_data` bumps `rev_version(this)` internally
                // and returns `None` only when the entity lacks a
                // `TextContent` component — a malformed `NodeKind::Text`
                // entity (e.g. legacy world.spawn paths). WHATWG §3.6
                // does not mandate an error for that case; we follow
                // major browsers in silently no-op'ing so misuse stays
                // visible-on-debug (via the absent text update) without
                // tearing down the JS frame.
                if dom.set_text_data(this, &text).is_none() {
                    // No-op: malformed entity, see comment above.
                }
                Ok(JsValue::Undefined)
            }
            Some(NodeKind::Comment) => {
                if let Ok(mut cd) = dom.world_mut().get::<&mut CommentData>(this) {
                    text.clone_into(&mut cd.0);
                }
                dom.rev_version(this);
                Ok(JsValue::Undefined)
            }
            _ => {
                // WHATWG DOM §4.4 `Node.textContent` setter, Element/DocumentFragment
                // (`#dom-node-textcontent`): node = null, or a new Text (data = value,
                // node document = this's) when value is non-empty; then "string replace
                // all with node within this" = the canonical record-producing
                // `apply_replace_all` (§4.2.3 `#concept-node-replace-all`) → ONE coalesced
                // ChildList record (removed = prior children, added = [text] | «», queued
                // iff non-empty).
                let node = if text.is_empty() {
                    None
                } else {
                    let owner = dom.owner_document(this);
                    Some(dom.create_text_with_owner(text, owner))
                };
                for record in apply_replace_all(dom, this, node) {
                    // No per-caller CSSOM-cache prune for removed `<style>`/`<link>`
                    // children: per CLAUDE.md's side-store rule an entity-keyed cache
                    // (`SessionCore::cssom_sheets`, a GC mark-roots input via
                    // `active_cssom_rule_ids`) must be pruned at the GC/despawn
                    // chokepoint, NOT per mutation-caller. removeChild /
                    // replaceChildren never pruned it either; reverting the R5
                    // per-caller `cssom_sheets.remove` here keeps every removal path
                    // uniform. The GC-owned prune is tracked by defer slot
                    // `#11-cssom-sheets-prune-at-removal-chokepoint`.
                    session.push_notify_record(record);
                }
                Ok(JsValue::Undefined)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 9. SetNodeValue
// ---------------------------------------------------------------------------

/// `node.nodeValue` setter.
pub struct SetNodeValue;

impl DomApiHandler for SetNodeValue {
    fn method_name(&self) -> &str {
        "nodeValue.set"
    }

    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        _session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = crate::util::require_string_arg(args, 0)?;

        match dom.node_kind(this) {
            Some(NodeKind::Text | NodeKind::CdataSection) => {
                // `set_text_data` bumps `rev_version(this)` internally.
                let _ = dom.set_text_data(this, &value);
            }
            Some(NodeKind::Comment) => {
                if let Ok(mut cd) = dom.world_mut().get::<&mut CommentData>(this) {
                    value.clone_into(&mut cd.0);
                }
                dom.rev_version(this);
            }
            _ => {
                // Element, Document, DocumentType, DocumentFragment — no-op.
            }
        }

        Ok(JsValue::Undefined)
    }
}
