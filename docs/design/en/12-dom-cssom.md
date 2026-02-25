
# 12. DOM API & CSSOM

The Object Model APIs (DOM and CSSOM) are the primary interfaces through which scripts interact with page content and styles. Both are built on the ScriptSession (Chapter 13), which handles identity mapping, mutation buffering, and GC coordination. The OM plugins focus purely on domain logic.

## 12.1 DOM API Architecture

The DOM API is the bridge between script engines and the ECS-based DOM store. All DOM operations go through the ScriptSession, which handles identity mapping and mutation buffering. DomApiHandler plugins focus purely on domain logic.

### 12.1.1 DomApiHandler Plugin Trait

Each DOM API method is implemented as a plugin conforming to the DomApiHandler trait, following the same dual dispatch pattern as all other elidex plugins:

```rust
#[elidex_plugin(dispatch = "static")]
pub trait DomApiHandler: Send + Sync {
    fn method_name(&self) -> &str;
    fn spec_level(&self) -> DomSpecLevel;
    fn invoke(
        &self,
        this: EntityId,
        args: &[JsValue],
        session: &mut dyn ScriptSession,
        dom: &EcsDom,   // read-only; writes go through session.record_mutation()
    ) -> Result<JsValue>;
}

pub enum DomSpecLevel {
    Living,       // DOM Living Standard: querySelector, addEventListener, etc.
    Legacy,       // Legacy APIs: getElementsByClassName (live), attachEvent, etc.
    Deprecated,   // Dangerous: document.write, document.all, etc.
}
```

This means every DOM method is individually toggleable. elidex-app can exclude legacy DOM APIs at compile time, while elidex-browser includes them via the compat layer.

### 12.1.2 Core vs. Compat DOM APIs

| API | Core | Compat | Notes |
| --- | --- | --- | --- |
| querySelector / querySelectorAll | ✓ | — | Returns static NodeList (snapshot). Primary query API. |
| getElementById | ✓ | — | Direct ECS lookup by ID component. Fast path. |
| addEventListener / removeEventListener | ✓ | — | ECS EventTarget component. Standard event flow. |
| createElement / createTextNode | ✓ | — | Creates new ECS entity with appropriate components. |
| appendChild / insertBefore / removeChild | ✓ | — | Records Mutation::AppendChild etc. in session. Triggers re-layout on flush. |
| getAttribute / setAttribute | ✓ | — | Records Mutation::SetAttribute in session. |
| MutationObserver | ✓ | — | Session flush generates MutationRecords from buffered mutations. First-class. |
| classList / dataset | ✓ | — | Convenience APIs backed by ECS Attributes component. |
| innerHTML / outerHTML | ✓ | — | Serialization and fragment parsing. Widely used by frameworks. |
| getElementsByClassName (live HTMLCollection) | ✗ | ✓ | Live collections registered in session via register_live_query(). Re-evaluated on each flush. |
| document.write / document.writeln | ✗ | ✓ | Interrupts parser stream. Extremely disruptive to pipeline. Compat shim serializes to innerHTML. |
| document.all | ✗ | ✓ | Famous quirk: typeof document.all === "undefined" yet it exists. Compat only. |
| element.attachEvent / detachEvent | ✗ | ✓ | IE legacy. Shimmed to addEventListener. |

### 12.1.3 ECS Integration Patterns

With the ScriptSession, DOM API handlers read ECS state directly but write through the session's mutation buffer:

```rust
// querySelector → ECS query over TagType + Attributes (read-only, no session needed)
fn query_selector(root: EntityId, selector: &str, dom: &EcsDom) -> Option<EntityId> {
    let parsed = css_selector::parse(selector)?;
    dom.query::<(TreeRelation, TagType, Attributes)>()
        .descendants_of(root)
        .find(|(_, tag, attrs)| parsed.matches(tag, attrs))
}

// setAttribute → writes go through session mutation buffer
fn set_attribute(entity: EntityId, name: &str, value: &str, session: &mut dyn ScriptSession) {
    session.record_mutation(Mutation::SetAttribute(entity, name.into(), value.into()));
}

// element.style → Identity Map ensures same wrapper returned each time
fn get_style(entity: EntityId, session: &mut dyn ScriptSession) -> JsObjectRef {
    session.get_or_create_wrapper(entity, ComponentKind::InlineStyle)
}
```

### 12.1.4 Shadow DOM and Web Components

Shadow DOM is the foundation for Web Components and is actively used by modern web frameworks. The ECS can model shadow roots as a separate tree scope:

```rust
pub struct ShadowRoot {
    mode: ShadowRootMode,  // Open or Closed
    host: EntityId,         // Element that owns this shadow root
}

pub struct TreeRelation {
    parent: EntityId,
    first_child: Option<EntityId>,
    next_sibling: Option<EntityId>,
    shadow_root: Option<EntityId>,  // If this entity hosts a shadow tree
    tree_scope: TreeScope,           // Which scope this node belongs to
}
```

Shadow DOM support is important for elidex-app as well, since it provides component encapsulation for application UI. Implementation is scheduled for Phase 3 (Chapter 3).

## 12.2 CSSOM (CSS Object Model)

CSSOM provides script access to CSS rules and stylesheets. It shares the same structural challenge as DOM API—OOP wrappers over ECS data—and is built on the same ScriptSession infrastructure. Identity mapping, mutation buffering, and GC coordination are inherited from the session, so CSSOM plugins focus purely on CSS domain logic.

### 12.2.1 CssomApiHandler Plugin Trait

```rust
#[elidex_plugin(dispatch = "static")]
pub trait CssomApiHandler: Send + Sync {
    fn method_name(&self) -> &str;
    fn spec_level(&self) -> CssomSpecLevel;
    fn invoke(
        &self,
        this: EntityId,
        args: &[JsValue],
        session: &mut dyn ScriptSession,
        dom: &EcsDom,
    ) -> Result<JsValue>;
}

pub enum CssomSpecLevel {
    Living,       // CSSOM Living Standard (all current APIs)
    // Future: Legacy, Deprecated — architecture ready when needed
}
```

### 12.2.2 CSSOM API Coverage

| API | Core | Notes |
| --- | --- | --- |
| element.style | ✓ | CSSStyleDeclaration for inline styles. Session Identity Map ensures `el.style === el.style`. Writes record Mutation::SetInlineStyle in session buffer. |
| window.getComputedStyle() | ✓ | Read-only CSSStyleDeclaration. Reads from ECS ComputedStyle component. Live: automatically reflects style recalculations after session flush. |
| document.styleSheets | ✓ | StyleSheetList. Each CSSStyleSheet is an ECS entity. Session Identity Map provides stable wrappers. |
| CSSStyleSheet.insertRule() / deleteRule() | ✓ | Records Mutation::InsertCssRule / DeleteCssRule in session buffer. Style recalculation triggered on flush. |
| CSS.supports() | ✓ | Query against PluginRegistry. Returns true if a CssPropertyHandler exists for the given property/value. |
| CSSStyleSheet() constructor | ✓ | Constructable Stylesheets. Creates new ECS entity. Foundation for Shadow DOM styling. |
| element.computedStyleMap() (CSS Typed OM) | ✓ | CSSNumericValue-based typed access. No string parsing overhead. P2 priority. |
| element.style.cssText | ✓ | Bulk write. Records multiple Mutation::SetInlineStyle entries in session buffer. |
| getComputedStyle().getPropertyValue() | ✓ | Per-property read. Dispatches to CssPropertyHandler plugin via PluginRegistry. |

CSSOM is currently all core with no compat APIs. Unlike DOM, which accumulated decades of legacy APIs (document.all, attachEvent, live collections), CSSOM is relatively young and clean. However, the CssomSpecLevel enum includes the architectural provision for Legacy/Deprecated tiers when they become necessary—for example, if IE-era APIs like `element.currentStyle` or `element.runtimeStyle` are needed for compat.

### 12.2.3 ECS Model for Stylesheets

Stylesheets are stored in the ECS as entities, allowing the same session-mediated pattern used for DOM nodes:

```rust
pub struct StyleSheetData {
    owner: StyleSheetOwner,       // <link>, <style>, or constructable
    rules: Vec<CssRuleEntity>,    // Each rule is also an entity
    disabled: bool,
    media: MediaList,
}

pub enum StyleSheetOwner {
    LinkElement(EntityId),        // <link rel="stylesheet">
    StyleElement(EntityId),       // <style>
    Constructed,                  // new CSSStyleSheet()
}
```

When script modifies a stylesheet via CSSOM (e.g., `sheet.insertRule()`), the mutation flows through the session buffer. On flush, the StyleSystem is notified that affected stylesheets changed, triggering targeted style recalculation only for elements that match the modified rules.
