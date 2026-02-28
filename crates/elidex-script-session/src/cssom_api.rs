//! CSSOM API handler trait for script-engine method dispatch.

use elidex_ecs::{EcsDom, Entity};
use elidex_plugin::{CssSpecLevel, JsValue};

use crate::session::SessionCore;
use crate::types::DomApiError;

/// Handler for a single CSSOM API method.
///
/// Implementations of this trait register with a [`PluginRegistry`] and are
/// dispatched by the script engine when JS code calls a CSSOM method
/// (e.g. `element.style.setProperty()`, `document.styleSheets`).
///
/// [`PluginRegistry`]: elidex_plugin::PluginRegistry
pub trait CssomApiHandler: Send + Sync {
    /// Returns the CSSOM method name (e.g. `"setProperty"`, `"getComputedStyle"`).
    fn method_name(&self) -> &str;

    /// Returns the specification level of this CSSOM API method.
    fn spec_level(&self) -> CssSpecLevel {
        CssSpecLevel::Standard
    }

    /// Invoke the CSSOM method.
    ///
    /// # Parameters
    ///
    /// - `this` — The entity on which the method is called.
    /// - `args` — JS arguments passed to the method.
    /// - `session` — The session core for identity mapping and mutation recording.
    /// - `dom` — The ECS DOM for direct reads and entity creation.
    ///
    /// # Errors
    ///
    /// Returns `DomApiError` if the operation fails.
    fn invoke(
        &self,
        this: Entity,
        args: &[JsValue],
        session: &mut SessionCore,
        dom: &mut EcsDom,
    ) -> Result<JsValue, DomApiError>;
}
