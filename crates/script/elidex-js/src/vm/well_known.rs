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

    // -- Event constructor globals --
    // `Event` / `CustomEvent` are the global names bound to their
    // respective constructable functions; `detail` is a property key
    // on CustomEvent instances (CustomEventInit dict member + accessor).
    pub(crate) event_global: StringId,
    pub(crate) custom_event_global: StringId,
    pub(crate) detail: StringId,

    // -- Specialized Event constructor globals (UIEvent family) --
    // WebIDL names for the UIEvent family.  Each binds to a
    // constructable function installed during `register_globals` and
    // chains through `UIEvent.prototype → Event.prototype`.
    pub(crate) ui_event_global: StringId,
    pub(crate) mouse_event_global: StringId,
    pub(crate) keyboard_event_global: StringId,
    pub(crate) focus_event_global: StringId,
    pub(crate) input_event_global: StringId,

    // -- UIEvent / MouseEvent / KeyboardEvent init-dict keys --
    // `view`, plus MouseEvent-specific constructor keys beyond the
    // UA-dispatch Mouse payload set.  `detail` is reused from
    // CustomEvent above; `location` is reused from Window.location.
    pub(crate) view: StringId,
    pub(crate) screen_x: StringId,
    pub(crate) screen_y: StringId,
    pub(crate) movement_x: StringId,
    pub(crate) movement_y: StringId,

    // -- Non-UIEvent specialized constructor globals --
    // PromiseRejectionEvent / ErrorEvent / HashChangeEvent /
    // PopStateEvent all chain directly to `Event.prototype` (not
    // UIEvent — they're sibling subclasses of Event).
    pub(crate) promise_rejection_event_global: StringId,
    pub(crate) error_event_global: StringId,
    pub(crate) hash_change_event_global: StringId,
    pub(crate) pop_state_event_global: StringId,

    // -- ErrorEvent / PopStateEvent init-dict keys --
    // `message` / `error` / `reason` / `promise` / `old_url` /
    // `new_url` already exist elsewhere and are reused here.
    pub(crate) filename: StringId,
    pub(crate) lineno: StringId,
    pub(crate) colno: StringId,
    pub(crate) state: StringId,

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
    // DocumentType.prototype attrs (PR4f C7).
    pub(crate) public_id: StringId,
    pub(crate) system_id: StringId,
    // Document collections + stubs (PR4f C7).
    pub(crate) cookie: StringId,
    pub(crate) referrer: StringId,
    pub(crate) forms: StringId,
    pub(crate) images: StringId,
    pub(crate) links: StringId,
    // HTMLElement.prototype method + accessor names (PR5b §C1).
    // `focus` / `blur` install as methods; `activeElement` /
    // `hasFocus` install as document accessor + method
    // respectively.  Pre-interned here so install sites and
    // receiver brand-check helpers share a single StringId.
    pub(crate) focus: StringId,
    pub(crate) blur: StringId,
    // HTMLElement IDL attribute names (PR5b §C2).  Each covers the
    // IDL property identifier; the matching HTML content-attribute
    // name is sometimes identical (lang / title / dir / hidden /
    // nonce / translate / spellcheck / autofocus) and sometimes a
    // case-folded variant (`accessKey` ↔ `accesskey`, `tabIndex` ↔
    // `tabindex`, `inputMode` ↔ `inputmode`, `enterKeyHint` ↔
    // `enterkeyhint`, `contentEditable` ↔ `contenteditable`).
    // WHATWG §3.2.8 / §6.6 / §6.7.  `isContentEditable` is a
    // readonly derived accessor with no backing content attribute.
    pub(crate) access_key: StringId,
    pub(crate) tab_index: StringId,
    pub(crate) draggable: StringId,
    pub(crate) hidden: StringId,
    pub(crate) lang: StringId,
    pub(crate) dir: StringId,
    pub(crate) title: StringId,
    pub(crate) translate: StringId,
    pub(crate) spellcheck: StringId,
    pub(crate) autocapitalize: StringId,
    pub(crate) input_mode: StringId,
    pub(crate) enter_key_hint: StringId,
    pub(crate) nonce: StringId,
    pub(crate) content_editable: StringId,
    pub(crate) is_content_editable: StringId,
    pub(crate) autofocus: StringId,
    // HTMLCollection / NodeList IDL members (PR5b §C3).
    pub(crate) item: StringId,
    pub(crate) named_item: StringId,
    // NamedNodeMap + Attr IDL members (PR5b §C4 + §C4.5).
    pub(crate) attributes: StringId,
    pub(crate) get_attribute_node: StringId,
    pub(crate) set_attribute_node: StringId,
    pub(crate) remove_attribute_node: StringId,
    pub(crate) get_named_item: StringId,
    pub(crate) set_named_item: StringId,
    pub(crate) remove_named_item: StringId,
    pub(crate) get_named_item_ns: StringId,
    pub(crate) set_named_item_ns: StringId,
    pub(crate) remove_named_item_ns: StringId,
    pub(crate) owner_element: StringId,
    pub(crate) namespace_uri: StringId,
    pub(crate) local_name: StringId,
    pub(crate) prefix: StringId,
    pub(crate) specified: StringId,
    // HTMLIFrameElement.prototype property names (PR4f C8).
    pub(crate) src: StringId,
    pub(crate) srcdoc: StringId,
    pub(crate) referrer_policy: StringId,
    pub(crate) allow: StringId,
    pub(crate) width: StringId,
    pub(crate) height: StringId,
    pub(crate) loading: StringId,
    pub(crate) sandbox: StringId,
    pub(crate) allow_fullscreen: StringId,
    pub(crate) content_document: StringId,
    pub(crate) content_window: StringId,
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

    // -- DOMException --
    // The constructor identifier, WebIDL member names, and the
    // subset of WHATWG §2.1 spec exception names currently used.
    // Additional names land at their first use site (interning is
    // deduplicating, so a later `well_known` entry is a
    // HashMap-hit fallback for hot paths).
    pub(crate) dom_exception: StringId,
    pub(crate) dom_exc_syntax_error: StringId,
    pub(crate) dom_exc_hierarchy_request_error: StringId,
    pub(crate) dom_exc_not_found_error: StringId,
    pub(crate) dom_exc_wrong_document_error: StringId,
    pub(crate) dom_exc_invalid_state_error: StringId,
    pub(crate) dom_exc_timeout_error: StringId,
    pub(crate) dom_exc_data_clone_error: StringId,

    // -- Headers (WHATWG Fetch §5.2) --
    // Method and iteration-helper names.  `get` / `set` / `append` /
    // `value` / `key` already live elsewhere and are reused here.
    // `delete` is an ES keyword — field named `delete_str` to
    // sidestep the `r#delete` raw-identifier contortion.
    pub(crate) headers_global: StringId,
    pub(crate) headers: StringId,
    pub(crate) delete_str: StringId,
    pub(crate) has: StringId,
    pub(crate) get_set_cookie: StringId,
    pub(crate) for_each: StringId,
    pub(crate) entries: StringId,
    pub(crate) keys: StringId,
    pub(crate) values: StringId,
    pub(crate) set_cookie_header: StringId,

    // -- Request / Response (WHATWG Fetch §5.3 / §5.5) --
    // Globals + IDL attr names + static factory method names +
    // enum strings.  `url` / `headers` already live above and are
    // reused here.
    pub(crate) request: StringId,
    pub(crate) response: StringId,
    pub(crate) method: StringId,
    pub(crate) body: StringId,
    pub(crate) body_used: StringId,
    pub(crate) ok: StringId,
    pub(crate) status_text: StringId,
    pub(crate) redirected: StringId,
    pub(crate) redirect: StringId,
    pub(crate) mode: StringId,
    pub(crate) credentials: StringId,
    pub(crate) cache: StringId,
    pub(crate) clone: StringId,
    pub(crate) json: StringId,
    // Method / response-type string constants — pre-interned so
    // per-instance `.type` / `.method` accessor reads return the
    // pool entry directly.
    pub(crate) http_get: StringId,
    pub(crate) http_head: StringId,
    pub(crate) http_post: StringId,
    pub(crate) http_put: StringId,
    pub(crate) http_delete: StringId,
    pub(crate) http_options: StringId,
    pub(crate) http_patch: StringId,
    pub(crate) http_connect: StringId,
    pub(crate) http_trace: StringId,
    pub(crate) content_type: StringId,
    pub(crate) application_json_utf8: StringId,
    pub(crate) text_plain_charset_utf8: StringId,
    pub(crate) response_type_default: StringId,
    pub(crate) response_type_basic: StringId,
    pub(crate) response_type_cors: StringId,
    pub(crate) response_type_error: StringId,
    pub(crate) response_type_opaque: StringId,
    pub(crate) response_type_opaqueredirect: StringId,

    // -- ArrayBuffer / Blob / Body mixin (ES2020 §24.1 / File API
    // §3 / WHATWG Fetch §5 Body mixin) --
    // Constructor global names are separate fields from the
    // camelCase attribute / method names to sidestep the name
    // collision the Headers ctor ran into (`"Headers"` ctor name
    // vs `"headers"` attr name).
    pub(crate) array_buffer_global: StringId,
    pub(crate) blob_global: StringId,
    pub(crate) byte_length: StringId,
    pub(crate) size: StringId,
    pub(crate) slice: StringId,
    pub(crate) text: StringId,
    pub(crate) array_buffer: StringId,

    // -- TypedArray + DataView (ES2024 §23.2 / §25.3) --
    // Constructor name StringIds.  `data_view_global` and the 11
    // concrete-subclass entries back the real globals; the abstract
    // `%TypedArray%` intrinsic has no globalThis binding per
    // §23.2.2 so no StringId is needed for it.  All are interned
    // eagerly so `register_typed_array_subclass` can fetch them
    // without a per-call `strings.intern(...)` round-trip.
    pub(crate) data_view_global: StringId,
    pub(crate) int8_array_global: StringId,
    pub(crate) uint8_array_global: StringId,
    pub(crate) uint8_clamped_array_global: StringId,
    pub(crate) int16_array_global: StringId,
    pub(crate) uint16_array_global: StringId,
    pub(crate) int32_array_global: StringId,
    pub(crate) uint32_array_global: StringId,
    pub(crate) float32_array_global: StringId,
    pub(crate) float64_array_global: StringId,
    pub(crate) bigint64_array_global: StringId,
    pub(crate) biguint64_array_global: StringId,
    pub(crate) buffer: StringId,
    pub(crate) byte_offset: StringId,
    pub(crate) bytes_per_element: StringId,

    // -- TextEncoder / TextDecoder (WHATWG Encoding §8) --
    // Constructor globals use the `*_global` suffix convention.
    // Method / attribute names are separate fields: `encoding`
    // doubles as the `TextDecoder.prototype.encoding` getter name
    // and an options-bag key, so the StringId is shared.
    pub(crate) text_encoder_global: StringId,
    pub(crate) text_decoder_global: StringId,
    pub(crate) encode: StringId,
    pub(crate) encode_into: StringId,
    pub(crate) decode: StringId,
    pub(crate) encoding: StringId,
    pub(crate) fatal: StringId,
    pub(crate) ignore_bom: StringId,
    pub(crate) read: StringId,
    pub(crate) written: StringId,
    pub(crate) stream: StringId,
    pub(crate) utf_8: StringId,
    pub(crate) utf_16le: StringId,
    pub(crate) utf_16be: StringId,
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

            // Event constructor globals.
            event_global: strings.intern("Event"),
            custom_event_global: strings.intern("CustomEvent"),
            detail: strings.intern("detail"),

            // Specialized Event constructor globals (UIEvent family).
            ui_event_global: strings.intern("UIEvent"),
            mouse_event_global: strings.intern("MouseEvent"),
            keyboard_event_global: strings.intern("KeyboardEvent"),
            focus_event_global: strings.intern("FocusEvent"),
            input_event_global: strings.intern("InputEvent"),

            // UIEvent / MouseEvent init-dict keys.
            view: strings.intern("view"),
            screen_x: strings.intern("screenX"),
            screen_y: strings.intern("screenY"),
            movement_x: strings.intern("movementX"),
            movement_y: strings.intern("movementY"),

            // Non-UIEvent specialized Event constructor globals.
            promise_rejection_event_global: strings.intern("PromiseRejectionEvent"),
            error_event_global: strings.intern("ErrorEvent"),
            hash_change_event_global: strings.intern("HashChangeEvent"),
            pop_state_event_global: strings.intern("PopStateEvent"),

            // ErrorEvent / PopStateEvent init-dict keys.
            filename: strings.intern("filename"),
            lineno: strings.intern("lineno"),
            colno: strings.intern("colno"),
            state: strings.intern("state"),

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
            public_id: strings.intern("publicId"),
            system_id: strings.intern("systemId"),
            cookie: strings.intern("cookie"),
            referrer: strings.intern("referrer"),
            forms: strings.intern("forms"),
            images: strings.intern("images"),
            links: strings.intern("links"),
            focus: strings.intern("focus"),
            blur: strings.intern("blur"),
            access_key: strings.intern("accessKey"),
            tab_index: strings.intern("tabIndex"),
            draggable: strings.intern("draggable"),
            hidden: strings.intern("hidden"),
            lang: strings.intern("lang"),
            dir: strings.intern("dir"),
            title: strings.intern("title"),
            translate: strings.intern("translate"),
            spellcheck: strings.intern("spellcheck"),
            autocapitalize: strings.intern("autocapitalize"),
            input_mode: strings.intern("inputMode"),
            enter_key_hint: strings.intern("enterKeyHint"),
            nonce: strings.intern("nonce"),
            content_editable: strings.intern("contentEditable"),
            is_content_editable: strings.intern("isContentEditable"),
            autofocus: strings.intern("autofocus"),
            item: strings.intern("item"),
            named_item: strings.intern("namedItem"),
            attributes: strings.intern("attributes"),
            get_attribute_node: strings.intern("getAttributeNode"),
            set_attribute_node: strings.intern("setAttributeNode"),
            remove_attribute_node: strings.intern("removeAttributeNode"),
            get_named_item: strings.intern("getNamedItem"),
            set_named_item: strings.intern("setNamedItem"),
            remove_named_item: strings.intern("removeNamedItem"),
            get_named_item_ns: strings.intern("getNamedItemNS"),
            set_named_item_ns: strings.intern("setNamedItemNS"),
            remove_named_item_ns: strings.intern("removeNamedItemNS"),
            owner_element: strings.intern("ownerElement"),
            namespace_uri: strings.intern("namespaceURI"),
            local_name: strings.intern("localName"),
            prefix: strings.intern("prefix"),
            specified: strings.intern("specified"),
            src: strings.intern("src"),
            srcdoc: strings.intern("srcdoc"),
            referrer_policy: strings.intern("referrerPolicy"),
            allow: strings.intern("allow"),
            width: strings.intern("width"),
            height: strings.intern("height"),
            loading: strings.intern("loading"),
            sandbox: strings.intern("sandbox"),
            allow_fullscreen: strings.intern("allowFullscreen"),
            content_document: strings.intern("contentDocument"),
            content_window: strings.intern("contentWindow"),
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

            dom_exception: strings.intern("DOMException"),
            dom_exc_syntax_error: strings.intern("SyntaxError"),
            dom_exc_hierarchy_request_error: strings.intern("HierarchyRequestError"),
            dom_exc_not_found_error: strings.intern("NotFoundError"),
            dom_exc_wrong_document_error: strings.intern("WrongDocumentError"),
            dom_exc_invalid_state_error: strings.intern("InvalidStateError"),
            dom_exc_timeout_error: strings.intern("TimeoutError"),
            dom_exc_data_clone_error: strings.intern("DataCloneError"),

            // Headers (WHATWG Fetch §5.2).
            headers_global: strings.intern("Headers"),
            headers: strings.intern("headers"),
            delete_str: strings.intern("delete"),
            has: strings.intern("has"),
            get_set_cookie: strings.intern("getSetCookie"),
            for_each: strings.intern("forEach"),
            entries: strings.intern("entries"),
            keys: strings.intern("keys"),
            values: strings.intern("values"),
            set_cookie_header: strings.intern("set-cookie"),

            // Request / Response (WHATWG Fetch §5.3 / §5.5).
            request: strings.intern("Request"),
            response: strings.intern("Response"),
            method: strings.intern("method"),
            body: strings.intern("body"),
            body_used: strings.intern("bodyUsed"),
            ok: strings.intern("ok"),
            status_text: strings.intern("statusText"),
            redirected: strings.intern("redirected"),
            redirect: strings.intern("redirect"),
            mode: strings.intern("mode"),
            credentials: strings.intern("credentials"),
            cache: strings.intern("cache"),
            clone: strings.intern("clone"),
            json: strings.intern("json"),
            http_get: strings.intern("GET"),
            http_head: strings.intern("HEAD"),
            http_post: strings.intern("POST"),
            http_put: strings.intern("PUT"),
            http_delete: strings.intern("DELETE"),
            http_options: strings.intern("OPTIONS"),
            http_patch: strings.intern("PATCH"),
            http_connect: strings.intern("CONNECT"),
            http_trace: strings.intern("TRACE"),
            content_type: strings.intern("content-type"),
            application_json_utf8: strings.intern("application/json"),
            text_plain_charset_utf8: strings.intern("text/plain;charset=UTF-8"),
            response_type_default: strings.intern("default"),
            response_type_basic: strings.intern("basic"),
            response_type_cors: strings.intern("cors"),
            response_type_error: strings.intern("error"),
            response_type_opaque: strings.intern("opaque"),
            response_type_opaqueredirect: strings.intern("opaqueredirect"),

            // ArrayBuffer / Blob / Body mixin.
            array_buffer_global: strings.intern("ArrayBuffer"),
            blob_global: strings.intern("Blob"),
            byte_length: strings.intern("byteLength"),
            size: strings.intern("size"),
            slice: strings.intern("slice"),
            text: strings.intern("text"),
            array_buffer: strings.intern("arrayBuffer"),

            // TypedArray + DataView.
            data_view_global: strings.intern("DataView"),
            int8_array_global: strings.intern("Int8Array"),
            uint8_array_global: strings.intern("Uint8Array"),
            uint8_clamped_array_global: strings.intern("Uint8ClampedArray"),
            int16_array_global: strings.intern("Int16Array"),
            uint16_array_global: strings.intern("Uint16Array"),
            int32_array_global: strings.intern("Int32Array"),
            uint32_array_global: strings.intern("Uint32Array"),
            float32_array_global: strings.intern("Float32Array"),
            float64_array_global: strings.intern("Float64Array"),
            bigint64_array_global: strings.intern("BigInt64Array"),
            biguint64_array_global: strings.intern("BigUint64Array"),
            buffer: strings.intern("buffer"),
            byte_offset: strings.intern("byteOffset"),
            bytes_per_element: strings.intern("BYTES_PER_ELEMENT"),

            // TextEncoder / TextDecoder.
            text_encoder_global: strings.intern("TextEncoder"),
            text_decoder_global: strings.intern("TextDecoder"),
            encode: strings.intern("encode"),
            encode_into: strings.intern("encodeInto"),
            decode: strings.intern("decode"),
            encoding: strings.intern("encoding"),
            fatal: strings.intern("fatal"),
            ignore_bom: strings.intern("ignoreBOM"),
            read: strings.intern("read"),
            written: strings.intern("written"),
            stream: strings.intern("stream"),
            utf_8: strings.intern("utf-8"),
            utf_16le: strings.intern("utf-16le"),
            utf_16be: strings.intern("utf-16be"),
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
