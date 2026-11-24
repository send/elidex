use super::substitute::Environment;
use elidex_plugin::component_values::{ComponentValues, ComputedCustom};
use std::collections::HashMap;

#[test]
fn actual_serialized_bytes_determine_the_limit() {
    for (name, join, prefix) in [
        ("a/*x*", ", ", "a/*x**/, "),
        ("\"a", ", ", "\"a\", "),
        ("2", "px ", "2/**/px "),
        (r"\61", " ", r"\61/**/ "),
    ] {
        let inherited = HashMap::from([(
            "--x".into(),
            ComputedCustom::Value(ComponentValues::capture(name).unwrap()),
        )]);
        for size in [65_536, 65_537] {
            let suffix = "s".repeat(size - prefix.len());
            let seq = ComponentValues::capture(&format!("var(--x){join}{suffix}")).unwrap();
            let result = Environment::new(&inherited, HashMap::new()).ordinary(&seq);
            if size == 65_536 {
                let result = result.expect("exact limit");
                assert_eq!(result.source(), format!("{prefix}{suffix}"));
            } else {
                assert!(result.is_none());
            }
        }
    }
}

#[test]
fn repeated_references_charge_each_occurrence() {
    for size in [32_767, 32_768] {
        let text = "x".repeat(size);
        let props = HashMap::from([(
            "--x".into(),
            ComputedCustom::Value(ComponentValues::capture(&text).unwrap()),
        )]);
        let seq = ComponentValues::capture("var(--x) var(--x)").unwrap();
        let result = Environment::new(&props, HashMap::new()).ordinary(&seq);
        assert_eq!(result.is_some(), size == 32_767);
    }
}

#[test]
fn only_selected_fallback_counts_toward_output_budget() {
    let huge = "x".repeat(65_537);
    let inherited = HashMap::from([
        (
            "--ok".into(),
            ComputedCustom::Value(ComponentValues::capture("2px").unwrap()),
        ),
        (
            "--name".into(),
            ComputedCustom::Value(ComponentValues::capture("--ok").unwrap()),
        ),
    ]);
    for (first, expected) in [
        ("--ok", Some("2px")),
        ("...var(--name)", Some("2px")),
        ("--missing", None),
    ] {
        let value = ComponentValues::capture(&format!("var({first},{huge})")).unwrap();
        let actual = Environment::new(&inherited, HashMap::new()).ordinary(&value);
        assert_eq!(actual.as_ref().map(ComponentValues::source), expected);
    }
}

#[test]
fn shortening_eof_string_repair_uses_final_byte_length() {
    let inherited = HashMap::from([(
        "--x".into(),
        ComputedCustom::Value(ComponentValues::capture(r#""\000061"#).unwrap()),
    )]);
    for size in [65_536, 65_537] {
        let suffix = "x".repeat(size - 4);
        let value = ComponentValues::capture(&format!("var(--x),{suffix}")).unwrap();
        let actual = Environment::new(&inherited, HashMap::new()).ordinary(&value);
        if size == 65_536 {
            assert_eq!(actual.unwrap().source(), format!(r#""a",{suffix}"#));
        } else {
            assert!(actual.is_none());
        }
    }
}

#[test]
fn numeric_source_and_typed_leaves_survive_substituted_reparse() {
    for (property, source) in [
        ("flex-grow", "001.2500"),
        ("flex-basis", "002.54000cm"),
        ("flex-basis", "012.5000%"),
        ("flex-grow", "calc(001.2500 + 02)"),
        ("flex-basis", "calc(002.54000cm + 012.5000%)"),
    ] {
        let inherited = HashMap::from([(
            "--x".into(),
            ComputedCustom::Value(ComponentValues::capture(source).unwrap()),
        )]);
        let seq = ComponentValues::capture("var(--x)").unwrap();
        let expanded = Environment::new(&inherited, HashMap::new())
            .ordinary(&seq)
            .unwrap();
        assert_eq!(expanded.source(), source);
        let literal = elidex_css::parse_value_for_property(property, source, None).unwrap();
        let substituted =
            elidex_css::parse_value_for_property(property, expanded.source(), None).unwrap();
        assert_eq!(literal[0].value, substituted[0].value);
    }
}
