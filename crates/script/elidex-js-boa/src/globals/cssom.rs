//! CSSOM (CSS Object Model) JS object builders.
//!
//! Implements:
//! - `document.styleSheets` → `StyleSheetList`
//! - `CSSStyleSheet` (cssRules, insertRule, deleteRule, type)
//! - `CSSRuleList` (length, item)
//! - `CSSStyleRule` (selectorText, cssText, style, type)
//! - `CSSStyleDeclaration` extensions (cssText, length, item)

use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{js_string, Context, JsNativeError, JsResult, JsValue, NativeFunction};

use crate::bridge::HostBridge;

/// Hidden property key storing the sheet index on `CSSStyleSheet` objects.
const SHEET_INDEX_KEY: &str = "__elidex_sheet_idx__";

/// Hidden property key storing the rule index on `CSSStyleRule` objects.
const RULE_INDEX_KEY: &str = "__elidex_rule_idx__";

/// Build the `document.styleSheets` StyleSheetList-like JS object.
///
/// Returns an array-like object with `length` and `item(index)` method,
/// plus numeric index access via properties.
pub fn build_stylesheet_list(bridge: &HostBridge, ctx: &mut Context) -> JsValue {
    let mut obj = ObjectInitializer::new(ctx);

    // length — dynamic getter
    let realm = obj.context().realm().clone();
    let b_len = bridge.clone();
    let length_getter = NativeFunction::from_copy_closure_with_captures(
        |_this, _args, bridge, _ctx| Ok(JsValue::from(bridge.stylesheet_count() as f64)),
        b_len,
    )
    .to_js_function(&realm);
    obj.accessor(
        js_string!("length"),
        Some(length_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // item(index) — returns CSSStyleSheet or null
    let b_item = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, bridge, ctx| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = args
                    .first()
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                if index < bridge.stylesheet_count() {
                    build_stylesheet_object(index, bridge, ctx)
                } else {
                    Ok(JsValue::null())
                }
            },
            b_item,
        ),
        js_string!("item"),
        1,
    );

    obj.build().into()
}

/// Build a `CSSStyleSheet`-like JS object for the given sheet index.
#[allow(clippy::unnecessary_wraps)]
fn build_stylesheet_object(
    sheet_index: usize,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let mut obj = ObjectInitializer::new(ctx);

    // Hidden sheet index.
    #[allow(clippy::cast_precision_loss)]
    obj.property(
        js_string!(SHEET_INDEX_KEY),
        JsValue::from(sheet_index as f64),
        Attribute::empty(),
    );

    // type → "text/css"
    obj.property(
        js_string!("type"),
        JsValue::from(js_string!("text/css")),
        Attribute::READONLY,
    );

    // disabled — writable property (stub: always false)
    obj.property(
        js_string!("disabled"),
        JsValue::from(false),
        Attribute::WRITABLE | Attribute::CONFIGURABLE,
    );

    // cssRules — getter that returns a CSSRuleList
    let realm = obj.context().realm().clone();
    let b_rules = bridge.clone();
    let rules_getter = NativeFunction::from_copy_closure_with_captures(
        move |this, _args, bridge, ctx| {
            let idx = extract_sheet_index(this, ctx)?;
            build_rule_list(idx, bridge, ctx)
        },
        b_rules,
    )
    .to_js_function(&realm);
    obj.accessor(
        js_string!("cssRules"),
        Some(rules_getter),
        None,
        Attribute::CONFIGURABLE,
    );

    // insertRule(rule, index?)
    let b_insert = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let sheet_idx = extract_sheet_index(this, ctx)?;
                let rule_text = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or(String::new(), |s| s.to_std_string_escaped());

                // Default index: end of rules list.
                let rules_len = bridge.stylesheet_rules(sheet_idx).map_or(0, |r| r.len());
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = args
                    .get(1)
                    .and_then(JsValue::as_number)
                    .map_or(rules_len, |n| n as usize);

                match bridge.cssom_insert_rule(sheet_idx, index, &rule_text) {
                    Some(idx) => Ok(JsValue::from(idx as f64)),
                    None => Err(JsNativeError::range()
                        .with_message("Failed to execute 'insertRule': invalid index or rule")
                        .into()),
                }
            },
            b_insert,
        ),
        js_string!("insertRule"),
        1,
    );

    // deleteRule(index)
    let b_delete = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |this, args, bridge, ctx| {
                let sheet_idx = extract_sheet_index(this, ctx)?;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = args
                    .first()
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);

                if bridge.cssom_delete_rule(sheet_idx, index) {
                    Ok(JsValue::undefined())
                } else {
                    Err(JsNativeError::range()
                        .with_message("Failed to execute 'deleteRule': index out of range")
                        .into())
                }
            },
            b_delete,
        ),
        js_string!("deleteRule"),
        1,
    );

    Ok(obj.build().into())
}

/// Build a `CSSRuleList`-like JS object for the given sheet index.
fn build_rule_list(
    sheet_index: usize,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let rules = bridge.stylesheet_rules(sheet_index).unwrap_or_default();

    // Pre-build rule objects before creating ObjectInitializer (avoids
    // double-borrow of ctx).
    let mut rule_objects = Vec::with_capacity(rules.len());
    for (i, rule) in rules.iter().enumerate() {
        rule_objects.push(build_style_rule(sheet_index, i, rule, bridge, ctx)?);
    }

    let mut obj = ObjectInitializer::new(ctx);

    // length
    obj.property(
        js_string!("length"),
        JsValue::from(rules.len() as f64),
        Attribute::READONLY,
    );

    // item(index)
    let b_item = bridge.clone();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            move |_this, args, bridge, ctx| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = args
                    .first()
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                let rules = bridge.stylesheet_rules(sheet_index).unwrap_or_default();
                if index < rules.len() {
                    build_style_rule(sheet_index, index, &rules[index], bridge, ctx)
                } else {
                    Ok(JsValue::null())
                }
            },
            b_item,
        ),
        js_string!("item"),
        1,
    );

    // Numeric index access (pre-built objects)
    for (i, rule_obj) in rule_objects.into_iter().enumerate() {
        obj.property(
            js_string!(i.to_string().as_str()),
            rule_obj,
            Attribute::CONFIGURABLE,
        );
    }

    Ok(obj.build().into())
}

/// Build a `CSSStyleRule`-like JS object.
#[allow(clippy::unnecessary_wraps)] // Returns JsResult for consistency with item() callback.
fn build_style_rule(
    sheet_index: usize,
    rule_index: usize,
    rule: &crate::bridge::CssomRule,
    bridge: &HostBridge,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    // Pre-build the style object before the ObjectInitializer borrows ctx.
    let style_obj =
        build_rule_style_declaration(sheet_index, rule_index, &rule.declarations, bridge, ctx);
    let css_text = rule.css_text();

    let mut obj = ObjectInitializer::new(ctx);

    // Hidden indices.
    #[allow(clippy::cast_precision_loss)]
    {
        obj.property(
            js_string!(SHEET_INDEX_KEY),
            JsValue::from(sheet_index as f64),
            Attribute::empty(),
        );
        obj.property(
            js_string!(RULE_INDEX_KEY),
            JsValue::from(rule_index as f64),
            Attribute::empty(),
        );
    }

    // type → 1 (CSSRule.STYLE_RULE)
    obj.property(js_string!("type"), JsValue::from(1), Attribute::READONLY);

    // selectorText
    obj.property(
        js_string!("selectorText"),
        JsValue::from(js_string!(rule.selector_text.as_str())),
        Attribute::CONFIGURABLE,
    );

    // cssText
    obj.property(
        js_string!("cssText"),
        JsValue::from(js_string!(css_text.as_str())),
        Attribute::CONFIGURABLE,
    );

    // style — a CSSStyleDeclaration for the rule's declarations
    obj.property(js_string!("style"), style_obj, Attribute::CONFIGURABLE);

    Ok(obj.build().into())
}

/// Build a `CSSStyleDeclaration`-like JS object for a rule's declarations.
fn build_rule_style_declaration(
    sheet_index: usize,
    rule_index: usize,
    declarations: &[(String, String)],
    _bridge: &HostBridge,
    ctx: &mut Context,
) -> JsValue {
    let mut obj = ObjectInitializer::new(ctx);

    // Hidden indices for future setProperty/removeProperty support on rule styles.
    #[allow(clippy::cast_precision_loss)]
    {
        obj.property(
            js_string!(SHEET_INDEX_KEY),
            JsValue::from(sheet_index as f64),
            Attribute::empty(),
        );
        obj.property(
            js_string!(RULE_INDEX_KEY),
            JsValue::from(rule_index as f64),
            Attribute::empty(),
        );
    }

    // length
    obj.property(
        js_string!("length"),
        JsValue::from(declarations.len() as f64),
        Attribute::READONLY,
    );

    // cssText
    let css_text: String = declarations
        .iter()
        .map(|(p, v)| format!("{p}: {v};"))
        .collect::<Vec<_>>()
        .join(" ");
    obj.property(
        js_string!("cssText"),
        JsValue::from(js_string!(css_text.as_str())),
        Attribute::CONFIGURABLE,
    );

    // item(index) → property name
    let decl_names: Vec<String> = declarations.iter().map(|(p, _)| p.clone()).collect();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, names, _ctx| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let index = args
                    .first()
                    .and_then(JsValue::as_number)
                    .map_or(0, |n| n as usize);
                Ok(names
                    .get(index)
                    .map_or(JsValue::from(js_string!("")), |name| {
                        JsValue::from(js_string!(name.as_str()))
                    }))
            },
            decl_names,
        ),
        js_string!("item"),
        1,
    );

    // getPropertyValue(name)
    let decl_map: Vec<(String, String)> = declarations.to_vec();
    obj.function(
        NativeFunction::from_copy_closure_with_captures(
            |_this, args, decls, ctx| {
                let name = args
                    .first()
                    .map(|v| v.to_string(ctx))
                    .transpose()?
                    .map_or(String::new(), |s| s.to_std_string_escaped());
                let value = decls
                    .iter()
                    .find(|(p, _)| *p == name)
                    .map_or(String::new(), |(_, v)| v.clone());
                Ok(JsValue::from(js_string!(value.as_str())))
            },
            decl_map,
        ),
        js_string!("getPropertyValue"),
        1,
    );

    // Numeric index access (property names at indices)
    for (i, (prop, _)) in declarations.iter().enumerate() {
        obj.property(
            js_string!(i.to_string().as_str()),
            JsValue::from(js_string!(prop.as_str())),
            Attribute::CONFIGURABLE,
        );
    }

    obj.build().into()
}

/// Extract the sheet index from a JS object's hidden property.
fn extract_sheet_index(this: &JsValue, ctx: &mut Context) -> JsResult<usize> {
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("CSSOM method called on non-object"))?;
    let id_val = obj.get(js_string!(SHEET_INDEX_KEY), ctx)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = id_val
        .as_number()
        .ok_or_else(|| JsNativeError::typ().with_message("invalid CSSStyleSheet object"))?
        as usize;
    Ok(idx)
}
