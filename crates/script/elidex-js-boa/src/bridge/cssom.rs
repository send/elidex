//! CSSOM (CSS Object Model) methods for `HostBridge`.
//!
//! Manages lightweight stylesheet representations for JS access and
//! records pending mutations for the content thread to apply.

use super::{parse_cssom_rule_from_text, CssomMutation, CssomRule, CssomSheet, HostBridge};

impl HostBridge {
    /// Replace the CSSOM stylesheet list (called by content thread after pipeline).
    pub fn set_stylesheets(&self, sheets: Vec<CssomSheet>) {
        self.inner.borrow_mut().stylesheets = sheets;
    }

    /// Get the number of stylesheets.
    #[must_use]
    pub fn stylesheet_count(&self) -> usize {
        self.inner.borrow().stylesheets.len()
    }

    /// Access a stylesheet's rules by index.
    #[must_use]
    pub fn stylesheet_rules(&self, sheet_index: usize) -> Option<Vec<CssomRule>> {
        self.inner
            .borrow()
            .stylesheets
            .get(sheet_index)
            .map(|s| s.rules.clone())
    }

    /// Insert a rule into a stylesheet's CSSOM representation and record a pending mutation.
    ///
    /// Returns the actual insertion index on success, or `None` if the sheet/index is invalid.
    pub fn cssom_insert_rule(
        &self,
        sheet_index: usize,
        rule_index: usize,
        rule_text: &str,
    ) -> Option<usize> {
        let mut inner = self.inner.borrow_mut();
        let sheet = inner.stylesheets.get_mut(sheet_index)?;
        if rule_index > sheet.rules.len() {
            return None;
        }
        // Validation uses a lightweight parser; the content thread reparses with
        // the full CSS parser for spec-compliant handling. Discrepancies are
        // acceptable since the full parser is authoritative.
        let rule = parse_cssom_rule_from_text(rule_text)?;
        sheet.rules.insert(rule_index, rule);
        inner.cssom_mutations.push(CssomMutation::InsertRule {
            sheet_index,
            rule_index,
            rule_text: rule_text.to_string(),
        });
        Some(rule_index)
    }

    /// Delete a rule from a stylesheet's CSSOM representation and record a pending mutation.
    ///
    /// Returns `true` on success, `false` if the sheet/index is invalid.
    pub fn cssom_delete_rule(&self, sheet_index: usize, rule_index: usize) -> bool {
        let mut inner = self.inner.borrow_mut();
        let Some(sheet) = inner.stylesheets.get_mut(sheet_index) else {
            return false;
        };
        if rule_index >= sheet.rules.len() {
            return false;
        }
        sheet.rules.remove(rule_index);
        inner.cssom_mutations.push(CssomMutation::DeleteRule {
            sheet_index,
            rule_index,
        });
        true
    }

    /// Take all pending CSSOM mutations (consumed by content thread).
    pub fn take_cssom_mutations(&self) -> Vec<CssomMutation> {
        std::mem::take(&mut self.inner.borrow_mut().cssom_mutations)
    }

    /// Returns `true` if there are pending CSSOM mutations.
    #[must_use]
    pub fn has_cssom_mutations(&self) -> bool {
        !self.inner.borrow().cssom_mutations.is_empty()
    }
}
