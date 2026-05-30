//! html5lib-tests tokenizer corpus, run against the strict tokenizer.
//!
//! Per the A2 plan decision D3, the tokenizer is exercised by the full
//! html5lib tokenizer suite (vendored under `tests/data/html5lib/`, MIT).
//! Because strict mode has no error recovery, the corpus splits cleanly:
//!
//! - a test whose `errors` list is **non-empty** must make the tokenizer
//!   abort with [`StrictParseError`];
//! - a test with **no** errors is conformant HTML5, so the emitted token
//!   stream must match the expected `output` exactly (character tokens
//!   coalesced into runs, as html5lib reports them).
//!
//! Vendored corpus: the core html5lib tokenizer suite — `test1`–`test4`,
//! `contentModelFlags`, `escapeFlag`, `numericEntities`, and the full
//! `namedEntities` (all 2231 references). The harness handles
//! `initialStates`, `lastStartTag`, and `doubleEscaped`, so adding a
//! future suite file is a one-line `suite_test!` entry.

use super::states::{State, Tokenizer};
use super::token::Token;
use serde_json::Value;
use std::collections::BTreeMap;

/// A token normalized for comparison against html5lib's expected output.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Norm {
    Char(String),
    Start(String, BTreeMap<String, String>, bool),
    End(String),
    Comment(String),
    Doctype(Option<String>, Option<String>, Option<String>, bool),
}

/// Map an html5lib initial-state name to our [`State`].
fn state_from_name(name: &str) -> State {
    match name {
        "Data state" => State::Data,
        "PLAINTEXT state" => State::Plaintext,
        "RCDATA state" => State::Rcdata,
        "RAWTEXT state" => State::Rawtext,
        "Script data state" => State::ScriptData,
        "CDATA section state" => State::CdataSection,
        other => panic!("unknown initial state in test data: {other}"),
    }
}

/// Decode the `\uXXXX` escapes html5lib uses when `doubleEscaped` is set.
fn double_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'u') {
            chars.next();
            let hex: String = (0..4).filter_map(|_| chars.next()).collect();
            let code = u32::from_str_radix(&hex, 16).expect("valid \\u escape");
            out.push(char::from_u32(code).expect("valid scalar in test data"));
        } else {
            out.push(c);
        }
    }
    out
}

/// Run the strict tokenizer over `input` from `state`, coalescing
/// character tokens into runs and dropping the trailing EOF.
fn normalize_run(input: &str, state: State, last_start_tag: Option<&str>) -> Result<Vec<Norm>, ()> {
    let mut t = Tokenizer::new(input);
    t.set_state(state);
    if let Some(name) = last_start_tag {
        t.set_last_start_tag(name);
    }
    let mut out: Vec<Norm> = Vec::new();
    loop {
        let Ok(tok) = t.next_token() else {
            return Err(());
        };
        match tok {
            Token::EndOfFile => break,
            Token::Character(c) => {
                if let Some(Norm::Char(s)) = out.last_mut() {
                    s.push(c);
                } else {
                    out.push(Norm::Char(c.to_string()));
                }
            }
            Token::StartTag(tag) => {
                let attrs = tag.attrs.into_iter().collect();
                out.push(Norm::Start(tag.name, attrs, tag.self_closing));
            }
            Token::EndTag(tag) => out.push(Norm::End(tag.name)),
            Token::Comment(s) => out.push(Norm::Comment(s)),
            Token::Doctype(d) => out.push(Norm::Doctype(
                d.name,
                d.public_id,
                d.system_id,
                !d.force_quirks,
            )),
        }
    }
    Ok(out)
}

/// Convert an html5lib expected-output array into [`Norm`] values.
fn expected_norms(output: &[Value], double_escaped: bool) -> Vec<Norm> {
    let unescape = |s: &str| {
        if double_escaped {
            double_unescape(s)
        } else {
            s.to_string()
        }
    };
    let opt = |v: &Value| match v {
        Value::Null => None,
        Value::String(s) => Some(unescape(s)),
        other => panic!("unexpected DOCTYPE field: {other:?}"),
    };

    let mut norms = Vec::with_capacity(output.len());
    for item in output {
        let arr = item.as_array().expect("token must be an array");
        let kind = arr[0].as_str().expect("token kind string");
        let norm = match kind {
            "Character" => Norm::Char(unescape(arr[1].as_str().unwrap())),
            "Comment" => Norm::Comment(unescape(arr[1].as_str().unwrap())),
            "StartTag" => {
                let name = unescape(arr[1].as_str().unwrap());
                let attrs = arr[2]
                    .as_object()
                    .expect("attrs object")
                    .iter()
                    .map(|(k, v)| (unescape(k), unescape(v.as_str().unwrap())))
                    .collect();
                let self_closing = arr.get(3).and_then(Value::as_bool).unwrap_or(false);
                Norm::Start(name, attrs, self_closing)
            }
            "EndTag" => Norm::End(unescape(arr[1].as_str().unwrap())),
            "DOCTYPE" => Norm::Doctype(
                opt(&arr[1]),
                opt(&arr[2]),
                opt(&arr[3]),
                arr[4].as_bool().expect("correctness bool"),
            ),
            other => panic!("unknown expected token kind: {other}"),
        };
        norms.push(norm);
    }
    norms
}

/// Drive every test in one html5lib suite file.
fn run_suite(file: &str, raw: &str) {
    let doc: Value = serde_json::from_str(raw).expect("parse test suite JSON");
    let tests = doc["tests"].as_array().expect("`tests` array");

    for (idx, test) in tests.iter().enumerate() {
        let desc = test["description"].as_str().unwrap_or("<no description>");
        let double_escaped = test
            .get("doubleEscaped")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let input_raw = test["input"].as_str().expect("input string");
        let input = if double_escaped {
            double_unescape(input_raw)
        } else {
            input_raw.to_string()
        };
        let last_start_tag = test.get("lastStartTag").and_then(Value::as_str);
        let has_errors = test
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|e| !e.is_empty());

        let states: Vec<State> = match test.get("initialStates").and_then(Value::as_array) {
            Some(arr) => arr
                .iter()
                .map(|v| state_from_name(v.as_str().unwrap()))
                .collect(),
            None => vec![State::Data],
        };

        for state in states {
            let got = normalize_run(&input, state, last_start_tag);
            if has_errors {
                assert!(
                    got.is_err(),
                    "{file}#{idx} ({desc}) [{state:?}]: expected strict reject, got {got:?}"
                );
            } else {
                let output = test["output"].as_array().expect("output array");
                let want = expected_norms(output, double_escaped);
                let got = got.unwrap_or_else(|()| {
                    panic!("{file}#{idx} ({desc}) [{state:?}]: unexpected strict reject")
                });
                assert_eq!(
                    got, want,
                    "{file}#{idx} ({desc}) [{state:?}] token mismatch"
                );
            }
        }
    }
}

macro_rules! suite_test {
    ($name:ident, $file:literal) => {
        #[test]
        fn $name() {
            run_suite(
                $file,
                include_str!(concat!("../../tests/data/html5lib/tokenizer/", $file)),
            );
        }
    };
}

suite_test!(html5lib_test1, "test1.test");
suite_test!(html5lib_test2, "test2.test");
suite_test!(html5lib_test3, "test3.test");
suite_test!(html5lib_test4, "test4.test");
suite_test!(html5lib_content_model_flags, "contentModelFlags.test");
suite_test!(html5lib_escape_flag, "escapeFlag.test");
suite_test!(html5lib_numeric_entities, "numericEntities.test");
suite_test!(html5lib_named_entities, "namedEntities.test");
