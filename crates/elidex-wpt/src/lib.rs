//! WPT-style test harness for elidex CSS conformance testing.
//!
//! Provides a JSON-based test case format for validating CSS cascade,
//! inheritance, and computed style resolution against expected values.

mod format;
mod harness;
pub mod suites;

pub use format::parse_test_cases;
pub use harness::{run_test_case, run_test_suite, WptTestCase, WptTestResult};
