use super::*;

#[test]
fn pending_admission_uses_decoded_functions_and_validates_each_call() {
    for property in ["width", "--x"] {
        for value in [
            "var()",
            "var(,)",
            "calc(var(--ok) + var())",
            "var(--x, var())",
        ] {
            assert!(
                parse_value_for_property(property, value, None).is_none(),
                "{property}: {value}"
            );
        }
        for value in [
            "var(123,5px)",
            "var(--x,)",
            "calc(VaR(--n) + 1px)",
            r"v\61r(--x)",
        ] {
            assert!(
                parse_value_for_property(property, value, None).is_some(),
                "{property}: {value}"
            );
        }
    }
    assert!(parse_value_for_property("not-a-real-property", "var(--x)", None).is_none());
    assert!(parse_value_for_property("width", "1px", None).is_some());
    assert!(parse_value_for_property("width", "\"var(\"", None).is_none());
    let decls = parse_decls("width:7px; width:var(); height:9px");
    assert_eq!(
        decls
            .iter()
            .map(|d| d.property.as_str())
            .collect::<Vec<_>>(),
        ["width", "height"]
    );
}

#[test]
fn empty_custom_property_has_a_specified_space_and_no_components() {
    for css in ["--e:;", "--e:   ;"] {
        let decls = parse_decls(css);
        assert_eq!(decls[0].value, custom(""));
        assert_eq!(decls[0].value.to_css_string(), " ");
    }
    let value = parse_value_for_property("--copy", "2/**/px", None).unwrap();
    assert_eq!(value[0].value, custom("2/**/px"));
}

#[test]
fn var_function_parser_leaves_the_following_component() {
    let mut source = ParserInput::new("VAR(--x, 001px) solid");
    let mut parser = Parser::new(&mut source);
    assert_eq!(
        parse_var_function(&mut parser).unwrap().to_css_string(),
        "VAR(--x, 001px)"
    );
    parser.expect_ident_matching("solid").unwrap();
}
