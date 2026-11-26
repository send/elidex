use super::*;

#[test]
fn factor_group_and_basis_expand_like_explicit_longhands() {
    for (input, grow, shrink, basis) in [
        ("0", "0", "1", "0px"),
        ("2", "2", "1", "0px"),
        ("2 0", "2", "0", "0px"),
        ("0px", "1", "1", "0px"),
        ("0%", "1", "1", "0%"),
        ("0 0 0", "0", "0", "0px"),
        ("auto", "1", "1", "auto"),
        ("none", "0", "0", "auto"),
        ("content", "1", "1", "content"),
        ("0 auto", "0", "1", "auto"),
        ("25% 2", "2", "1", "25%"),
        ("2 25%", "2", "1", "25%"),
        ("25% 2 3", "2", "3", "25%"),
        ("2 3 25%", "2", "3", "25%"),
        ("2 calc(50% - 1px)", "2", "1", "calc(50% - 1px)"),
        ("calc(50% - 1px) 2", "2", "1", "calc(50% - 1px)"),
        ("calc(1 + 1) 10px", "calc(1 + 1)", "1", "10px"),
        ("10px calc(1 + 1)", "calc(1 + 1)", "1", "10px"),
    ] {
        let short = parse_single("flex", &format!("{input} !important"));
        assert_eq!(short.len(), 3, "{input}");
        for (decl, (name, value)) in short.iter().zip([
            ("flex-grow", grow),
            ("flex-shrink", shrink),
            ("flex-basis", basis),
        ]) {
            assert!(decl.important);
            assert_eq!(decl.property, name);
            assert_eq!(decl.value, parse_single(name, value)[0].value, "{input}");
        }
    }
}

#[test]
fn invalid_group_or_unit_leaves_next_declaration_intact() {
    for value in [
        "2 10px 3",
        "2 3 4",
        "auto content",
        "none 1",
        "-1",
        "2 -3",
        "-1px",
        "2bogus",
        "2 0 -1px",
        "2 0 -5%",
        "2 0 2bogus",
        "stretch",
        "min-content",
        "max-content",
        "fit-content",
        "calc(1 + 1px)",
        "calc(1+ 2)",
        "calc(1 +2)",
        "calc(1/**/+ 2)",
    ] {
        let declarations = parse_decls(&format!("flex:{value}; width:3px"));
        assert_eq!(declarations.len(), 1, "{value}");
        assert_eq!(declarations[0].property, "width");
    }
}

#[test]
fn basis_range_agrees_across_block_grammars() {
    let registry = elidex_plugin::CssPropertyRegistry::default();
    for grammar in [None, Some(&registry)] {
        for (basis, valid) in [
            ("-1px", false),
            ("-5%", false),
            ("2bogus", false),
            ("0em", true),
            ("calc(10% + 2px)", true),
        ] {
            for (property, value, count) in [
                ("flex-basis", basis.to_string(), 1),
                ("flex", basis.to_string(), 3),
                ("flex", format!("2 0 {basis}"), 3),
            ] {
                let decls = crate::parse_declaration_block_with_registry(
                    &format!("{property}:{value}; width:1px"),
                    grammar,
                );
                assert_eq!(
                    decls.len(),
                    if valid { count + 1 } else { 1 },
                    "{property}:{value}"
                );
                assert_eq!(decls.last().unwrap().property, "width");
            }
        }
    }
}
