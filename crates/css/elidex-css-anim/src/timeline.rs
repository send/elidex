//! Document timeline for CSS Animations (Web Animations Level 1 §3.3).

/// A document timeline tracks the current time for all animations.
///
/// Time is measured in seconds since the timeline origin (document creation).
#[derive(Clone, Debug)]
pub struct DocumentTimeline {
    /// Current time in seconds.
    current_time: f64,
}

impl DocumentTimeline {
    /// Create a new timeline starting at time 0.
    #[must_use]
    pub fn new() -> Self {
        Self { current_time: 0.0 }
    }

    /// Advance the timeline by `dt` seconds.
    ///
    /// No-ops for non-finite or negative values.
    pub fn advance(&mut self, dt: f64) {
        if dt.is_finite() && dt >= 0.0 {
            self.current_time += dt;
        }
    }

    /// Get the current time in seconds.
    #[must_use]
    pub fn current_time(&self) -> f64 {
        self.current_time
    }

    /// Set the current time directly (for testing or seeking).
    pub fn set_time(&mut self, time: f64) {
        self.current_time = time;
    }
}

impl Default for DocumentTimeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_advance() {
        let mut tl = DocumentTimeline::new();
        assert_eq!(tl.current_time(), 0.0);
        tl.advance(0.016);
        assert!((tl.current_time() - 0.016).abs() < 1e-10);
        tl.advance(0.016);
        assert!((tl.current_time() - 0.032).abs() < 1e-10);
    }

    #[test]
    fn timeline_set_time() {
        let mut tl = DocumentTimeline::new();
        tl.set_time(5.0);
        assert_eq!(tl.current_time(), 5.0);
    }

    #[test]
    fn timeline_default() {
        let tl = DocumentTimeline::default();
        assert_eq!(tl.current_time(), 0.0);
    }
}
