//! Extract style and script sources from the DOM tree.
//!
//! Walks the DOM in document order to collect `<style>`, `<link rel="stylesheet">`,
//! and `<script>` elements, producing lists of resources to fetch or inline.

use elidex_ecs::{Attributes, EcsDom, Entity, TagType, TextContent};

/// A stylesheet source found in the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleSource {
    /// Inline `<style>` element text content.
    Inline(String),
    /// External `<link rel="stylesheet" href="...">` URL.
    External(String),
}

/// A script source found in the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptSource {
    /// Inline `<script>` element with text content.
    Inline {
        /// The script source code.
        source: String,
        /// The entity of the `<script>` element.
        entity: Entity,
    },
    /// External `<script src="...">` element.
    External {
        /// The `src` attribute value (relative or absolute URL).
        src: String,
        /// The entity of the `<script>` element.
        entity: Entity,
    },
}

/// Collect all text content from direct children of the given entity.
fn collect_text_content(dom: &EcsDom, entity: Entity) -> String {
    let mut text = String::new();
    for child in dom.children_iter(entity) {
        if let Ok(tc) = dom.world().get::<&TextContent>(child) {
            text.push_str(&tc.0);
        }
    }
    text
}

/// Extract all stylesheet sources from the DOM in document order.
///
/// Collects `<style>` elements (inline CSS) and `<link rel="stylesheet" href="...">`
/// elements (external CSS references).
pub fn extract_style_sources(dom: &EcsDom, document: Entity) -> Vec<StyleSource> {
    let mut sources = Vec::new();
    collect_styles(dom, document, &mut sources);
    sources
}

fn collect_styles(dom: &EcsDom, entity: Entity, sources: &mut Vec<StyleSource>) {
    if let Ok(tag) = dom.world().get::<&TagType>(entity) {
        match tag.0.as_str() {
            "style" => {
                let text = collect_text_content(dom, entity);
                if !text.is_empty() {
                    sources.push(StyleSource::Inline(text));
                }
                return; // Don't recurse into <style> children.
            }
            "link" => {
                if let Ok(attrs) = dom.world().get::<&Attributes>(entity) {
                    let is_stylesheet = attrs.get("rel").is_some_and(|r| {
                        r.split_ascii_whitespace()
                            .any(|token| token.eq_ignore_ascii_case("stylesheet"))
                    });
                    if is_stylesheet {
                        if let Some(href) = attrs.get("href") {
                            if !href.is_empty() {
                                sources.push(StyleSource::External(href.to_string()));
                            }
                        }
                    }
                }
                return; // <link> is void element, no children.
            }
            _ => {}
        }
    }

    for child in dom.children_iter(entity) {
        collect_styles(dom, child, sources);
    }
}

/// Extract all script sources from the DOM in document order.
///
/// Collects inline `<script>` elements and `<script src="...">` elements.
pub fn extract_script_sources(dom: &EcsDom, document: Entity) -> Vec<ScriptSource> {
    let mut sources = Vec::new();
    collect_scripts(dom, document, &mut sources);
    sources
}

fn collect_scripts(dom: &EcsDom, entity: Entity, sources: &mut Vec<ScriptSource>) {
    if let Ok(tag) = dom.world().get::<&TagType>(entity) {
        if tag.0 == "script" {
            let attrs_ref = dom.world().get::<&Attributes>(entity).ok();

            // Skip scripts with a non-JavaScript type attribute.
            if let Some(type_val) = attrs_ref.as_ref().and_then(|a| a.get("type")) {
                let t = type_val.trim();
                if !t.is_empty()
                    && !t.eq_ignore_ascii_case("text/javascript")
                    && !t.eq_ignore_ascii_case("application/javascript")
                {
                    return;
                }
            }

            let has_src = attrs_ref.and_then(|attrs| attrs.get("src").map(ToString::to_string));

            if let Some(src) = has_src {
                if !src.is_empty() {
                    sources.push(ScriptSource::External { src, entity });
                }
            } else {
                // Collect inline text content.
                let source = collect_text_content(dom, entity);
                if !source.is_empty() {
                    sources.push(ScriptSource::Inline { source, entity });
                }
            }
            return; // Don't recurse into <script> children.
        }
    }

    for child in dom.children_iter(entity) {
        collect_scripts(dom, child, sources);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dom_with_style(css: &str) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let head = dom.create_element("head", Attributes::default());
        let _ = dom.append_child(doc, head);
        let style = dom.create_element("style", Attributes::default());
        let text = dom.create_text(css);
        let _ = dom.append_child(style, text);
        let _ = dom.append_child(head, style);
        (dom, doc)
    }

    fn make_dom_with_link(href: &str) -> (EcsDom, Entity) {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let head = dom.create_element("head", Attributes::default());
        let _ = dom.append_child(doc, head);
        let mut attrs = Attributes::default();
        attrs.set("rel", "stylesheet");
        attrs.set("href", href);
        let link = dom.create_element("link", attrs);
        let _ = dom.append_child(head, link);
        (dom, doc)
    }

    #[test]
    fn extract_inline_style() {
        let (dom, doc) = make_dom_with_style("body { color: red; }");
        let sources = extract_style_sources(&dom, doc);
        assert_eq!(
            sources,
            vec![StyleSource::Inline("body { color: red; }".into())]
        );
    }

    #[test]
    fn extract_external_stylesheet() {
        let (dom, doc) = make_dom_with_link("style.css");
        let sources = extract_style_sources(&dom, doc);
        assert_eq!(sources, vec![StyleSource::External("style.css".into())]);
    }

    #[test]
    fn extract_mixed_styles() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let head = dom.create_element("head", Attributes::default());
        let _ = dom.append_child(doc, head);

        // <link rel="stylesheet" href="a.css">
        let mut attrs = Attributes::default();
        attrs.set("rel", "stylesheet");
        attrs.set("href", "a.css");
        let link = dom.create_element("link", attrs);
        let _ = dom.append_child(head, link);

        // <style>p { margin: 0; }</style>
        let style = dom.create_element("style", Attributes::default());
        let text = dom.create_text("p { margin: 0; }");
        let _ = dom.append_child(style, text);
        let _ = dom.append_child(head, style);

        let sources = extract_style_sources(&dom, doc);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0], StyleSource::External("a.css".into()));
        assert_eq!(sources[1], StyleSource::Inline("p { margin: 0; }".into()));
    }

    #[test]
    fn empty_style_ignored() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let style = dom.create_element("style", Attributes::default());
        let _ = dom.append_child(doc, style);
        let sources = extract_style_sources(&dom, doc);
        assert!(sources.is_empty());
    }

    #[test]
    fn link_without_href_ignored() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("rel", "stylesheet");
        let link = dom.create_element("link", attrs);
        let _ = dom.append_child(doc, link);
        let sources = extract_style_sources(&dom, doc);
        assert!(sources.is_empty());
    }

    #[test]
    fn link_non_stylesheet_ignored() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("rel", "icon");
        attrs.set("href", "favicon.ico");
        let link = dom.create_element("link", attrs);
        let _ = dom.append_child(doc, link);
        let sources = extract_style_sources(&dom, doc);
        assert!(sources.is_empty());
    }

    #[test]
    fn extract_inline_script() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let script = dom.create_element("script", Attributes::default());
        let text = dom.create_text("alert(1)");
        let _ = dom.append_child(script, text);
        let _ = dom.append_child(doc, script);

        let sources = extract_script_sources(&dom, doc);
        assert_eq!(sources.len(), 1);
        match &sources[0] {
            ScriptSource::Inline { source, .. } => assert_eq!(source, "alert(1)"),
            ScriptSource::External { .. } => panic!("expected inline"),
        }
    }

    #[test]
    fn extract_external_script() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("src", "app.js");
        let script = dom.create_element("script", attrs);
        let _ = dom.append_child(doc, script);

        let sources = extract_script_sources(&dom, doc);
        assert_eq!(sources.len(), 1);
        match &sources[0] {
            ScriptSource::External { src, .. } => assert_eq!(src, "app.js"),
            ScriptSource::Inline { .. } => panic!("expected external"),
        }
    }

    #[test]
    fn scripts_document_order() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let body = dom.create_element("body", Attributes::default());
        let _ = dom.append_child(doc, body);

        // Inline script first.
        let s1 = dom.create_element("script", Attributes::default());
        let t1 = dom.create_text("var a = 1;");
        let _ = dom.append_child(s1, t1);
        let _ = dom.append_child(body, s1);

        // External script second.
        let mut attrs = Attributes::default();
        attrs.set("src", "b.js");
        let s2 = dom.create_element("script", attrs);
        let _ = dom.append_child(body, s2);

        let sources = extract_script_sources(&dom, doc);
        assert_eq!(sources.len(), 2);
        assert!(
            matches!(&sources[0], ScriptSource::Inline { source, .. } if source == "var a = 1;")
        );
        assert!(matches!(&sources[1], ScriptSource::External { src, .. } if src == "b.js"));
    }

    #[test]
    fn empty_script_ignored() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let script = dom.create_element("script", Attributes::default());
        let _ = dom.append_child(doc, script);
        let sources = extract_script_sources(&dom, doc);
        assert!(sources.is_empty());
    }

    #[test]
    fn script_type_module_skipped() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("type", "module");
        let script = dom.create_element("script", attrs);
        let text = dom.create_text("import x from 'y';");
        let _ = dom.append_child(script, text);
        let _ = dom.append_child(doc, script);
        let sources = extract_script_sources(&dom, doc);
        assert!(sources.is_empty());
    }

    #[test]
    fn script_type_text_javascript_allowed() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("type", "text/javascript");
        let script = dom.create_element("script", attrs);
        let text = dom.create_text("var x = 1;");
        let _ = dom.append_child(script, text);
        let _ = dom.append_child(doc, script);
        let sources = extract_script_sources(&dom, doc);
        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn script_type_application_javascript_allowed() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("type", "application/javascript");
        let script = dom.create_element("script", attrs);
        let text = dom.create_text("var x = 1;");
        let _ = dom.append_child(script, text);
        let _ = dom.append_child(doc, script);
        let sources = extract_script_sources(&dom, doc);
        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn script_no_type_allowed() {
        // No type attribute at all should be treated as JavaScript.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let script = dom.create_element("script", Attributes::default());
        let text = dom.create_text("var x = 1;");
        let _ = dom.append_child(script, text);
        let _ = dom.append_child(doc, script);
        let sources = extract_script_sources(&dom, doc);
        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn script_type_json_skipped() {
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("type", "application/json");
        let script = dom.create_element("script", attrs);
        let text = dom.create_text(r#"{"key": "value"}"#);
        let _ = dom.append_child(script, text);
        let _ = dom.append_child(doc, script);
        let sources = extract_script_sources(&dom, doc);
        assert!(sources.is_empty());
    }

    #[test]
    fn link_rel_multi_token_stylesheet() {
        // rel="alternate stylesheet" should still be detected.
        let mut dom = EcsDom::new();
        let doc = dom.create_document_root();
        let mut attrs = Attributes::default();
        attrs.set("rel", "alternate stylesheet");
        attrs.set("href", "alt.css");
        let link = dom.create_element("link", attrs);
        let _ = dom.append_child(doc, link);
        let sources = extract_style_sources(&dom, doc);
        assert_eq!(sources, vec![StyleSource::External("alt.css".into())]);
    }
}
