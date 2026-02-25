use std::collections::HashMap;

pub mod css;
pub mod html;
pub mod js;
mod util;

/// Feature occurrence counts keyed by feature name.
pub type FeatureCount = HashMap<String, usize>;

/// Maximum number of extraction loop iterations to prevent CPU exhaustion on
/// malformed HTML.
const MAX_EXTRACT_ITERATIONS: usize = 100_000;
