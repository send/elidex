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
    cycle: bool,
    depth: usize,
}

pub(super) struct Environment<'a> {
    inherited: &'a ComputedMap,
    locals: HashMap<String, Sequence>,
    cache: HashMap<String, Cached>,
    active: Vec<Active>,
    deferred: Option<String>,
}

impl<'a> Environment<'a> {
    pub(super) fn new(inherited: &'a ComputedMap, locals: HashMap<String, Sequence>) -> Self {
        Self {
            inherited,
            locals,
            cache: HashMap::new(),
            active: Vec::new(),
            deferred: None,
        }
    }

    fn lookup(&mut self, name: &str) -> Result<ComputedCustom, Failure> {
        if let Some(index) = self.active.iter().position(|entry| entry.name == name) {
            for entry in &mut self.active[index..] {
                entry.cycle = true;
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
                cycle: false,
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
            let value = if active.cycle {
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
        while i < range.end {
            let call = if early_only {
                if i + 3 < range.end
                    && items[i..i + 3].iter().all(|t| t.kind == Kind::Delim('.'))
                    && is_var(&items[i + 3].kind)
                {
                    i + 3
                } else {
                    i = items[i].end;
                    continue;
                }
            } else if is_var(&items[i].kind) {
                i
            } else {
                i += 1;
                continue;
            };
            writer.append(seq, start..i).ok_or(Failure::Invalid)?;
            let replacement = self.call(seq, call, nesting + 1)?;
            writer.replacement(&replacement).ok_or(Failure::Invalid)?;
            i = items[call].end;
            start = i;
        }
        if early_only && start == range.start {
            let source = if range.is_empty() {
                ""
            } else {
                &seq.source()[items[range.start].span.start..items[range.end - 1].span.end]
            };
            return Sequence::fragment(source).ok_or(Failure::Invalid);
        }
        writer
            .append(seq, start..range.end)
            .ok_or(Failure::Invalid)?;
        let source = writer.finish().ok_or(Failure::Invalid)?;
        Sequence::fragment(&source).ok_or(Failure::Invalid)
    }

    fn call(&mut self, seq: &Sequence, index: usize, nesting: usize) -> Result<Sequence, Failure> {
        let call = &seq.items()[index];
        let args = self.expand(
            seq,
            index + 1..call.end - usize::from(call.closed),
            true,
            nesting,
        )?;
        let mut comma = None;
        let mut i = 0;
        while i < args.items().len() {
            if args.items()[i].kind == Kind::Comma {
                comma = Some(i);
                break;
            }
            i = args.items()[i].end;
        }
        let name = match self.expand(
            &args,
            0..comma.unwrap_or(args.items().len()),
            false,
            nesting,
        ) {
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
        let fallback = comma.ok_or(Failure::Invalid)? + 1;
        self.expand(&args, fallback..args.items().len(), false, nesting)
    }
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
