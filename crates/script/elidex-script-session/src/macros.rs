//! Internal macros for the script-session crate.

/// Define an API handler trait with standard method signatures.
///
/// Both `DomApiHandler` and `CssomApiHandler` share the same shape:
/// `method_name()`, `spec_level()` (with a default), and `invoke()`.
/// This macro deduplicates the trait definitions.
macro_rules! define_api_handler {
    (
        $(#[$meta:meta])*
        $trait_name:ident,
        $spec_type:ty,
        $default_level:expr
    ) => {
        $(#[$meta])*
        pub trait $trait_name: Send + Sync {
            /// Human-readable method name (e.g. `"querySelector"`).
            fn method_name(&self) -> &str;

            /// The specification level this handler targets.
            fn spec_level(&self) -> $spec_type {
                $default_level
            }

            /// Execute the handler.
            fn invoke(
                &self,
                this: elidex_ecs::Entity,
                args: &[elidex_plugin::JsValue],
                session: &mut crate::session::SessionCore,
                dom: &mut elidex_ecs::EcsDom,
            ) -> Result<elidex_plugin::JsValue, crate::types::DomApiError>;
        }
    };
}
