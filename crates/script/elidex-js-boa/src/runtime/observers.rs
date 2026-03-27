//! Observer delivery methods for `JsRuntime`.
//!
//! Handles `MutationObserver`, `ResizeObserver`, `IntersectionObserver`,
//! and `MediaQueryList` change notification delivery to JS callbacks.

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsValue};

use elidex_ecs::{EcsDom, Entity};
use elidex_script_session::SessionCore;

use super::{JsRuntime, UnbindGuard};

impl JsRuntime {
    /// Deliver mutation records to all `MutationObserver` callbacks.
    ///
    /// Feeds session-level `MutationRecord`s to the observer registries,
    /// then invokes JS callbacks for observers with pending records.
    pub fn deliver_mutation_records(
        &mut self,
        records: &[elidex_script_session::MutationRecord],
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        // Feed records to the registry.
        for record in records {
            self.bridge.with_mutation_observers(|reg| {
                reg.notify(record, &|target, ancestor| {
                    // Walk up the tree from target to check if ancestor is an ancestor.
                    let mut current = dom.get_parent(target);
                    while let Some(node) = current {
                        if node == ancestor {
                            return true;
                        }
                        current = dom.get_parent(node);
                    }
                    false
                });
            });
        }

        // Collect observer IDs with pending records.
        let observer_ids: Vec<u64> = self.bridge.with_mutation_observers(|reg| {
            reg.observers_with_records()
                .map(elidex_api_observers::mutation::MutationObserverId::raw)
                .collect()
        });

        if observer_ids.is_empty() {
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        for observer_id in observer_ids {
            let mo_id = elidex_api_observers::mutation::MutationObserverId::from_raw(observer_id);
            let records = self
                .bridge
                .with_mutation_observers(|reg| reg.take_records(mo_id));
            if records.is_empty() {
                continue;
            }

            let Some(callback) = self.bridge.get_observer_callback(observer_id) else {
                continue;
            };
            let observer_obj = self
                .bridge
                .get_observer_object(observer_id)
                .map_or(JsValue::undefined(), JsValue::from);

            let arr = boa_engine::object::builtins::JsArray::new(&mut self.ctx);
            for record in &records {
                let obj = crate::globals::observers::mutation_record_to_js(record, &mut self.ctx);
                let _ = arr.push(obj, &mut self.ctx);
            }

            if let Err(err) = callback.call(
                &observer_obj,
                &[JsValue::from(arr), observer_obj.clone()],
                &mut self.ctx,
            ) {
                eprintln!("[JS MutationObserver Error] {err}");
            }
        }

        if let Err(err) = self.ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }

    /// Deliver resize observations to all `ResizeObserver` callbacks.
    ///
    /// Compares current element sizes against last known sizes and invokes
    /// callbacks for observers with changed targets.
    pub fn deliver_resize_observations(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        let observations = self.bridge.with_resize_observers(|reg| {
            reg.gather_observations(&|entity| {
                let lb = dom.world().get::<&elidex_plugin::LayoutBox>(entity).ok()?;
                let bb = lb.border_box();
                Some((lb.content.size, bb.size))
            })
        });

        if observations.is_empty() {
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        for (observer_id_typed, entries) in &observations {
            let observer_id = observer_id_typed.raw();
            let Some(callback) = self.bridge.get_observer_callback(observer_id) else {
                continue;
            };
            let observer_obj = self
                .bridge
                .get_observer_object(observer_id)
                .map_or(JsValue::undefined(), JsValue::from);

            let arr = boa_engine::object::builtins::JsArray::new(&mut self.ctx);
            for entry in entries {
                let obj = resize_entry_to_js(entry, &mut self.ctx);
                let _ = arr.push(obj, &mut self.ctx);
            }

            if let Err(err) = callback.call(
                &observer_obj,
                &[JsValue::from(arr), observer_obj.clone()],
                &mut self.ctx,
            ) {
                eprintln!("[JS ResizeObserver Error] {err}");
            }
        }

        if let Err(err) = self.ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }

    /// Deliver intersection observations to all `IntersectionObserver` callbacks.
    pub fn deliver_intersection_observations(
        &mut self,
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
        viewport: elidex_plugin::Rect,
    ) {
        let observations = self.bridge.with_intersection_observers(|reg| {
            reg.gather_observations(
                &|entity| {
                    let lb = dom.world().get::<&elidex_plugin::LayoutBox>(entity).ok()?;
                    let bb = lb.border_box();
                    Some(elidex_plugin::Rect::new(
                        lb.content.origin.x,
                        lb.content.origin.y,
                        bb.size.width,
                        bb.size.height,
                    ))
                },
                viewport,
            )
        });

        if observations.is_empty() {
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        for (observer_id_typed, entries) in &observations {
            let observer_id = observer_id_typed.raw();
            let Some(callback) = self.bridge.get_observer_callback(observer_id) else {
                continue;
            };
            let observer_obj = self
                .bridge
                .get_observer_object(observer_id)
                .map_or(JsValue::undefined(), JsValue::from);

            let arr = boa_engine::object::builtins::JsArray::new(&mut self.ctx);
            for entry in entries {
                let obj = intersection_entry_to_js(entry, &mut self.ctx);
                let _ = arr.push(obj, &mut self.ctx);
            }

            if let Err(err) = callback.call(
                &observer_obj,
                &[JsValue::from(arr), observer_obj.clone()],
                &mut self.ctx,
            ) {
                eprintln!("[JS IntersectionObserver Error] {err}");
            }
        }

        if let Err(err) = self.ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }

    /// Dispatch "change" events to `MediaQueryList` listeners whose result changed.
    ///
    /// `changed` is a list of `(media_query_id, new_matches)` pairs returned
    /// by `HostBridge::re_evaluate_media_queries()`.
    pub fn deliver_media_query_changes(
        &mut self,
        changed: &[(u64, bool)],
        session: &mut SessionCore,
        dom: &mut EcsDom,
        document_entity: Entity,
    ) {
        if changed.is_empty() {
            return;
        }

        self.bridge.bind(session, dom, document_entity);
        let _guard = UnbindGuard(&self.bridge);

        for &(id, new_matches) in changed {
            let listeners = self.bridge.media_query_listeners(id);
            if listeners.is_empty() {
                continue;
            }
            let media = self.bridge.media_query_string(id).unwrap_or_default();

            // Build a MediaQueryListEvent-like object.
            let event = ObjectInitializer::new(&mut self.ctx)
                .property(
                    js_string!("matches"),
                    JsValue::from(new_matches),
                    Attribute::READONLY,
                )
                .property(
                    js_string!("media"),
                    JsValue::from(js_string!(media.as_str())),
                    Attribute::READONLY,
                )
                .build();
            let event_val = JsValue::from(event);

            // Build a MediaQueryList-like object to use as `this` per spec.
            // Note: This creates a fresh object rather than reusing the original
            // MQL returned by matchMedia(). The `matches` and `media` properties
            // are correct, but `this !== original_mql` for identity checks.
            // TODO: Store MQL JS objects in bridge for identity preservation.
            let mql_this = ObjectInitializer::new(&mut self.ctx)
                .property(
                    js_string!("matches"),
                    JsValue::from(new_matches),
                    Attribute::READONLY,
                )
                .property(
                    js_string!("media"),
                    JsValue::from(js_string!(media.as_str())),
                    Attribute::READONLY,
                )
                .build();
            let this_val = JsValue::from(mql_this);

            for listener in &listeners {
                if let Err(err) =
                    listener.call(&this_val, std::slice::from_ref(&event_val), &mut self.ctx)
                {
                    eprintln!("[JS MediaQueryList Error] {err}");
                }
            }
        }

        if let Err(err) = self.ctx.run_jobs() {
            eprintln!("[JS Microtask Error] {err}");
        }
    }
}

fn resize_entry_to_js(
    entry: &elidex_api_observers::resize::ResizeObserverEntry,
    ctx: &mut Context,
) -> JsValue {
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("target"),
            JsValue::from(entry.target.to_bits().get() as f64),
            Attribute::all(),
        )
        .property(
            js_string!("contentBoxWidth"),
            JsValue::from(f64::from(entry.content_box_size.width)),
            Attribute::all(),
        )
        .property(
            js_string!("contentBoxHeight"),
            JsValue::from(f64::from(entry.content_box_size.height)),
            Attribute::all(),
        )
        .property(
            js_string!("borderBoxWidth"),
            JsValue::from(f64::from(entry.border_box_size.width)),
            Attribute::all(),
        )
        .property(
            js_string!("borderBoxHeight"),
            JsValue::from(f64::from(entry.border_box_size.height)),
            Attribute::all(),
        )
        .build();
    JsValue::from(obj)
}

fn intersection_entry_to_js(
    entry: &elidex_api_observers::intersection::IntersectionObserverEntry,
    ctx: &mut Context,
) -> JsValue {
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("target"),
            JsValue::from(entry.target.to_bits().get() as f64),
            Attribute::all(),
        )
        .property(
            js_string!("intersectionRatio"),
            JsValue::from(entry.intersection_ratio),
            Attribute::all(),
        )
        .property(
            js_string!("isIntersecting"),
            JsValue::from(entry.is_intersecting),
            Attribute::all(),
        )
        .build();
    JsValue::from(obj)
}
