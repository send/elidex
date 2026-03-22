//! Parallel style resolution for sibling elements.
//!
//! When the `parallel` feature is enabled, `build_computed_style` is run
//! in parallel across sibling elements using rayon. The cascade phase
//! (which requires `&EcsDom`) remains sequential.

use std::collections::HashMap;

use elidex_plugin::{ComputedStyle, CssValue};
use rayon::prelude::*;

use crate::resolve::{build_computed_style, ResolveContext};

/// Owned property map — keys and values are owned so they can be sent across threads.
pub(crate) type OwnedPropertyMap = HashMap<String, CssValue>;

/// Convert a borrowed `PropertyMap` to an owned map.
pub(crate) fn to_owned_map(borrowed: &HashMap<&str, &CssValue>) -> OwnedPropertyMap {
    borrowed
        .iter()
        .map(|(&k, &v)| (k.to_string(), v.clone()))
        .collect()
}

/// Minimum number of siblings to trigger parallel resolution.
/// Below this threshold, sequential execution avoids rayon overhead.
const PARALLEL_THRESHOLD: usize = 8;

/// Resolve `build_computed_style` in parallel across sibling inputs.
///
/// Each input is `(OwnedPropertyMap)`. All siblings share the same
/// `parent_style` and `ctx`.
///
/// Returns computed styles in the same order as the input.
pub(crate) fn par_resolve_siblings(
    inputs: &[OwnedPropertyMap],
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> Vec<ComputedStyle> {
    if inputs.len() < PARALLEL_THRESHOLD {
        return inputs
            .iter()
            .map(|owned| build_computed_style_owned(owned, parent_style, ctx))
            .collect();
    }

    inputs
        .par_iter()
        .map(|owned| build_computed_style_owned(owned, parent_style, ctx))
        .collect()
}

/// Build a `ComputedStyle` from an owned property map.
///
/// Converts the owned map to borrowed references and delegates to
/// the existing `build_computed_style`.
fn build_computed_style_owned(
    owned: &OwnedPropertyMap,
    parent_style: &ComputedStyle,
    ctx: &ResolveContext,
) -> ComputedStyle {
    let borrowed: HashMap<&str, &CssValue> = owned.iter().map(|(k, v)| (k.as_str(), v)).collect();
    build_computed_style(&borrowed, parent_style, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use elidex_plugin::{CssColor, CssValue, LengthUnit};

    fn default_ctx() -> ResolveContext {
        ResolveContext {
            viewport: elidex_plugin::Size::new(1280.0, 720.0),
            em_base: 16.0,
            root_font_size: 16.0,
        }
    }

    #[test]
    fn parallel_empty_children() {
        let results = par_resolve_siblings(&[], &ComputedStyle::default(), &default_ctx());
        assert!(results.is_empty());
    }

    #[test]
    fn parallel_single_child() {
        let mut map = OwnedPropertyMap::new();
        map.insert("color".to_string(), CssValue::Color(CssColor::RED));
        let results = par_resolve_siblings(&[map], &ComputedStyle::default(), &default_ctx());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].color, CssColor::RED);
    }

    #[test]
    fn parallel_with_inheritance() {
        let parent = ComputedStyle {
            color: CssColor::RED,
            ..ComputedStyle::default()
        };
        // Empty winners → should inherit color from parent.
        let inputs: Vec<OwnedPropertyMap> = (0..4).map(|_| OwnedPropertyMap::new()).collect();
        let results = par_resolve_siblings(&inputs, &parent, &default_ctx());
        for style in &results {
            assert_eq!(style.color, CssColor::RED);
        }
    }

    #[test]
    fn parallel_mixed_elements() {
        let mut map1 = OwnedPropertyMap::new();
        map1.insert("color".to_string(), CssValue::Color(CssColor::RED));

        let mut map2 = OwnedPropertyMap::new();
        map2.insert(
            "font-size".to_string(),
            CssValue::Length(24.0, LengthUnit::Px),
        );

        let results =
            par_resolve_siblings(&[map1, map2], &ComputedStyle::default(), &default_ctx());
        assert_eq!(results[0].color, CssColor::RED);
        assert_eq!(results[1].font_size, 24.0);
    }

    #[test]
    fn parallel_threshold() {
        // Exactly at threshold — should use rayon path.
        let inputs: Vec<OwnedPropertyMap> = (0..PARALLEL_THRESHOLD)
            .map(|_| OwnedPropertyMap::new())
            .collect();
        let results = par_resolve_siblings(&inputs, &ComputedStyle::default(), &default_ctx());
        assert_eq!(results.len(), PARALLEL_THRESHOLD);
    }

    #[test]
    fn parallel_siblings_match_sequential() {
        let parent = ComputedStyle {
            color: CssColor::new(0, 128, 0, 255),
            font_size: 20.0,
            ..ComputedStyle::default()
        };
        let ctx = default_ctx();

        // Build 20 sibling inputs with varying properties.
        let inputs: Vec<OwnedPropertyMap> = (0..20)
            .map(|i| {
                let mut map = OwnedPropertyMap::new();
                if i % 2 == 0 {
                    map.insert("color".to_string(), CssValue::Color(CssColor::RED));
                }
                if i % 3 == 0 {
                    map.insert(
                        "font-size".to_string(),
                        CssValue::Length(24.0, LengthUnit::Px),
                    );
                }
                map
            })
            .collect();

        // Sequential.
        let sequential: Vec<ComputedStyle> = inputs
            .iter()
            .map(|owned| {
                let borrowed: HashMap<&str, &CssValue> =
                    owned.iter().map(|(k, v)| (k.as_str(), v)).collect();
                build_computed_style(&borrowed, &parent, &ctx)
            })
            .collect();

        // Parallel.
        let parallel = par_resolve_siblings(&inputs, &parent, &ctx);

        assert_eq!(sequential.len(), parallel.len());
        for (s, p) in sequential.iter().zip(parallel.iter()) {
            assert_eq!(s.color, p.color);
            assert_eq!(s.font_size, p.font_size);
            assert_eq!(s.display, p.display);
        }
    }
}
