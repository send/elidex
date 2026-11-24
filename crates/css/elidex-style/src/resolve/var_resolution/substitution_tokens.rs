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
