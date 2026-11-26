//! The parsed stylesheet Arc is issued by the owner's version-stamped ECS cache.
//! Keep one capture per live version and select the last rule in document order.
use super::{AnimationEngine, IndexedRule};
use elidex_css::Stylesheet;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

#[derive(Debug)]
pub(super) struct SheetCapture {
    version: Weak<Stylesheet>,
    rules: Vec<Arc<IndexedRule>>,
}

impl SheetCapture {
    fn new(sheet: &Stylesheet, version: Weak<Stylesheet>) -> Self {
        let registry = sheet.grammar_registry.as_deref();
        let rules = sheet
            .keyframes_raw
            .iter()
            .map(|(name, body)| {
                let rule = crate::parse::parse_keyframes_with_registry(name, body, registry);
                Arc::new(IndexedRule::new(rule, registry))
            })
            .collect();
        Self { version, rules }
    }
}

impl AnimationEngine {
    /// Seed from the construction-time cascade before DOM version stamps are available.
    pub fn from_stylesheets(sheets: &[Stylesheet]) -> Self {
        let mut engine = Self::new();
        let captures = sheets
            .iter()
            .filter(|s| !s.keyframes_raw.is_empty())
            .map(|sheet| SheetCapture::new(sheet, Weak::new()))
            .collect();
        engine.install_captures(captures);
        engine
    }

    /// Update only changed captures, including membership and source-order changes.
    pub fn sync_stylesheets(&mut self, sheets: &[Arc<Stylesheet>]) {
        let relevant = sheets.iter().filter(|s| !s.keyframes_raw.is_empty());
        if self
            .sheets
            .iter()
            .map(|s| s.version.as_ptr())
            .eq(relevant.clone().map(Arc::as_ptr))
        {
            return;
        }
        let mut previous: HashMap<_, _> = self
            .sheets
            .drain(..)
            .map(|s| (s.version.as_ptr(), s))
            .collect();
        let captures = relevant
            .map(|sheet| {
                previous
                    .remove(&Arc::as_ptr(sheet))
                    .unwrap_or_else(|| SheetCapture::new(sheet, Arc::downgrade(sheet)))
            })
            .collect();
        self.install_captures(captures);
    }

    fn install_captures(&mut self, captures: Vec<SheetCapture>) {
        self.stylesheet_rules.clear();
        for capture in &captures {
            for indexed in &capture.rules {
                self.standalone.remove(&indexed.rule.name);
                self.stylesheet_rules
                    .insert(indexed.rule.name.clone(), Arc::clone(indexed));
            }
        }
        self.sheets = captures;
    }
}
