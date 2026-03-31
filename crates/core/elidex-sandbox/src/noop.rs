//! No-op sandbox enforcer for `SingleProcess` mode.

use elidex_plugin::{SandboxError, SandboxPolicy};

use crate::SandboxEnforcer;

/// No-op enforcer that accepts any policy without applying restrictions.
///
/// Used in `SingleProcess` mode where the content thread shares an address
/// space with the browser thread and cannot be sandboxed at the OS level.
pub struct NoopEnforcer;

impl SandboxEnforcer for NoopEnforcer {
    fn apply(&self, _policy: &SandboxPolicy) -> Result<(), SandboxError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "noop"
    }
}
