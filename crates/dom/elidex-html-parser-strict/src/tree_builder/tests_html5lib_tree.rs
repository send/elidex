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
//! `#document-fragment` cases are driven through
//! [`crate::parse_fragment_strict`] (WHATWG HTML §13.4): the context element
//! is the tag on the line after `#document-fragment`, and the returned
//! detached roots are serialized and compared, with the same has-errors /
//! no-errors split as the document path.
//!
//! Cases the strict path does not cover are skipped (and counted, not silently
//! dropped): `#script-off` (the strict baseline models scripting enabled), and
//! foreign content — `<svg>` / `<math>` in the data, or a namespaced fragment
//! context (`svg path`, `math ms`), deferred to
//! `#11-strict-fragment-foreign-context` (the fragment-context counterpart of
//! the closed inline-foreign-content slot `#11-html-parser-strict-foreign-content`).

use elidex_ecs::{Attributes, EcsDom};

use super::tests::{serialize_document, serialize_fragment};
use super::TreeBuilder;
use crate::{parse_fragment_strict, ParseFragmentOptions};

/// One parsed `.dat` test vector. The booleans are independent test
/// classifications (has-errors / script-off / foreign content), not a state
/// machine; `context` is `Some(tag)` for a `#document-fragment` case.
#[allow(clippy::struct_excessive_bools)]
struct Case {
    input: String,
    has_errors: bool,
    context: Option<String>,
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
    let mut context: Option<String> = None;
    let mut script_off = false;
    let mut section = Section::None;
    let mut started = false;

    let finish = |data: &mut Vec<&str>,
                  errors: &mut Vec<&str>,
                  document: &mut Vec<&str>,
                  context: &mut Option<String>,
                  script_off: &mut bool,
                  cases: &mut Vec<Case>| {
        // Trailing blank lines in the document section are the inter-case
        // separator, not tree content.
        while document.last() == Some(&"") {
            document.pop();
        }
        let input = data.join("\n");
        let doc = document.join("\n");
        // A namespaced fragment context (`svg path`, `math ms`) is foreign —
        // deferred with the rest of the foreign content.
        let foreign = input.contains("<svg")
            || input.contains("<math")
            || doc.contains("<svg ")
            || doc.contains("<math ")
            || context.as_deref().is_some_and(|c| c.contains(' '));
        cases.push(Case {
            input,
            has_errors: !errors.is_empty(),
            context: context.take(),
            script_off: *script_off,
            foreign,
            document: doc,
        });
        data.clear();
        errors.clear();
        document.clear();
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
                        &mut context,
                        &mut script_off,
                        &mut cases,
                    );
                }
                started = true;
                section = Section::Data;
            }
            "#errors" => section = Section::Errors,
            "#new-errors" | "#script-on" => section = Section::Ignore,
            // The context element is the single line that follows
            // `#document-fragment` (e.g. `td`, or `svg path` for foreign).
            "#document-fragment" => section = Section::FragmentContext,
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
                Section::FragmentContext => {
                    if !line.is_empty() {
                        context = Some(line.to_string());
                        // Only the first line is the context; ignore the rest
                        // until the next section header.
                        section = Section::Ignore;
                    }
                }
                Section::Ignore | Section::None => {}
            },
        }
    }
    if started {
        finish(
            &mut data,
            &mut errors,
            &mut document,
            &mut context,
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
    FragmentContext,
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
        if case.script_off || case.foreign {
            skipped += 1;
            continue;
        }
        // Document vs fragment differ only in how the tree is built and
        // serialized; normalize both to `Ok(serialized tree)` / `Err(reject)`
        // so the has-errors split below is shared.
        let parsed: Result<String, crate::StrictParseError> = match &case.context {
            None => TreeBuilder::build(&case.input).map(|tree| serialize_document(&tree)),
            Some(ctx_tag) => {
                let mut dom = EcsDom::new();
                let ctx = dom.create_element(ctx_tag.as_str(), Attributes::default());
                parse_fragment_strict(&case.input, ctx, &mut dom, ParseFragmentOptions::default())
                    .map(|roots| serialize_fragment(&dom, &roots))
            }
        };
        let label = match &case.context {
            Some(ctx) => format!("{name} fragment[{ctx}]"),
            None => name.to_string(),
        };
        if case.has_errors {
            match parsed {
                Err(_) => reject_ok += 1,
                Ok(_) => failures.push(format!(
                    "{label}: expected strict reject but parsed OK\n  input: {:?}",
                    case.input
                )),
            }
        } else {
            match parsed {
                Ok(got) => {
                    if got.trim_end_matches('\n') == case.document.trim_end_matches('\n') {
                        tree_ok += 1;
                    } else {
                        failures.push(format!(
                            "{label}: tree mismatch\n  input: {:?}\n  expected:\n{}\n  got:\n{}",
                            case.input,
                            case.document,
                            got.trim_end_matches('\n')
                        ));
                    }
                }
                Err(err) => failures.push(format!(
                    "{label}: expected OK but strict rejected ({err})\n  input: {:?}",
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
        (
            "tests_innerHTML_1",
            include_str!("../../tests/data/html5lib/tree-construction/tests_innerHTML_1.dat"),
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
        "html5lib tree-construction: {total_reject} reject-asserts, {total_tree} tree-matches, {total_skip} skipped (script-off / foreign content)"
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
