//! CSS Variables §3 and CSS Values §A: evaluate substitutions in the
//! declaring element's environment, including early invocation and cycles.

use elidex_css::SubstitutionWriter as Writer;
use elidex_plugin::component_values::{
    ComponentKind as Kind, ComponentValues as Sequence, ComputedCustom,
};
use std::{collections::HashMap, ops::Range};

type ComputedMap = HashMap<String, ComputedCustom>;
const MAX_DEPTH: usize = 32;

#[derive(Clone)]
struct Cached {
    value: ComputedCustom,
    depth: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Failure {
    Invalid,
    Depth,
}

struct Active {
    name: String,
    cycle_root: Option<usize>,
    depth: usize,
}

pub(super) struct Environment<'a> {
    inherited: &'a ComputedMap,
    locals: HashMap<String, Sequence>,
    cache: HashMap<String, Cached>,
    active: Vec<Active>,
    // Unwound participants remain reachable until their entire cycle closes.
    unwound: HashMap<String, usize>,
    deferred: Option<String>,
}

impl<'a> Environment<'a> {
    pub(super) fn new(inherited: &'a ComputedMap, locals: HashMap<String, Sequence>) -> Self {
        Self {
            inherited,
            locals,
            cache: HashMap::new(),
            active: Vec::new(),
            unwound: HashMap::new(),
            deferred: None,
        }
    }

    fn lookup(&mut self, name: &str) -> Result<ComputedCustom, Failure> {
        let active_index = self.active.iter().position(|entry| entry.name == name);
        if let Some(index) = active_index.or_else(|| self.unwound.get(name).copied()) {
            let root = self.active[index].cycle_root.unwrap_or(index);
            for entry in &mut self.active[root..] {
                entry.cycle_root = Some(root);
            }
            for earlier in self.unwound.values_mut() {
                *earlier = (*earlier).min(root);
            }
            return Ok(ComputedCustom::Invalid);
        }
        let Some(local) = self.locals.get(name) else {
            return Ok(self
                .inherited
                .get(name)
                .cloned()
                .unwrap_or(ComputedCustom::Invalid));
        };
        let cached = if let Some(entry) = self.cache.get(name) {
            if self.active.len() + entry.depth > MAX_DEPTH {
                return Err(Failure::Depth);
            }
            entry.clone()
        } else {
            if self.active.len() >= MAX_DEPTH {
                return Err(Failure::Depth);
            }
            let local = local.clone();
            self.active.push(Active {
                name: name.to_owned(),
                cycle_root: None,
                depth: 1,
            });
            let result = if local.contains_var() {
                self.expand(&local, 0..local.items().len(), false, 0)
            } else {
                Ok(local)
            }
            .map(|result| match sole_ident(&result) {
                Some(ident) if ident.eq_ignore_ascii_case("initial") => ComputedCustom::Invalid,
                Some(ident)
                    if ident.eq_ignore_ascii_case("inherit")
                        || ident.eq_ignore_ascii_case("unset") =>
                {
                    self.inherited
                        .get(name)
                        .cloned()
                        .unwrap_or(ComputedCustom::Invalid)
                }
                _ => ComputedCustom::Value(result),
            });
            let active = self.active.pop().expect("active custom property");
            if matches!(result, Err(Failure::Depth)) {
                if self.active.len() == 1 {
                    // This child exhausted the caller's remaining budget. Establish
                    // its own computed value from a full budget before retrying.
                    self.deferred = Some(name.to_owned());
                }
                if !self.active.is_empty() || self.deferred.is_some() {
                    return Err(Failure::Depth);
                }
            }
            let value = if active.cycle_root.is_some() {
                ComputedCustom::Invalid
            } else {
                result.unwrap_or(ComputedCustom::Invalid)
            };
            let entry = Cached {
                depth: if value == ComputedCustom::Invalid {
                    0
                } else {
                    active.depth
                },
                value,
            };
            if let Some(root) = active.cycle_root {
                if root < self.active.len() {
                    self.unwound.insert(name.to_owned(), root);
                    return Ok(ComputedCustom::Invalid);
                }
                self.unwound.retain(|member, anchor| {
                    if *anchor != root {
                        return true;
                    }
                    self.cache.insert(member.clone(), entry.clone());
                    false
                });
            }
            self.cache.insert(name.to_owned(), entry.clone());
            entry
        };
        if let Some(parent) = self.active.last_mut() {
            parent.depth = parent.depth.max(cached.depth + 1);
        }
        Ok(cached.value)
    }

    /// Resolve each deferred dependency with its own budget. The work list
    /// contains root requests, never nested Rust calls or partial results.
    fn root(&mut self, name: &str) -> ComputedCustom {
        let mut work = vec![name.to_owned()];
        let mut pending = HashMap::from([(name.to_owned(), 0)]);
        while let Some(request) = work.last() {
            let result = self.lookup(request);
            self.unwound.clear();
            if let Some(dependency) = self.deferred.take() {
                if let Some(&start) = pending.get(&dependency) {
                    for participant in &work[start..] {
                        self.cache.insert(
                            participant.clone(),
                            Cached {
                                value: ComputedCustom::Invalid,
                                depth: 0,
                            },
                        );
                    }
                } else {
                    pending.insert(dependency.clone(), work.len());
                    work.push(dependency);
                }
            } else {
                pending.remove(request);
                work.pop();
                if work.is_empty() {
                    return result.unwrap_or(ComputedCustom::Invalid);
                }
            }
        }
        unreachable!("work list starts with a root request")
    }

    pub(super) fn computed(mut self) -> ComputedMap {
        let mut result = self.inherited.clone();
        for name in self.locals.keys().cloned().collect::<Vec<_>>() {
            result.insert(name.clone(), self.root(&name));
        }
        result
    }

    pub(super) fn ordinary(&mut self, value: &Sequence) -> Option<Sequence> {
        self.expand(value, 0..value.items().len(), false, 0).ok()
    }

    fn expand(
        &mut self,
        seq: &Sequence,
        range: Range<usize>,
        early_only: bool,
        nesting: usize,
    ) -> Result<Sequence, Failure> {
        if nesting > MAX_DEPTH {
            return Err(Failure::Depth);
        }
        let items = seq.items();
        let mut writer = Writer::default();
        let mut i = range.start;
        let mut start = i;
        let mut invalid = false;
        while i < range.end {
            let call = if early_only {
                let Some(call) = next_spread(seq, i..range.end) else {
                    break;
                };
                i = call - 3;
                call
            } else if is_var(&items[i].kind) {
                i
            } else {
                i += 1;
                continue;
            };
            if !invalid {
                invalid = if early_only {
                    writer.append_arguments(seq, start..i)
                } else {
                    writer.append(seq, start..i)
                }
                .is_none();
            }
            match self.call(seq, call, nesting + 1) {
                Ok(value) if !invalid => {
                    if i == range.start && items[call].end == range.end {
                        if value
                            .items()
                            .last()
                            .is_some_and(|t| t.span.end > Writer::MAX_BYTES)
                        {
                            return Err(Failure::Invalid);
                        }
                        return Ok(value);
                    }
                    invalid = writer.replacement(&value).is_none();
                }
                Ok(_) => {}
                Err(Failure::Invalid) => invalid = true,
                Err(Failure::Depth) => return Err(Failure::Depth),
            }
            i = items[call].end;
            start = i;
        }
        if invalid {
            return Err(Failure::Invalid);
        }
        if early_only {
            writer.append_arguments(seq, start..range.end)
        } else {
            writer.append(seq, start..range.end)
        }
        .ok_or(Failure::Invalid)?;
        writer.finish().ok_or(Failure::Invalid)
    }

    fn call(&mut self, seq: &Sequence, index: usize, nesting: usize) -> Result<Sequence, Failure> {
        let call = &seq.items()[index];
        let range = index + 1..call.end - usize::from(call.closed);
        let early;
        let (args, range) = if next_spread(seq, range.clone()).is_some() {
            early = self.expand(seq, range, true, nesting)?;
            (&early, 0..early.items().len())
        } else {
            (seq, range)
        };
        let (name_range, fallback) = args.variable_arguments(range).ok_or(Failure::Invalid)?;
        let name = match self.expand(args, name_range, false, nesting) {
            Ok(name) => Some(name),
            Err(Failure::Invalid) => None,
            Err(Failure::Depth) => return Err(Failure::Depth),
        };
        if let Some(ident) = name
            .as_ref()
            .and_then(sole_ident)
            .filter(|n| n.starts_with("--") && *n != "--")
        {
            if let ComputedCustom::Value(result) = self.lookup(ident)? {
                return Ok(result);
            }
        }
        let fallback = fallback.ok_or(Failure::Invalid)?;
        self.expand(args, fallback, false, nesting)
    }
}

/// CSS Values 5 Appendix A: descend in preorder except into another var().
fn next_spread(seq: &Sequence, range: Range<usize>) -> Option<usize> {
    let items = seq.items();
    let mut i = range.start;
    while i < range.end {
        if i + 3 < range.end
            && items[i..i + 3].iter().all(|t| t.kind == Kind::Delim('.'))
            && is_var(&items[i + 3].kind)
        {
            return Some(i + 3);
        }
        i = if is_var(&items[i].kind) {
            items[i].end
        } else {
            i + 1
        };
    }
    None
}

fn is_var(kind: &Kind) -> bool {
    matches!(kind, Kind::Function(name) if name.eq_ignore_ascii_case("var"))
}

fn sole_ident(seq: &Sequence) -> Option<&str> {
    let mut components = seq
        .items()
        .iter()
        .filter(|c| !matches!(c.kind, Kind::Whitespace | Kind::Comment));
    let Kind::Ident(name) = &components.next()?.kind else {
        return None;
    };
    components.next().is_none().then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_failure_visits_sibling_dependencies_in_both_root_orders() {
        let big = "x".repeat(Writer::MAX_BYTES + 1);
        for body in ["var(--big) var(--b)".to_owned(), format!("{big} var(--b)")] {
            let locals = HashMap::from([
                ("--big".into(), Sequence::capture(&big).unwrap()),
                ("--a".into(), Sequence::capture(&body).unwrap()),
                ("--b".into(), Sequence::capture("var(--a,2px)").unwrap()),
            ]);
            let inherited = HashMap::new();
            for first in ["--a", "--b"] {
                let mut env = Environment::new(&inherited, locals.clone());
                assert_eq!(env.root(first), ComputedCustom::Invalid);
                let width = Sequence::capture("var(--b,7px)").unwrap();
                assert_eq!(env.ordinary(&width).unwrap().source(), "7px");
            }
        }
    }

    #[test]
    fn publishes_a_cycle_only_when_all_participants_have_unwound() {
        let locals = HashMap::from([
            (
                "--a".into(),
                Sequence::capture("var(--b,var(--c))").unwrap(),
            ),
            (
                "--b".into(),
                Sequence::capture("var(--b,var(--a))").unwrap(),
            ),
            ("--c".into(), Sequence::capture("var(--b,1px)").unwrap()),
        ]);
        let inherited = HashMap::new();
        for a in locals.keys() {
            for b in locals.keys().filter(|b| *b != a) {
                let c = locals.keys().find(|c| *c != a && *c != b).unwrap();
                let mut env = Environment::new(&inherited, locals.clone());
                for name in [a, b, c] {
                    assert_eq!(
                        env.root(name),
                        ComputedCustom::Invalid,
                        "{a},{b},{c}: {name}"
                    );
                }
                let width = Sequence::capture("var(--c,7px)").unwrap();
                assert_eq!(env.ordinary(&width).unwrap().source(), "7px");
            }
        }
    }

    #[test]
    fn depth_failure_and_caller_fallback_are_order_independent() {
        for count in [32, 33] {
            let mut locals = HashMap::new();
            for i in 1..=count {
                let source = if i == count {
                    "2px".to_owned()
                } else {
                    format!("var(--n{})", i + 1)
                };
                locals.insert(format!("--n{i}"), Sequence::capture(&source).unwrap());
            }
            locals.insert(
                "--outer".to_owned(),
                Sequence::capture("var(--n1,7px)").unwrap(),
            );
            let inherited = HashMap::new();
            for warm in [false, true] {
                let mut env = Environment::new(&inherited, locals.clone());
                if warm {
                    env.root("--n1");
                }
                let expected = if count == 33 {
                    ComputedCustom::Value(Sequence::capture("7px").unwrap())
                } else {
                    ComputedCustom::Invalid
                };
                assert_eq!(env.root("--outer"), expected, "count={count}, warm={warm}");
            }
        }
    }
}
