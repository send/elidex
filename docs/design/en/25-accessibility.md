
# 25. Accessibility

## 25.1 Overview

Accessibility (a11y) ensures elidex content is usable with screen readers, switch devices, voice control, and other assistive technologies. The a11y system derives an accessibility tree from the ECS DOM and layout results, then exposes it to platform APIs via AccessKit.

```
ECS DOM (TreeRelation, TagType, Attributes, ComputedStyle, LayoutBox)
  │
  ▼
A11ySystem (ECS system)
  │  reads: semantic roles, ARIA attributes, text content, bounding rects
  │  writes: AccessibilityTree resource
  ▼
AccessKit adapter
  │  translates to platform API
  ▼
Platform (NSAccessibility / UI Automation / AT-SPI2)
  │
  ▼
Assistive technology (VoiceOver, NVDA, Orca)
```

## 25.2 Accessibility Tree

### 25.2.1 Tree Construction

The A11ySystem is an ECS system that constructs the accessibility tree by reading DOM components:

```rust
pub struct A11ySystem;

impl A11ySystem {
    pub fn build_tree(&self, world: &World) -> AccessibilityTree {
        let mut tree = AccessibilityTree::new();

        for (entity, tag, attrs, style, layout) in
            world.query::<(Entity, &TagType, &Attributes, &ComputedStyle, Option<&LayoutBox>)>()
        {
            // Skip elements hidden from a11y
            if attrs.get("aria-hidden") == Some("true") || style.display == Display::None {
                continue;
            }

            let role = self.compute_role(tag, attrs);
            let name = self.compute_accessible_name(entity, world, attrs);
            let bounds = layout.map(|l| l.bounding_rect());

            tree.add_node(AccessibilityNode {
                entity,
                role,
                name,
                description: attrs.get("aria-description").map(String::from),
                value: self.compute_value(tag, attrs),
                state: self.compute_state(attrs),
                bounds,
                children: children_of(world, entity).collect(),
                actions: self.compute_actions(tag, attrs),
            });
        }

        tree
    }
}
```

### 25.2.2 Role Mapping

Each HTML element plugin declares its default ARIA role:

| Element | Default Role | Notes |
| --- | --- | --- |
| `<button>` | button | |
| `<a href>` | link | Only with href attribute |
| `<input type="text">` | textbox | |
| `<input type="checkbox">` | checkbox | |
| `<img alt="...">` | img | `alt=""` → presentational (hidden) |
| `<nav>` | navigation | Landmark |
| `<main>` | main | Landmark |
| `<h1>`–`<h6>` | heading | With aria-level |
| `<table>` | table | |
| `<ul>`, `<ol>` | list | |
| `<li>` | listitem | |

Explicit ARIA roles (`role="..."`) override default roles. Invalid role values are ignored.

### 25.2.3 Accessible Name Computation

Following the Accessible Name and Description Computation (ACCNAME) algorithm:

1. `aria-labelledby` → concatenate text of referenced elements
2. `aria-label` → use directly
3. Native label (`<label for>`, `alt`, `title`, `placeholder`)
4. Text content (for elements like `<button>`, `<a>`)
5. `title` attribute (last resort)

## 25.3 Platform Integration

### 25.3.1 AccessKit

The `accesskit` crate provides cross-platform a11y abstraction:

| Platform | Native API | AccessKit Adapter |
| --- | --- | --- |
| macOS | NSAccessibility | accesskit_macos |
| Windows | UI Automation | accesskit_windows |
| Linux | AT-SPI2 (D-Bus) | accesskit_unix |

AccessKit exposes a tree-update model: on each frame (or on DOM mutation), elidex sends a `TreeUpdate` containing changed nodes. AccessKit translates these into platform-specific API calls.

### 25.3.2 Update Strategy

Full tree rebuild on every frame would be expensive. Instead, the A11ySystem tracks dirty nodes:

- DOM mutation (node added/removed/attribute changed) → mark dirty
- Layout change (bounding rect changed) → mark dirty
- Style change affecting visibility → mark dirty

Only dirty subtrees are re-evaluated. The `TreeUpdate` sent to AccessKit contains only changed nodes.

## 25.4 Focus Management

Focus is tracked as an ECS resource:

```rust
pub struct FocusState {
    pub focused_entity: Option<EntityId>,
    pub focus_visible: bool,
}
```

Focus order follows DOM order (default) or `tabindex`. When focus changes, the A11ySystem notifies AccessKit, which fires platform focus events. Screen readers announce the newly focused element.

Focus trapping (for modals): `<dialog>` and elements with `role="dialog"` restrict Tab cycling to their descendants.

## 25.5 Live Regions

ARIA live regions (`aria-live="polite|assertive"`) announce dynamic content changes:

```rust
pub struct LiveRegionAnnouncement {
    pub text: String,
    pub priority: LivePriority,
}

pub enum LivePriority {
    Polite,     // Announced after current speech
    Assertive,  // Interrupts current speech
}
```

When content within a live region changes, the A11ySystem extracts the changed text and sends an announcement to AccessKit.

## 25.6 High Contrast and Forced Colors

`prefers-color-scheme` and `forced-colors` media queries are exposed to CSS. In `forced-colors: active` mode, the browser overrides page colors with the system color scheme (Windows High Contrast). The rendering pipeline respects `color-scheme` and system colors (`Canvas`, `CanvasText`, `LinkText`, etc.).

## 25.7 elidex-app Accessibility

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| A11y tree | Full, automatic | Full, automatic |
| Platform integration | AccessKit | AccessKit |
| Focus management | Standard | Standard + app can manage focus via Embedding API |
| Live regions | Full support | Full support |
| High contrast | Respected | Respected |
