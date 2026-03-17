//! Conversion from `DomApiError` to boa `JsNativeError`.

use boa_engine::JsNativeError;
use elidex_script_session::{DomApiError, DomApiErrorKind};

/// Convert a [`DomApiError`] to a boa [`JsNativeError`].
pub fn dom_error_to_js_error(err: DomApiError) -> JsNativeError {
    match err.kind {
        DomApiErrorKind::TypeError => JsNativeError::typ().with_message(err.message),
        DomApiErrorKind::SyntaxError => JsNativeError::syntax().with_message(err.message),
        DomApiErrorKind::NotFoundError | DomApiErrorKind::HierarchyRequestError => {
            // Map DOM-specific errors to TypeError (closest standard JS error).
            JsNativeError::typ().with_message(format!("{}: {}", err.kind, err.message))
        }
        DomApiErrorKind::InvalidStateError => {
            JsNativeError::typ().with_message(format!("InvalidStateError: {}", err.message))
        }
        DomApiErrorKind::IndexSizeError => {
            JsNativeError::range().with_message(format!("IndexSizeError: {}", err.message))
        }
        DomApiErrorKind::InvalidCharacterError => {
            JsNativeError::typ().with_message(format!("InvalidCharacterError: {}", err.message))
        }
        DomApiErrorKind::InUseAttributeError => {
            JsNativeError::typ().with_message(format!("InUseAttributeError: {}", err.message))
        }
        DomApiErrorKind::NotSupportedError => {
            JsNativeError::typ().with_message(format!("NotSupportedError: {}", err.message))
        }
        DomApiErrorKind::Other | _ => JsNativeError::error().with_message(err.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_error_conversion() {
        let err = DomApiError {
            kind: DomApiErrorKind::TypeError,
            message: "wrong type".into(),
        };
        let js_err = dom_error_to_js_error(err);
        assert_eq!(js_err.to_string(), "TypeError: wrong type");
    }

    #[test]
    fn syntax_error_conversion() {
        let err = DomApiError {
            kind: DomApiErrorKind::SyntaxError,
            message: "bad selector".into(),
        };
        let js_err = dom_error_to_js_error(err);
        assert_eq!(js_err.to_string(), "SyntaxError: bad selector");
    }

    #[test]
    fn not_found_maps_to_type_error() {
        let err = DomApiError {
            kind: DomApiErrorKind::NotFoundError,
            message: "node missing".into(),
        };
        let js_err = dom_error_to_js_error(err);
        let s = js_err.to_string();
        assert!(s.contains("NotFoundError"));
    }
}
