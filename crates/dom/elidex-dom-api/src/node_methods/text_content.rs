//! `textContent` (NodeKind-aware) and `nodeValue` handlers.

use elidex_ecs::{CommentData, EcsDom, Entity, NodeKind, TextContent};
use elidex_plugin::JsValue;
use elidex_script_session::{DomApiError, DomApiHandler, SessionCore};

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
                if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(this) {
                    text.clone_into(&mut tc.0);
                }
                dom.rev_version(this);
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
                let children = dom.children(this);
                for child in children {
                    session.release(child);
                    let _ = dom.remove_child(this, child);
                }
                if !text.is_empty() {
                    let text_node = dom.create_text(text);
                    let _ = dom.append_child(this, text_node);
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
                if let Ok(mut tc) = dom.world_mut().get::<&mut TextContent>(this) {
                    value.clone_into(&mut tc.0);
                }
                dom.rev_version(this);
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
