//! Custom element reaction processing for `JsRuntime`.
//!
//! Handles draining and executing custom element lifecycle reactions
//! (upgrade, connected, disconnected, attributeChanged, adopted).

use boa_engine::{js_string, Context, JsNativeError, JsValue};

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;

use crate::bridge::HostBridge;
use crate::globals::element::core::entity_bits_as_f64;

use super::{is_connected_to_document, walk_subtree_for_upgrade, JsRuntime, UnbindGuard};

impl JsRuntime {
    /// Drain and execute all pending custom element reactions.
    ///
    /// Processes `Upgrade`, `Connected`, `Disconnected`, and `AttributeChanged`
    /// reactions by invoking the appropriate lifecycle callbacks on the JS
    /// constructor prototype. Iterates up to `MAX_CE_DRAIN_ITERATIONS` to
    /// handle reactions enqueued during callback execution.
    #[allow(clippy::too_many_lines)]
    pub(super) fn drain_custom_element_reactions(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        use elidex_custom_elements::{CEState, CustomElementReaction, CustomElementState};

        const MAX_CE_DRAIN_ITERATIONS: usize = 16;

        for iteration in 0..MAX_CE_DRAIN_ITERATIONS {
            let reactions = self.bridge.drain_ce_reactions();
            if reactions.is_empty() {
                break;
            }
            if iteration == MAX_CE_DRAIN_ITERATIONS - 1 {
                eprintln!(
                    "[CE] reaction drain hit max iterations ({MAX_CE_DRAIN_ITERATIONS}); \
                     some lifecycle callbacks may be deferred"
                );
            }

            self.bridge.bind(session, dom, document_entity);
            let _guard = UnbindGuard(&self.bridge);

            for reaction in reactions {
                match reaction {
                    CustomElementReaction::Upgrade(entity) => {
                        // Look up the definition name from the entity's CE state.
                        let def_name = self.bridge.with(|_session, dom| {
                            dom.world()
                                .get::<&CustomElementState>(entity)
                                .ok()
                                .map(|s| (s.definition_name.clone(), s.state))
                        });
                        let Some((name, current_state)) = def_name else {
                            continue;
                        };

                        // Guard: skip if already upgraded or failed (duplicate from
                        // pending queue + DOM walk).
                        if current_state == CEState::Custom || current_state == CEState::Failed {
                            continue;
                        }

                        let Some(constructor) = self.bridge.get_custom_element_constructor(&name)
                        else {
                            continue;
                        };

                        // Verify extends tag match for customized built-in elements
                        // before upgrading (WHATWG HTML §4.13.3). If the definition
                        // specifies an `extends` tag, the element's actual tag must match.
                        if let Some(ext) = self.bridge.ce_extends_tag(&name) {
                            let tag_matches = self.bridge.with(|_session, dom| {
                                dom.world()
                                    .get::<&elidex_ecs::TagType>(entity)
                                    .ok()
                                    .is_some_and(|tag| tag.0.eq_ignore_ascii_case(&ext))
                            });
                            if !tag_matches {
                                // Tag mismatch — skip upgrade, set Failed.
                                self.bridge.with(|_session, dom| {
                                    if let Ok(mut ce) =
                                        dom.world_mut().get::<&mut CustomElementState>(entity)
                                    {
                                        ce.state = CEState::Failed;
                                    }
                                });
                                continue;
                            }
                        }

                        // Set state to Precustomized during upgrade.
                        self.bridge.with(|_session, dom| {
                            if let Ok(mut ce) =
                                dom.world_mut().get::<&mut CustomElementState>(entity)
                            {
                                ce.state = CEState::Precustomized;
                            }
                        });

                        // Invoke with `new` semantics (class constructors require it).
                        // Note: constructor.construct() creates a new JS object that is NOT
                        // automatically linked to the ECS entity. The element wrapper is
                        // created separately via create_element_wrapper(). Full prototype
                        // chain integration (so `this` in constructor refers to the element)
                        // requires HTMLElement base class support, planned for M4-9 (JS engine).
                        let result = constructor.construct(&[], None, &mut self.ctx);

                        // Update state based on result.
                        let succeeded = result.is_ok();
                        let is_connected = self.bridge.with(|_session, dom| {
                            if let Ok(mut ce) =
                                dom.world_mut().get::<&mut CustomElementState>(entity)
                            {
                                if succeeded {
                                    ce.state = CEState::Custom;
                                } else {
                                    ce.state = CEState::Failed;
                                }
                            }
                            // Check if the element is connected (has a parent chain to doc).
                            if succeeded {
                                is_connected_to_document(entity, dom)
                            } else {
                                false
                            }
                        });

                        // After successful upgrade, fire attributeChangedCallback
                        // for any existing attributes in observedAttributes.
                        if succeeded {
                            let observed = self.bridge.ce_observed_attributes(&name);
                            if !observed.is_empty() {
                                self.bridge.with(|_session, dom| {
                                    if let Ok(attrs) =
                                        dom.world().get::<&elidex_ecs::Attributes>(entity)
                                    {
                                        for attr_name in &observed {
                                            if let Some(value) = attrs.get(attr_name) {
                                                self.bridge.enqueue_ce_reaction(
                                                    CustomElementReaction::AttributeChanged {
                                                        entity,
                                                        name: attr_name.clone(),
                                                        old_value: None,
                                                        new_value: Some(value.to_string()),
                                                    },
                                                );
                                            }
                                        }
                                    }
                                });
                            }
                        }

                        // If the element is already in a connected tree, fire connectedCallback.
                        if is_connected {
                            self.bridge
                                .enqueue_ce_reaction(CustomElementReaction::Connected(entity));
                        }

                        if let Err(err) = result {
                            eprintln!("[JS Custom Element Upgrade Error] {err}");
                        }
                    }
                    CustomElementReaction::Connected(entity) => {
                        invoke_ce_callback(
                            entity,
                            "connectedCallback",
                            &[],
                            &self.bridge,
                            &mut self.ctx,
                        );
                    }
                    CustomElementReaction::Disconnected(entity) => {
                        invoke_ce_callback(
                            entity,
                            "disconnectedCallback",
                            &[],
                            &self.bridge,
                            &mut self.ctx,
                        );
                    }
                    CustomElementReaction::AttributeChanged {
                        entity,
                        name,
                        old_value,
                        new_value,
                    } => {
                        let args = [
                            JsValue::from(js_string!(name.as_str())),
                            old_value
                                .as_ref()
                                .map_or(JsValue::null(), |v| JsValue::from(js_string!(v.as_str()))),
                            new_value
                                .as_ref()
                                .map_or(JsValue::null(), |v| JsValue::from(js_string!(v.as_str()))),
                        ];
                        invoke_ce_callback(
                            entity,
                            "attributeChangedCallback",
                            &args,
                            &self.bridge,
                            &mut self.ctx,
                        );
                    }
                    CustomElementReaction::Adopted {
                        entity,
                        old_document,
                        new_document,
                    } => {
                        // Note: oldDocument/newDocument passed as entity bits (f64). Full
                        // Document wrapper objects require M4-3.10 multi-document support.
                        let old_doc_val = JsValue::from(entity_bits_as_f64(old_document));
                        let new_doc_val = JsValue::from(entity_bits_as_f64(new_document));
                        invoke_ce_callback(
                            entity,
                            "adoptedCallback",
                            &[old_doc_val, new_doc_val],
                            &self.bridge,
                            &mut self.ctx,
                        );
                    }
                }
            }

            // Run microtask queue after processing reactions.
            if let Err(err) = self.ctx.run_jobs() {
                eprintln!("[JS Microtask Error] {err}");
            }

            // _guard dropped here, calls unbind().
        }
    }

    /// Public entry point to drain custom element reactions.
    ///
    /// Call this after `enqueue_ce_reactions_from_mutations()` to process the
    /// queued reactions outside of an `eval()` / `dispatch_event()` call.
    pub fn drain_custom_element_reactions_public(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        self.drain_custom_element_reactions(session, dom, document_entity);
    }

    /// Scan mutation records for custom element lifecycle reactions.
    ///
    /// For each `ChildList` record, checks added/removed nodes for CE entities
    /// and enqueues `Connected`/`Disconnected` reactions. For `Attribute` records,
    /// checks if the target is a CE with the attribute in `observedAttributes`
    /// and enqueues `AttributeChanged`.
    ///
    /// Call this after `session.flush()` and before observer delivery.
    pub fn enqueue_ce_reactions_from_mutations(
        &mut self,
        records: &[elidex_script_session::MutationRecord],
        dom: &EcsDom,
    ) {
        use elidex_custom_elements::{CEState, CustomElementReaction, CustomElementState};
        use elidex_script_session::MutationKind;

        for record in records {
            match record.kind {
                MutationKind::ChildList => {
                    // For "connected" reactions, only enqueue if the mutation
                    // target (parent) is in a connected tree. The added nodes
                    // are children of the target, so they share connectivity.
                    let target_connected = is_connected_to_document(record.target, dom);
                    for &entity in &record.added_nodes {
                        if target_connected {
                            crate::globals::element::core::enqueue_ce_reactions_for_subtree(
                                entity,
                                crate::globals::element::core::CeReactionKind::Connected,
                                &self.bridge,
                                dom,
                            );
                        }
                        // Upgrade undefined CEs regardless of connectivity —
                        // elements created via innerHTML in disconnected subtrees
                        // should still be upgraded when a definition exists.
                        walk_subtree_for_upgrade(entity, &self.bridge, dom, 0);
                    }
                    // For "disconnected" reactions, only fire if the parent
                    // (record.target) is still connected — meaning the removed
                    // child WAS connected before removal.
                    if target_connected {
                        for &entity in &record.removed_nodes {
                            crate::globals::element::core::enqueue_ce_reactions_for_subtree(
                                entity,
                                crate::globals::element::core::CeReactionKind::Disconnected,
                                &self.bridge,
                                dom,
                            );
                        }
                    }
                }
                MutationKind::Attribute => {
                    if let Some(ref attr_name) = record.attribute_name {
                        if let Ok(ce_state) = dom.world().get::<&CustomElementState>(record.target)
                        {
                            if ce_state.state == CEState::Custom
                                && self
                                    .bridge
                                    .ce_is_observed_attribute(&ce_state.definition_name, attr_name)
                            {
                                // Get the new value from the DOM.
                                let new_value = dom
                                    .world()
                                    .get::<&elidex_ecs::Attributes>(record.target)
                                    .ok()
                                    .and_then(|attrs| attrs.get(attr_name).map(String::from));

                                // Fire whenever a MutationRecord exists — the spec
                                // does not gate on old != new for attributeChangedCallback.
                                self.bridge.enqueue_ce_reaction(
                                    CustomElementReaction::AttributeChanged {
                                        entity: record.target,
                                        name: attr_name.clone(),
                                        old_value: record.old_value.clone(),
                                        new_value,
                                    },
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Invoke a lifecycle callback on a custom element's constructor prototype.
///
/// Free function to avoid borrow conflicts with `JsRuntime` methods.
fn invoke_ce_callback(
    entity: Entity,
    callback_name: &str,
    args: &[JsValue],
    bridge: &HostBridge,
    ctx: &mut Context,
) {
    use elidex_custom_elements::CustomElementState;

    let def_name = bridge.with(|_session, dom| {
        dom.world()
            .get::<&CustomElementState>(entity)
            .ok()
            .map(|s| s.definition_name.clone())
    });
    let Some(name) = def_name else { return };

    let Some(constructor) = bridge.get_custom_element_constructor(&name) else {
        return;
    };

    // Get the prototype and look up the callback method.
    let Ok(proto_val) = constructor.get(js_string!("prototype"), ctx) else {
        return;
    };
    let Some(proto_obj) = proto_val.as_object() else {
        return;
    };
    let Ok(cb_val) = proto_obj.get(js_string!(callback_name), ctx) else {
        return;
    };
    // Property is undefined/null — callback not defined, valid per spec.
    if cb_val.is_undefined() || cb_val.is_null() {
        return;
    }
    // Non-callable callback property: log TypeError (errors caught by caller).
    let Some(cb_func) = cb_val.as_callable() else {
        eprintln!(
            "[JS Custom Element {callback_name} Error] {}",
            JsNativeError::typ().with_message(format!("{callback_name} is not a function"))
        );
        return;
    };

    // Build element wrapper for `this`.
    let element_wrapper = bridge.with(|session, _dom| {
        let obj_ref =
            session.get_or_create_wrapper(entity, elidex_script_session::ComponentKind::Element);
        crate::globals::element::create_element_wrapper(entity, bridge, obj_ref, ctx)
    });

    if let Err(err) = cb_func.call(&element_wrapper, args, ctx) {
        eprintln!("[JS Custom Element {callback_name} Error] {err}");
    }
}
