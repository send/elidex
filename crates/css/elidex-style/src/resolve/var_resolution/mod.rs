//! CSS Variables §3 computed substitution and CSS Cascade §6.1 member replay.

mod substitute;
#[cfg(test)]
mod substitution_tokens;

use super::helpers::PropertyMap;
use crate::cascade::CascadeEntry;
use elidex_plugin::{component_values::ComputedCustom, ComputedStyle, CssValue};
use std::{borrow::Cow, collections::HashMap};
use substitute::Environment;

pub(crate) struct Resolved<'a> {
    pub(crate) custom: HashMap<String, ComputedCustom>,
    ordinary: HashMap<Cow<'a, str>, (CascadeEntry<'a>, Cow<'a, CssValue>)>,
}

impl Resolved<'_> {
    pub(crate) fn ordinary(&self) -> PropertyMap<'_> {
        self.ordinary
            .iter()
            .map(|(name, (_, value))| (name.as_ref(), value.as_ref()))
            .collect()
    }
}

fn insert<'a, T>(
    map: &mut HashMap<Cow<'a, str>, (CascadeEntry<'a>, T)>,
    key: Cow<'a, str>,
    candidate: CascadeEntry<'a>,
    value: T,
) {
    let entry = map.entry(key);
    if let std::collections::hash_map::Entry::Occupied(previous) = &entry {
        if !candidate.wins(previous.get().0) {
            return;
        }
    }
    entry.insert_entry((candidate, value));
}

pub(crate) fn resolve_candidates<'a>(
    candidates: &[CascadeEntry<'a>],
    parent: &ComputedStyle,
) -> Resolved<'a> {
    let mut locals = HashMap::new();
    for &entry in candidates {
        if let CssValue::Custom(components) = entry.value {
            insert(
                &mut locals,
                Cow::Borrowed(entry.property),
                entry,
                components.clone(),
            );
        }
    }
    let locals = locals
        .into_iter()
        .map(|(name, (_, seq))| (name.into_owned(), seq))
        .collect();
    let custom = Environment::new(&parent.custom_properties, locals).computed();
    let mut ordinary = HashMap::new();
    let mut env = Environment::new(&custom, HashMap::new());
    for &entry in candidates {
        if entry.property.starts_with("--") {
            continue;
        }
        if let CssValue::Pending(seq) = entry.value {
            let expanded = env.ordinary(seq).and_then(|v| {
                elidex_css::parse_value_for_property(entry.property, v.source(), entry.registry)
            });
            if let Some(declarations) = expanded {
                for decl in declarations {
                    insert(
                        &mut ordinary,
                        Cow::Owned(decl.property),
                        entry,
                        Cow::Owned(decl.value),
                    );
                }
            } else {
                for member in elidex_css::declaration_members(entry.property, entry.registry) {
                    insert(
                        &mut ordinary,
                        Cow::Owned(member),
                        entry,
                        Cow::Owned(CssValue::Unset),
                    );
                }
            }
        } else {
            insert(
                &mut ordinary,
                Cow::Borrowed(entry.property),
                entry,
                Cow::Borrowed(entry.value),
            );
        }
    }
    Resolved { custom, ordinary }
}
