//! html5lib tree-construction corpus harness for the strict tree builder.
//!
//! Runs the vendored html5lib-tests `tree-construction/*.dat` vectors (MIT,
//! `tests/data/html5lib/`) against [`super::TreeBuilder`]. Because strict mode
//! has no error recovery, the corpus splits cleanly:
//!
//! - a test whose `#errors` list is **non-empty** describes non-conforming
//!   input, so `build` must abort with [`crate::StrictParseError`];
//! - a test with **no** errors is conforming HTML5, so the built tree, dumped
//!   in the html5lib `#document` format, must match exactly.
//!
//! Cases the document-parse path does not cover are skipped (and
//! counted, not silently dropped): `#document-fragment` (strict fragment
//! parsing is not implemented — fragment parsing uses the compat path),
//! `#script-off` (the strict baseline models scripting enabled), and
//! foreign content (`<svg>` / `<math>`, deferred —
//! `#11-html-parser-strict-foreign-content`).

use super::tests::serialize_document;
use super::TreeBuilder;

/// One parsed `.dat` test vector. The four booleans are independent test
/// classifications (has-errors / fragment / script-off / foreign content), not
/// a state machine.
#[allow(clippy::struct_excessive_bools)]
struct Case {
    input: String,
    has_errors: bool,
    fragment: bool,
    script_off: bool,
    foreign: bool,
    document: String,
}

/// Parse an html5lib `.dat` file into its test cases.
///
/// The format is a line-oriented sequence of `#data` / `#errors` /
/// `#document` (plus optional `#new-errors`, `#document-fragment`,
/// `#script-on`, `#script-off`) sections; `#data` begins a new case. The
/// input is the `#data` lines joined with newlines (no trailing newline); the
/// expected tree is the `#document` lines joined likewise.
fn parse_dat(text: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    let mut data: Vec<&str> = Vec::new();
    let mut errors: Vec<&str> = Vec::new();
    let mut document: Vec<&str> = Vec::new();
    let mut fragment = false;
    let mut script_off = false;
    let mut section = Section::None;
    let mut started = false;

    let finish = |data: &mut Vec<&str>,
                  errors: &mut Vec<&str>,
                  document: &mut Vec<&str>,
                  fragment: &mut bool,
                  script_off: &mut bool,
                  cases: &mut Vec<Case>| {
        // Trailing blank lines in the document section are the inter-case
        // separator, not tree content.
        while document.last() == Some(&"") {
            document.pop();
        }
        let input = data.join("\n");
        let doc = document.join("\n");
        let foreign = input.contains("<svg")
            || input.contains("<math")
            || doc.contains("<svg ")
            || doc.contains("<math ");
        cases.push(Case {
            input,
            has_errors: !errors.is_empty(),
            fragment: *fragment,
            script_off: *script_off,
            foreign,
            document: doc,
        });
        data.clear();
        errors.clear();
        document.clear();
        *fragment = false;
        *script_off = false;
    };

    for line in text.lines() {
        match line {
            "#data" => {
                if started {
                    finish(
                        &mut data,
                        &mut errors,
                        &mut document,
                        &mut fragment,
                        &mut script_off,
                        &mut cases,
                    );
                }
                started = true;
                section = Section::Data;
            }
            "#errors" => section = Section::Errors,
            "#new-errors" | "#script-on" => section = Section::Ignore,
            "#document-fragment" => {
                fragment = true;
                section = Section::Ignore;
            }
            "#script-off" => {
                script_off = true;
                section = Section::Ignore;
            }
            "#document" => section = Section::Document,
            _ => match section {
                Section::Data => data.push(line),
                Section::Errors => {
                    if !line.is_empty() {
                        errors.push(line);
                    }
                }
                Section::Document => document.push(line),
                Section::Ignore | Section::None => {}
            },
        }
    }
    if started {
        finish(
            &mut data,
            &mut errors,
            &mut document,
            &mut fragment,
            &mut script_off,
            &mut cases,
        );
    }
    cases
}

enum Section {
    None,
    Data,
    Errors,
    Document,
    Ignore,
}

/// Run one suite, collecting failures rather than aborting on the first, and
/// returning `(reject_ok, tree_ok, skipped, failures)`.
fn run_suite(name: &str, raw: &str) -> (usize, usize, usize, Vec<String>) {
    let mut reject_ok = 0;
    let mut tree_ok = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();
    for case in parse_dat(raw) {
        if case.fragment || case.script_off || case.foreign {
            skipped += 1;
            continue;
        }
        let result = TreeBuilder::build(&case.input);
        if case.has_errors {
            match result {
                Err(_) => reject_ok += 1,
                Ok(_) => failures.push(format!(
                    "{name}: expected strict reject but parsed OK\n  input: {:?}",
                    case.input
                )),
            }
        } else {
            match result {
                Ok(parsed) => {
                    let got = serialize_document(&parsed);
                    if got.trim_end_matches('\n') == case.document.trim_end_matches('\n') {
                        tree_ok += 1;
                    } else {
                        failures.push(format!(
                            "{name}: tree mismatch\n  input: {:?}\n  expected:\n{}\n  got:\n{}",
                            case.input,
                            case.document,
                            got.trim_end_matches('\n')
                        ));
                    }
                }
                Err(err) => failures.push(format!(
                    "{name}: expected OK but strict rejected ({err})\n  input: {:?}",
                    case.input
                )),
            }
        }
    }
    (reject_ok, tree_ok, skipped, failures)
}

#[test]
fn html5lib_tree_construction_corpus() {
    let suites: &[(&str, &str)] = &[
        (
            "tests1",
            include_str!("../../tests/data/html5lib/tree-construction/tests1.dat"),
        ),
        (
            "tests2",
            include_str!("../../tests/data/html5lib/tree-construction/tests2.dat"),
        ),
        (
            "doctype01",
            include_str!("../../tests/data/html5lib/tree-construction/doctype01.dat"),
        ),
    ];

    let mut total_reject = 0;
    let mut total_tree = 0;
    let mut total_skip = 0;
    let mut all_failures = Vec::new();
    for (name, raw) in suites {
        let (reject_ok, tree_ok, skipped, failures) = run_suite(name, raw);
        total_reject += reject_ok;
        total_tree += tree_ok;
        total_skip += skipped;
        all_failures.extend(failures);
    }

    // Surface coverage so skipped cases are visible, not silently dropped.
    println!(
        "html5lib tree-construction: {total_reject} reject-asserts, {total_tree} tree-matches, {total_skip} skipped (fragment / script-off / foreign content)"
    );

    assert!(
        all_failures.is_empty(),
        "{} html5lib case(s) failed:\n\n{}",
        all_failures.len(),
        all_failures.join("\n\n")
    );
    // Guard against the corpus silently becoming a no-op.
    assert!(total_reject > 100, "expected many reject assertions");
    assert!(total_tree > 10, "expected conforming tree-match cases");
}
