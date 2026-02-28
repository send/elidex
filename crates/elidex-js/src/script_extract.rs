//! Extract `<script>` elements from the DOM tree.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, TextContent};

/// A script entry extracted from the DOM.
#[derive(Clone, Debug)]
pub struct ScriptEntry {
    /// The script source code (inline text content).
    pub source: String,
    /// The entity of the `<script>` element.
    pub entity: Entity,
}

/// Extract all `<script>` elements from the DOM tree in document order.
///
/// - Inline scripts: extracts text content as source.
/// - External scripts (`<script src="...">`): logged and skipped (M2-7).
pub fn extract_scripts(dom: &EcsDom, document: Entity) -> Vec<ScriptEntry> {
    let mut scripts = Vec::new();
    collect_scripts(dom, document, &mut scripts);
    scripts
}

fn collect_scripts(dom: &EcsDom, entity: Entity, scripts: &mut Vec<ScriptEntry>) {
    // Check if this entity is a <script> element.
    if let Ok(tag) = dom.world().get::<&TagType>(entity) {
        if tag.0 == "script" {
            // Check for src attribute (external script — skip for M2-4).
            let has_src = dom
                .world()
                .get::<&Attributes>(entity)
                .ok()
                .is_some_and(|attrs| attrs.contains("src"));

            if has_src {
                tracing::info!(
                    "Skipping external <script src=\"...\"> (M2-7 will add network support)"
                );
            } else {
                // Collect inline text content.
                let mut source = String::new();
                for child in dom.children_iter(entity) {
                    if let Ok(tc) = dom.world().get::<&TextContent>(child) {
                        source.push_str(&tc.0);
                    }
                }
                if !source.is_empty() {
                    scripts.push(ScriptEntry { source, entity });
                }
            }
            // Don't recurse into <script> children (they are text, not elements).
            return;
        }
    }

    // Recurse into children.
    for child in dom.children_iter(entity) {
        collect_scripts(dom, child, scripts);
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;

    fn make_script(dom: &mut EcsDom, code: &str) -> Entity {
        let script = dom.create_element("script", Attributes::default());
        let text = dom.create_text(code);
        dom.append_child(script, text);
        script
    }

    fn make_external_script(dom: &mut EcsDom, src: &str) -> Entity {
        let mut attrs = Attributes::default();
        attrs.set("src", src);
        dom.create_element("script", attrs)
    }

    #[test]
    fn extract_inline_script() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc, body);
        let script = make_script(&mut dom, "console.log('hi')");
        dom.append_child(body, script);

        let scripts = extract_scripts(&dom, doc);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].source, "console.log('hi')");
    }

    #[test]
    fn skip_external_script() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc, body);
        let script = make_external_script(&mut dom, "app.js");
        dom.append_child(body, script);

        let scripts = extract_scripts(&dom, doc);
        assert!(scripts.is_empty());
    }

    #[test]
    fn multiple_scripts_document_order() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let body = dom.create_element("body", Attributes::default());
        dom.append_child(doc, body);

        let s1 = make_script(&mut dom, "var a = 1;");
        let s2 = make_script(&mut dom, "var b = 2;");
        dom.append_child(body, s1);
        dom.append_child(body, s2);

        let scripts = extract_scripts(&dom, doc);
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0].source, "var a = 1;");
        assert_eq!(scripts[1].source, "var b = 2;");
    }

    #[test]
    fn nested_scripts() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(doc, div);
        let script = make_script(&mut dom, "nested();");
        dom.append_child(div, script);

        let scripts = extract_scripts(&dom, doc);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].source, "nested();");
    }

    #[test]
    fn empty_script_ignored() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let script = dom.create_element("script", Attributes::default());
        dom.append_child(doc, script);

        let scripts = extract_scripts(&dom, doc);
        assert!(scripts.is_empty());
    }
}
