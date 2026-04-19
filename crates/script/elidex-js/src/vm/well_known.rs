//! Pre-interned string IDs and well-known Symbol IDs, built once at
//! `Vm::new` time and consulted on every property operation,
//! identifier comparison, and event-object construction.
//!
//! Split out of `mod.rs` to keep that file under the project's
//! 1000-line convention.  The structs themselves are passive data
//! (fields are `Copy` newtypes) — the only behaviour here is the
//! `new` constructors that pre-intern each name on the crate's
//! `StringPool` / `SymbolRecord` table.

use super::pools::StringPool;
use super::value::{StringId, SymbolId, SymbolRecord};

/// Frequently used interned string IDs, cached at VM creation.
#[allow(dead_code)] // Fields used by interpreter and future built-ins.
pub(crate) struct WellKnownStrings {
    pub(crate) undefined: StringId,
    pub(crate) null: StringId,
    pub(crate) r#true: StringId,
    pub(crate) r#false: StringId,
    pub(crate) nan: StringId,
    pub(crate) infinity: StringId,
    pub(crate) neg_infinity: StringId,
    pub(crate) zero: StringId,
    pub(crate) empty: StringId,
    pub(crate) prototype: StringId,
    pub(crate) constructor: StringId,
    pub(crate) length: StringId,
    pub(crate) name: StringId,
    pub(crate) message: StringId,
    pub(crate) log: StringId,
    pub(crate) error: StringId,
    pub(crate) warn: StringId,
    pub(crate) object_type: StringId,
    pub(crate) boolean_type: StringId,
    pub(crate) number_type: StringId,
    pub(crate) string_type: StringId,
    pub(crate) function_type: StringId,
    pub(crate) symbol_type: StringId,
    pub(crate) bigint_type: StringId,
    pub(crate) object_to_string: StringId,
    pub(crate) next: StringId,
    pub(crate) value: StringId,
    pub(crate) done: StringId,
    pub(crate) return_str: StringId,
    pub(crate) last_index: StringId,
    pub(crate) index: StringId,
    pub(crate) input: StringId,
    pub(crate) join: StringId,
    pub(crate) to_json: StringId,
    pub(crate) get: StringId,
    pub(crate) set: StringId,
    pub(crate) enumerable: StringId,
    pub(crate) configurable: StringId,
    pub(crate) writable: StringId,
    pub(crate) source: StringId,
    pub(crate) flags: StringId,
    pub(crate) status: StringId,
    pub(crate) fulfilled: StringId,
    pub(crate) rejected: StringId,
    pub(crate) reason: StringId,
    pub(crate) errors: StringId,
    pub(crate) aggregate_error: StringId,

    // -- Event-dispatch identifiers (PR3) --
    // Property names installed on every event object — pre-interned
    // here to avoid a HashMap lookup per name per dispatch.  Listener
    // option keys (`capture`/`once`/`passive`) live here too since
    // every `addEventListener` with options-object form reads them.
    pub(crate) event_type: StringId,
    pub(crate) bubbles: StringId,
    pub(crate) cancelable: StringId,
    pub(crate) event_phase: StringId,
    pub(crate) target: StringId,
    pub(crate) current_target: StringId,
    pub(crate) time_stamp: StringId,
    pub(crate) composed: StringId,
    pub(crate) is_trusted: StringId,
    pub(crate) default_prevented: StringId,
    pub(crate) prevent_default: StringId,
    pub(crate) stop_propagation: StringId,
    pub(crate) stop_immediate_propagation: StringId,
    pub(crate) composed_path: StringId,
    pub(crate) capture: StringId,
    pub(crate) once: StringId,
    pub(crate) passive: StringId,
    pub(crate) document: StringId,
    pub(crate) window: StringId,
    pub(crate) navigator: StringId,
    pub(crate) performance: StringId,
    pub(crate) location: StringId,
    pub(crate) history: StringId,
    /// `document.readyState` constant — returned every read of the
    /// stub getter.  Pre-interning avoids a per-access StringPool
    /// allocation on what would otherwise be a HashMap lookup path.
    pub(crate) complete: StringId,
    /// `history.scrollRestoration` constant — same rationale as `complete`.
    pub(crate) auto: StringId,
    /// Pre-interned `"about:blank"` for callers that need the default
    /// blank-document URL as a `StringId` without repeating
    /// `strings.intern("about:blank")`.  `NavigationState::new` uses
    /// the `String` form directly; this field ensures the pool entry
    /// exists so that subsequent `intern` calls on the same literal
    /// are HashMap hits rather than fresh insertions.
    pub(crate) about_blank: StringId,
    pub(crate) unhandledrejection: StringId,
    pub(crate) promise: StringId,

    // -- Event payload property keys --
    // Pre-interned so `create_event_object`'s payload installation
    // can feed them directly into the precomputed-shape slot array
    // without per-dispatch `strings.intern(name)` calls.  Also used
    // by `VmInner::build_precomputed_event_shapes` to walk the
    // shape-transition chain once at `register_globals` time.
    //
    // Shared keys (used by multiple payload variants) are defined
    // once: `alt_key`, `ctrl_key`, `meta_key`, `shift_key`, `data`,
    // `code`, `elapsed_time`, `pseudo_element`, `key`.
    pub(crate) client_x: StringId,
    pub(crate) client_y: StringId,
    pub(crate) button: StringId,
    pub(crate) buttons: StringId,
    pub(crate) alt_key: StringId,
    pub(crate) ctrl_key: StringId,
    pub(crate) meta_key: StringId,
    pub(crate) shift_key: StringId,
    pub(crate) key: StringId,
    pub(crate) code: StringId,
    pub(crate) repeat: StringId,
    pub(crate) property_name: StringId,
    pub(crate) elapsed_time: StringId,
    pub(crate) pseudo_element: StringId,
    pub(crate) animation_name: StringId,
    pub(crate) input_type: StringId,
    pub(crate) data: StringId,
    pub(crate) is_composing: StringId,
    pub(crate) data_type: StringId,
    pub(crate) related_target: StringId,
    pub(crate) delta_x: StringId,
    pub(crate) delta_y: StringId,
    pub(crate) delta_mode: StringId,
    pub(crate) origin: StringId,
    pub(crate) last_event_id: StringId,
    pub(crate) was_clean: StringId,
    pub(crate) old_url: StringId,
    pub(crate) new_url: StringId,
    pub(crate) persisted: StringId,
    pub(crate) old_value: StringId,
    pub(crate) new_value: StringId,
    pub(crate) url: StringId,

    // -- Node / Element accessors --
    // Pre-interned here because every DOM access touches one of
    // them.  Keeping the `StringId`s in `WellKnownStrings` lets the
    // native getters reach them with a field read rather than a
    // per-call `strings.intern(...)`.
    pub(crate) parent_node: StringId,
    pub(crate) parent_element: StringId,
    pub(crate) first_child: StringId,
    pub(crate) last_child: StringId,
    pub(crate) next_sibling: StringId,
    pub(crate) previous_sibling: StringId,
    pub(crate) first_element_child: StringId,
    pub(crate) last_element_child: StringId,
    pub(crate) next_element_sibling: StringId,
    pub(crate) previous_element_sibling: StringId,
    pub(crate) child_nodes: StringId,
    pub(crate) children: StringId,
    pub(crate) child_element_count: StringId,
    pub(crate) has_child_nodes: StringId,
    pub(crate) is_connected: StringId,
    pub(crate) contains: StringId,
    pub(crate) node_type: StringId,
    pub(crate) node_name: StringId,
    pub(crate) node_value: StringId,
    pub(crate) text_content: StringId,
    pub(crate) tag_name: StringId,
    pub(crate) id: StringId,
    pub(crate) class_name: StringId,
    pub(crate) get_attribute: StringId,
    pub(crate) set_attribute: StringId,
    pub(crate) remove_attribute: StringId,
    pub(crate) has_attribute: StringId,
    pub(crate) get_attribute_names: StringId,
    pub(crate) toggle_attribute: StringId,
    pub(crate) append_child: StringId,
    pub(crate) remove_child: StringId,
    pub(crate) insert_before: StringId,
    pub(crate) replace_child: StringId,
    pub(crate) remove: StringId,
    pub(crate) matches: StringId,
    pub(crate) closest: StringId,
    pub(crate) query_selector: StringId,
    pub(crate) query_selector_all: StringId,
    pub(crate) clone_node: StringId,
    pub(crate) compare_document_position: StringId,
    pub(crate) is_same_node: StringId,
    pub(crate) is_equal_node: StringId,
    pub(crate) owner_document: StringId,
    pub(crate) get_root_node: StringId,
    pub(crate) normalize: StringId,
    // Element.prototype.insertAdjacent* (PR4f C4).
    pub(crate) insert_adjacent_element: StringId,
    pub(crate) insert_adjacent_text: StringId,
    pub(crate) beforebegin: StringId,
    pub(crate) afterbegin: StringId,
    pub(crate) beforeend: StringId,
    pub(crate) afterend: StringId,
    pub(crate) get_elements_by_tag_name: StringId,
    pub(crate) get_elements_by_class_name: StringId,
    // Document RO / RW accessors (PR4f C6 / C7).
    pub(crate) css1_compat: StringId,
    // CharacterData (PR4e C5) method names.  `data` / `length` live
    // elsewhere (`data` under event-payload keys above, `length`
    // under core).
    pub(crate) append_data: StringId,
    pub(crate) insert_data: StringId,
    pub(crate) delete_data: StringId,
    pub(crate) replace_data: StringId,
    pub(crate) substring_data: StringId,
    // Text.prototype (PR4e C5.5)
    pub(crate) split_text: StringId,
    // ChildNode / ParentNode mixins (PR4e C6 / C7)
    pub(crate) before: StringId,
    pub(crate) after: StringId,
    pub(crate) replace_with: StringId,
    pub(crate) prepend: StringId,
    pub(crate) append: StringId,
    pub(crate) replace_children: StringId,
    // `Node.prototype.nodeName` constants for non-Element nodes.
    // Pre-interned so `native_node_get_node_name` returns a cached
    // `StringId` without per-call allocation.
    pub(crate) hash_text: StringId,
    pub(crate) hash_comment: StringId,
    pub(crate) hash_document: StringId,
    pub(crate) hash_document_fragment: StringId,

    // -- AbortController / AbortSignal (PR4d) --
    // Method, accessor, and listener-option key names.  Pre-interned
    // here so `parse_listener_options` can look up `signal` without
    // re-interning per `addEventListener` call, and so the prototype
    // installer can assemble its property list without re-interning
    // on every VM construction.
    pub(crate) abort_controller: StringId,
    pub(crate) abort_signal: StringId,
    pub(crate) signal: StringId,
    pub(crate) aborted: StringId,
    pub(crate) abort: StringId,
    pub(crate) onabort: StringId,
    pub(crate) throw_if_aborted: StringId,
    pub(crate) abort_error: StringId,

    // EventTarget method names — referenced by both the
    // EventTarget.prototype installer and AbortSignal.prototype's
    // shadowing installer.  Pre-interning here means each prototype
    // installer hits an existing pool entry instead of allocating.
    pub(crate) add_event_listener: StringId,
    pub(crate) remove_event_listener: StringId,
    pub(crate) dispatch_event: StringId,
}

impl WellKnownStrings {
    /// Intern every well-known name on `strings` and return the
    /// populated table.  Must be the only caller to populate
    /// `WellKnownStrings` — the fields carry expectations (e.g.
    /// `empty` is literally `intern("")`) that other VM code relies
    /// on without re-checking.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn intern_all(strings: &mut StringPool) -> Self {
        Self {
            undefined: strings.intern("undefined"),
            null: strings.intern("null"),
            r#true: strings.intern("true"),
            r#false: strings.intern("false"),
            nan: strings.intern("NaN"),
            infinity: strings.intern("Infinity"),
            neg_infinity: strings.intern("-Infinity"),
            zero: strings.intern("0"),
            empty: strings.intern(""),
            prototype: strings.intern("prototype"),
            constructor: strings.intern("constructor"),
            length: strings.intern("length"),
            name: strings.intern("name"),
            message: strings.intern("message"),
            log: strings.intern("log"),
            error: strings.intern("error"),
            warn: strings.intern("warn"),
            object_type: strings.intern("object"),
            boolean_type: strings.intern("boolean"),
            number_type: strings.intern("number"),
            string_type: strings.intern("string"),
            function_type: strings.intern("function"),
            symbol_type: strings.intern("symbol"),
            bigint_type: strings.intern("bigint"),
            object_to_string: strings.intern("[object Object]"),
            next: strings.intern("next"),
            value: strings.intern("value"),
            done: strings.intern("done"),
            return_str: strings.intern("return"),
            last_index: strings.intern("lastIndex"),
            index: strings.intern("index"),
            input: strings.intern("input"),
            join: strings.intern("join"),
            to_json: strings.intern("toJSON"),
            get: strings.intern("get"),
            set: strings.intern("set"),
            enumerable: strings.intern("enumerable"),
            configurable: strings.intern("configurable"),
            writable: strings.intern("writable"),
            source: strings.intern("source"),
            flags: strings.intern("flags"),
            status: strings.intern("status"),
            fulfilled: strings.intern("fulfilled"),
            rejected: strings.intern("rejected"),
            reason: strings.intern("reason"),
            errors: strings.intern("errors"),
            aggregate_error: strings.intern("AggregateError"),
            event_type: strings.intern("type"),
            bubbles: strings.intern("bubbles"),
            cancelable: strings.intern("cancelable"),
            event_phase: strings.intern("eventPhase"),
            target: strings.intern("target"),
            current_target: strings.intern("currentTarget"),
            time_stamp: strings.intern("timeStamp"),
            composed: strings.intern("composed"),
            is_trusted: strings.intern("isTrusted"),
            default_prevented: strings.intern("defaultPrevented"),
            prevent_default: strings.intern("preventDefault"),
            stop_propagation: strings.intern("stopPropagation"),
            stop_immediate_propagation: strings.intern("stopImmediatePropagation"),
            composed_path: strings.intern("composedPath"),
            capture: strings.intern("capture"),
            once: strings.intern("once"),
            passive: strings.intern("passive"),
            document: strings.intern("document"),
            window: strings.intern("window"),
            navigator: strings.intern("navigator"),
            performance: strings.intern("performance"),
            location: strings.intern("location"),
            history: strings.intern("history"),
            complete: strings.intern("complete"),
            auto: strings.intern("auto"),
            about_blank: strings.intern("about:blank"),
            unhandledrejection: strings.intern("unhandledrejection"),
            promise: strings.intern("promise"),

            // Event-payload property keys.  Interned once here so
            // `create_event_object` can feed slots into
            // `define_with_precomputed_shape` without re-interning.
            client_x: strings.intern("clientX"),
            client_y: strings.intern("clientY"),
            button: strings.intern("button"),
            buttons: strings.intern("buttons"),
            alt_key: strings.intern("altKey"),
            ctrl_key: strings.intern("ctrlKey"),
            meta_key: strings.intern("metaKey"),
            shift_key: strings.intern("shiftKey"),
            key: strings.intern("key"),
            code: strings.intern("code"),
            repeat: strings.intern("repeat"),
            property_name: strings.intern("propertyName"),
            elapsed_time: strings.intern("elapsedTime"),
            pseudo_element: strings.intern("pseudoElement"),
            animation_name: strings.intern("animationName"),
            input_type: strings.intern("inputType"),
            data: strings.intern("data"),
            is_composing: strings.intern("isComposing"),
            data_type: strings.intern("dataType"),
            related_target: strings.intern("relatedTarget"),
            delta_x: strings.intern("deltaX"),
            delta_y: strings.intern("deltaY"),
            delta_mode: strings.intern("deltaMode"),
            origin: strings.intern("origin"),
            last_event_id: strings.intern("lastEventId"),
            was_clean: strings.intern("wasClean"),
            old_url: strings.intern("oldURL"),
            new_url: strings.intern("newURL"),
            persisted: strings.intern("persisted"),
            old_value: strings.intern("oldValue"),
            new_value: strings.intern("newValue"),
            url: strings.intern("url"),

            // Node / Element accessors.
            parent_node: strings.intern("parentNode"),
            parent_element: strings.intern("parentElement"),
            first_child: strings.intern("firstChild"),
            last_child: strings.intern("lastChild"),
            next_sibling: strings.intern("nextSibling"),
            previous_sibling: strings.intern("previousSibling"),
            first_element_child: strings.intern("firstElementChild"),
            last_element_child: strings.intern("lastElementChild"),
            next_element_sibling: strings.intern("nextElementSibling"),
            previous_element_sibling: strings.intern("previousElementSibling"),
            child_nodes: strings.intern("childNodes"),
            children: strings.intern("children"),
            child_element_count: strings.intern("childElementCount"),
            has_child_nodes: strings.intern("hasChildNodes"),
            is_connected: strings.intern("isConnected"),
            contains: strings.intern("contains"),
            node_type: strings.intern("nodeType"),
            node_name: strings.intern("nodeName"),
            node_value: strings.intern("nodeValue"),
            text_content: strings.intern("textContent"),
            tag_name: strings.intern("tagName"),
            id: strings.intern("id"),
            class_name: strings.intern("className"),
            get_attribute: strings.intern("getAttribute"),
            set_attribute: strings.intern("setAttribute"),
            remove_attribute: strings.intern("removeAttribute"),
            has_attribute: strings.intern("hasAttribute"),
            get_attribute_names: strings.intern("getAttributeNames"),
            toggle_attribute: strings.intern("toggleAttribute"),
            append_child: strings.intern("appendChild"),
            remove_child: strings.intern("removeChild"),
            insert_before: strings.intern("insertBefore"),
            replace_child: strings.intern("replaceChild"),
            remove: strings.intern("remove"),
            matches: strings.intern("matches"),
            closest: strings.intern("closest"),
            query_selector: strings.intern("querySelector"),
            query_selector_all: strings.intern("querySelectorAll"),
            clone_node: strings.intern("cloneNode"),
            compare_document_position: strings.intern("compareDocumentPosition"),
            is_same_node: strings.intern("isSameNode"),
            is_equal_node: strings.intern("isEqualNode"),
            owner_document: strings.intern("ownerDocument"),
            get_root_node: strings.intern("getRootNode"),
            normalize: strings.intern("normalize"),
            insert_adjacent_element: strings.intern("insertAdjacentElement"),
            insert_adjacent_text: strings.intern("insertAdjacentText"),
            beforebegin: strings.intern("beforebegin"),
            afterbegin: strings.intern("afterbegin"),
            beforeend: strings.intern("beforeend"),
            afterend: strings.intern("afterend"),
            get_elements_by_tag_name: strings.intern("getElementsByTagName"),
            get_elements_by_class_name: strings.intern("getElementsByClassName"),
            css1_compat: strings.intern("CSS1Compat"),
            append_data: strings.intern("appendData"),
            insert_data: strings.intern("insertData"),
            delete_data: strings.intern("deleteData"),
            replace_data: strings.intern("replaceData"),
            substring_data: strings.intern("substringData"),
            split_text: strings.intern("splitText"),
            before: strings.intern("before"),
            after: strings.intern("after"),
            replace_with: strings.intern("replaceWith"),
            prepend: strings.intern("prepend"),
            append: strings.intern("append"),
            replace_children: strings.intern("replaceChildren"),
            hash_text: strings.intern("#text"),
            hash_comment: strings.intern("#comment"),
            hash_document: strings.intern("#document"),
            hash_document_fragment: strings.intern("#document-fragment"),

            // AbortController / AbortSignal (PR4d).
            abort_controller: strings.intern("AbortController"),
            abort_signal: strings.intern("AbortSignal"),
            signal: strings.intern("signal"),
            aborted: strings.intern("aborted"),
            abort: strings.intern("abort"),
            onabort: strings.intern("onabort"),
            throw_if_aborted: strings.intern("throwIfAborted"),
            abort_error: strings.intern("AbortError"),
            add_event_listener: strings.intern("addEventListener"),
            remove_event_listener: strings.intern("removeEventListener"),
            dispatch_event: strings.intern("dispatchEvent"),
        }
    }
}

/// Well-known symbol IDs, allocated at VM creation.
#[allow(dead_code)]
pub(crate) struct WellKnownSymbols {
    pub(crate) iterator: SymbolId,
    pub(crate) async_iterator: SymbolId,
    pub(crate) has_instance: SymbolId,
    pub(crate) to_primitive: SymbolId,
    pub(crate) to_string_tag: SymbolId,
    pub(crate) species: SymbolId,
    pub(crate) is_concat_spreadable: SymbolId,
}

impl WellKnownSymbols {
    /// Allocate the 7 well-known Symbol records at fixed SymbolIds 0-6
    /// and return the populated table alongside the symbol Vec (which
    /// the caller stores on `VmInner.symbols`).  Returning the Vec
    /// avoids a second `SymbolId` round-trip during VM construction —
    /// the caller moves it straight into the field.
    pub(crate) fn alloc_all(strings: &mut StringPool) -> (Self, Vec<SymbolRecord>) {
        let mut symbols = Vec::new();
        let mut alloc = |desc: &str| -> SymbolId {
            let id = SymbolId(symbols.len() as u32);
            symbols.push(SymbolRecord {
                description: Some(strings.intern(desc)),
            });
            id
        };
        let well_known = Self {
            iterator: alloc("Symbol.iterator"),
            async_iterator: alloc("Symbol.asyncIterator"),
            has_instance: alloc("Symbol.hasInstance"),
            to_primitive: alloc("Symbol.toPrimitive"),
            to_string_tag: alloc("Symbol.toStringTag"),
            species: alloc("Symbol.species"),
            is_concat_spreadable: alloc("Symbol.isConcatSpreadable"),
        };
        (well_known, symbols)
    }
}
