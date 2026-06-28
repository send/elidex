//! `textContent` (NodeKind-aware) and `nodeValue` handlers.

use elidex_ecs::{CommentData, EcsDom, Entity, NodeKind, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{
    apply_replace_all, apply_replace_data, DomApiError, DomApiHandler, SessionCore,
};

use crate::char_data::{get_char_data, utf16_len};

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
            Some(NodeKind::Text | NodeKind::CdataSection | NodeKind::Comment) => {
                // WHATWG DOM §4.4 `Node.textContent` setter, CharacterData case
                // (`#dom-node-textcontent`): "Replace data with node, offset 0,
                // count node's length, and the given value" — identical to
                // `setData`. Route through the record-producing `apply_replace_data`
                // (fires the §4.10 characterData MutationRecord + ReplaceData
                // live-range event for Text/CDATASection) so `text.textContent = …`
                // / `comment.textContent = …` are observable, matching the
                // CharacterData methods. `apply_replace_data` returns `None` for a
                // malformed entity lacking both `TextContent` and `CommentData`
                // (legacy world.spawn paths) — a silent no-op, no record, matching
                // the prior browser-aligned behaviour.
                let old_data = get_char_data(this, dom).unwrap_or_default();
                if let Some(record) =
                    apply_replace_data(dom, this, 0, utf16_len(&old_data), &text, old_data)
                {
                    session.push_notify_record(record);
                }
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
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError> {
        let value = crate::util::require_string_arg(args, 0)?;

        // WHATWG DOM §4.4 `Node.nodeValue` setter, CharacterData case:
        // "Replace data with node, offset 0, count node's length, and the given
        // value" — identical to `setData` / the textContent setter. Route through
        // `apply_replace_data` so `text.nodeValue = …` / `comment.nodeValue = …`
        // produce characterData MutationRecords. Element / Document / DocumentType /
        // DocumentFragment = no-op (§4.4 nodeValue setter null branch).
        if matches!(
            dom.node_kind(this),
            Some(NodeKind::Text | NodeKind::CdataSection | NodeKind::Comment)
        ) {
            let old_data = get_char_data(this, dom).unwrap_or_default();
            if let Some(record) =
                apply_replace_data(dom, this, 0, utf16_len(&old_data), &value, old_data)
            {
                session.push_notify_record(record);
            }
        }

        Ok(JsValue::Undefined)
    }
}
