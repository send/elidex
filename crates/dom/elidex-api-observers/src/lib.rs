//! Observer API implementations (`MutationObserver`, `ResizeObserver`, `IntersectionObserver`).
//!
//! Engine-independent implementations that can be bound to any JS engine.

pub mod intersection;
pub mod mutation;
pub mod resize;
