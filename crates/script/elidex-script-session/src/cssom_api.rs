//! CSSOM API handler trait for script-engine method dispatch.

define_api_handler!(
    /// Handler for a single CSSOM API method.
    ///
    /// Implementations of this trait register with a [`PluginRegistry`] and are
    /// dispatched by the script engine when JS code calls a CSSOM method
    /// (e.g. `element.style.setProperty()`, `document.styleSheets`).
    ///
    /// [`PluginRegistry`]: elidex_plugin::PluginRegistry
    CssomApiHandler,
    elidex_plugin::CssSpecLevel,
    elidex_plugin::CssSpecLevel::Standard
);
