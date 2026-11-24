//! CSS Animations 1 §§3, 3.1: offsets and interval timing belong to each animated property.
use crate::{
    parse::{IntervalTiming, KeyframesRule},
    timing::TimingFunction,
};
use elidex_plugin::{CssPropertyRegistry, CssValue, LengthUnit};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
struct Entry {
    frame: usize,
    declaration: usize,
}

#[derive(Debug)]
pub(super) struct IndexedRule {
    pub(super) rule: KeyframesRule,
    tracks: BTreeMap<String, Vec<Entry>>,
}

impl IndexedRule {
    pub(super) fn new(rule: KeyframesRule, registry: Option<&CssPropertyRegistry>) -> Self {
        let mut tracks = BTreeMap::<String, Vec<Entry>>::new();
        for (frame, block) in rule.keyframes.iter().enumerate() {
            for (declaration, decl) in block.declarations.iter().enumerate() {
                let members = if matches!(decl.value, CssValue::Pending(_)) {
                    elidex_css::declaration_members(&decl.property, registry)
                } else {
                    vec![decl.property.clone()]
                };
                for property in members {
                    let track = tracks.entry(property).or_default();
                    let entry = Entry { frame, declaration };
                    if let Some(last) = track.last_mut().filter(|last| last.frame == frame) {
                        *last = entry;
                    } else {
                        track.push(entry);
                    }
                }
            }
        }
        Self { rule, tracks }
    }

    pub(super) fn sample(
        &self,
        p: f32,
        timing: Option<&TimingFunction>,
        target: &super::KeyframeTarget<'_>,
    ) -> Vec<(String, CssValue)> {
        self.tracks
            .iter()
            .filter_map(|(name, entries)| {
                let index =
                    entries.partition_point(|entry| self.rule.keyframes[entry.frame].offset <= p);
                let before = index.checked_sub(1).map(|i| entries[i]);
                let after = entries.get(index).copied();
                let offset = |entry: Entry| self.rule.keyframes[entry.frame].offset;
                let start = before.map_or(0., offset);
                let end = after.map_or(1., offset);
                let reference = if name == "font-size" {
                    target.font_lengths
                } else {
                    target.lengths
                };
                let endpoint = |entry: Option<Entry>| {
                    let value = match entry {
                        Some(entry) => {
                            &self.rule.keyframes[entry.frame].declarations[entry.declaration].value
                        }
                        None => target.underlying?.get(name)?,
                    };
                    let computed;
                    let value = if matches!(
                        value,
                        CssValue::Initial | CssValue::Inherit | CssValue::Unset
                    ) {
                        computed = (target.defaulted)(name, value);
                        &computed
                    } else {
                        value
                    };
                    Some(match value {
                        CssValue::Length(v, unit) => CssValue::Length(
                            elidex_plugin::css_resolve::resolve_length(*v, *unit, reference),
                            LengthUnit::Px,
                        ),
                        CssValue::TransformList(functions) => CssValue::TransformList(
                            functions
                                .iter()
                                .map(|f| {
                                    elidex_plugin::css_resolve::resolve_transform_func(f, reference)
                                })
                                .collect(),
                        ),
                        CssValue::Number(_) | CssValue::Calc(_)
                            if matches!(name.as_str(), "flex-grow" | "flex-shrink") =>
                        {
                            CssValue::Number(elidex_plugin::css_resolve::resolve_flex_factor(
                                value, reference,
                            )?)
                        }
                        _ => value.clone(),
                    })
                };
                let from = endpoint(before);
                let to = endpoint(after);
                let value = match (from, to) {
                    (Some(a), Some(b)) if end > start => {
                        let interval = before
                            .and_then(|e| self.rule.keyframes[e.frame].timing_function.as_ref())
                            .and_then(IntervalTiming::resolved)
                            .or(timing);
                        let t = (p - start) / (end - start);
                        crate::interpolate::interpolate(
                            &a,
                            &b,
                            interval.map_or(t, |f| f.sample(t)),
                            name,
                        )?
                    }
                    (Some(value), _) | (None, Some(value)) => value,
                    (None, None) => return None,
                };
                (!matches!(value, CssValue::Pending(_))).then(|| (name.clone(), value))
            })
            .collect()
    }
}
