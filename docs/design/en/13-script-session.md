
# 13. ScriptSession: Unified Script ↔ ECS Boundary

The scripting layers are the most complex part of a browser engine and the area where elidex's "cut legacy" philosophy has the greatest impact. The design applies the same three-tier pattern used throughout the engine: a clean core for modern standards, a pluggable compat layer for legacy, and the dual dispatch plugin system unifying both.

| Layer | Core (Modern) | Compat (Legacy) | Cut Boundary |
| --- | --- | --- | --- |
| HTML | HTML5 Living Standard | Deprecated tags → HTML5 | HTML5 spec |
| DOM API | DOM Living Standard | Live collections, document.write, legacy events | DOM Living Standard vs. legacy quirks |
| CSSOM | CSSOM Living Standard | (None currently; architecture ready for future compat) | CSSOM spec |
| ECMAScript | ES2020+ (let/const, arrow, async/await, modules, class) | Annex B semantics, var quirks, with, arguments.callee | ES2020 baseline + Annex B boundary |

Standard web APIs present an object-oriented view to scripts (Node → Element → HTMLElement inheritance, CSSStyleDeclaration, CSSStyleSheet hierarchy), while elidex's internals are data-oriented (ECS entity IDs + typed component arrays). This impedance mismatch appears in every Object Model API: DOM, CSSOM, and future OMs (Selection, Range, Performance, etc.).

Rather than solving this mismatch ad hoc in each OM layer, elidex introduces a unified ScriptSession that mediates all script ↔ ECS interactions, analogous to an ORM's Unit of Work / Session pattern (e.g., SQLAlchemy Session, JPA EntityManager).

## 13.1 Architecture

```
Script Engine (JS / Wasm)
       │
       ▼
  ScriptSession  ← Identity Map + Mutation Buffer + GC + Live Queries
       │
       ├── DomApiHandler plugins   (DOM operations, Chapter 12)
       ├── CssomApiHandler plugins (CSSOM operations, Chapter 12)
       ├── Future OM plugins       (Selection, Range, Performance, ...)
       │
       ▼
   ECS (Entity + Components)
```

The session provides five services that all OM layers share:

| Service | Problem Solved | Mechanism |
| --- | --- | --- |
| Identity Map | `el.style === el.style` must be true. Same (entity, component) must return the same JS wrapper object. | HashMap<(EntityId, ComponentKind), JsObjectRef>. Checked before creating new wrappers. |
| Mutation Buffer | Script mutations must not interleave with rendering. DOM and CSSOM changes within a single script task must be atomically visible. | Vec<Mutation> collects all changes during script execution. Flushed to ECS between script steps. |
| Flush | Batched application of buffered mutations to ECS components. Generates MutationObserver records. | `session.flush(dom)` applies all buffered mutations, diffs component state, emits MutationRecords. |
| Live Query Management | `getElementsByClassName` and similar live collections must reflect DOM changes automatically. | Registered live queries are re-evaluated after each flush. Snapshot queries (querySelectorAll) are not tracked. |
| GC Coordination | When JS garbage-collects a wrapper object, the Identity Map entry must be cleaned up. | Weak references or invoke-on-drop cleanup. Session.release(ref) removes the entry. |

## 13.2 ScriptSession Trait

```rust
pub trait ScriptSession {
    /// Identity Map: same (entity, component) always returns the same JS wrapper
    fn get_or_create_wrapper(
        &mut self,
        entity: EntityId,
        component: ComponentKind,
    ) -> JsObjectRef;

    /// Buffer a mutation (DOM or CSSOM) for later flush
    fn record_mutation(&mut self, mutation: Mutation);

    /// Flush: apply all buffered mutations to ECS, return MutationRecords
    fn flush(&mut self, dom: &mut EcsDom) -> Vec<MutationRecord>;

    /// Register a live query (e.g., getElementsByClassName result)
    fn register_live_query(&mut self, query: LiveQuery) -> LiveQueryHandle;

    /// GC notification: remove wrapper from Identity Map
    fn release(&mut self, js_ref: JsObjectRef);
}

pub enum Mutation {
    // DOM mutations
    SetAttribute(EntityId, String, String),
    AppendChild(EntityId, EntityId),
    RemoveChild(EntityId, EntityId),
    SetInnerHtml(EntityId, String),
    // CSSOM mutations
    SetInlineStyle(EntityId, String, String),    // entity, property, value
    InsertCssRule(EntityId, usize, String),       // stylesheet entity, index, rule text
    DeleteCssRule(EntityId, usize),               // stylesheet entity, index
    // Future OM mutations
    // SetSelection(...), etc.
}
```

## 13.3 Event Loop Integration

The event loop is the central sequencer that interleaves script execution with session flushes and rendering. The ScriptSession makes flush points explicit:

```rust
loop {
    // 1. Execute oldest macrotask from any task source:
    //    - Event handlers (click, input, keyboard)
    //    - setTimeout / setInterval callbacks
    //    - MessagePort / postMessage
    //    - Fetch response handlers
    let task = task_queue.pop();
    script_engine.eval(task);

    // 2. Drain all microtasks (Promise.then, MutationObserver callbacks,
    //    queueMicrotask)
    while let Some(microtask) = microtask_queue.pop() {
        script_engine.eval(microtask);
    }

    // 3. Flush session: apply all buffered DOM/CSSOM mutations to ECS
    let mutation_records = session.flush(&mut dom);
    // Deliver MutationObserver callbacks (may trigger more microtasks)
    deliver_mutation_observers(mutation_records);
    drain_microtasks();

    // 4. If a rendering opportunity: run requestAnimationFrame callbacks
    if vsync_ready() {
        for cb in animation_frame_callbacks.drain(..) {
            script_engine.eval(cb);
        }
        drain_microtasks();
        session.flush(&mut dom);  // flush rAF mutations
    }

    // 5. Render if needed
    if dom.has_pending_style_invalidations() {
        run_style_system();
        run_layout_system();
        let display_list = run_paint_system();
        compositor_channel.send(CompositorMsg::SubmitDisplayList(display_list));
    }

    // 6. If idle time remains: run requestIdleCallback callbacks
    if has_idle_time() {
        for cb in idle_callbacks.drain(..) {
            script_engine.eval(cb);
        }
    }
}
```

This loop shows the JS event loop semantics (steps 1–6). The full integrated Renderer event loop — including external event collection (IPC, I/O) and wait phases — is defined in Ch. 5, Section 5.4.2. This ordering is specified by the HTML Living Standard and must be implemented precisely. Mutations accumulate in the session buffer during script execution and are applied atomically to the ECS at well-defined points (steps 3 and 4), ensuring the rendering pipeline always sees a consistent state and mutation observers receive consistent, ordered records regardless of whether the mutation originated from DOM API or CSSOM.
