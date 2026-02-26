//! Style resolution context passed to [`CssPropertyHandler::resolve()`](crate::CssPropertyHandler::resolve).

/// Context available during CSS value resolution.
///
/// Provides the information needed to resolve relative CSS values
/// (e.g. `em`, `rem`, `vw`) into absolute values.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleContext {
    /// Viewport width in pixels.
    pub viewport_width: f32,
    /// Viewport height in pixels.
    pub viewport_height: f32,
    /// Parent element's computed font size in pixels (for `em` resolution).
    pub parent_font_size: f32,
    /// Root element's computed font size in pixels (for `rem` resolution).
    pub root_font_size: f32,
}

impl Default for StyleContext {
    fn default() -> Self {
        Self {
            viewport_width: 0.0,
            viewport_height: 0.0,
            parent_font_size: 16.0,
            root_font_size: 16.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_sizes() {
        let ctx = StyleContext::default();
        assert_eq!(ctx.parent_font_size, 16.0);
        assert_eq!(ctx.root_font_size, 16.0);
    }

    #[test]
    fn default_viewport() {
        let ctx = StyleContext::default();
        assert_eq!(ctx.viewport_width, 0.0);
        assert_eq!(ctx.viewport_height, 0.0);
    }

    #[test]
    fn custom_context() {
        let ctx = StyleContext {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            parent_font_size: 14.0,
            root_font_size: 18.0,
        };
        assert_eq!(ctx.viewport_width, 1920.0);
        assert_eq!(ctx.root_font_size, 18.0);
    }
}
