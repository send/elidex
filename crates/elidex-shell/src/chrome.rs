//! Browser chrome UI (address bar, navigation buttons).
//!
//! Renders an egui overlay at the top of the window containing
//! back/forward buttons, a reload button, and an address bar.

/// Height of the chrome bar in logical pixels.
pub const CHROME_HEIGHT: f32 = 36.0;

/// Chrome UI state.
pub struct ChromeState {
    /// Current text in the address bar.
    pub address_text: String,
    /// Whether the address bar is focused (suppresses content key events).
    pub address_focused: bool,
}

/// Action requested by the chrome UI.
pub enum ChromeAction {
    /// Navigate to the given URL string.
    Navigate(String),
    /// Go back in history.
    Back,
    /// Go forward in history.
    Forward,
    /// Reload the current page.
    Reload,
}

impl ChromeState {
    /// Create a new chrome state, optionally seeded with the current URL.
    pub fn new(url: Option<&url::Url>) -> Self {
        Self {
            address_text: url.map_or_else(String::new, ToString::to_string),
            address_focused: false,
        }
    }

    /// Update the address bar text to reflect the current URL.
    pub fn set_url(&mut self, url: &url::Url) {
        self.address_text = url.to_string();
    }

    /// Build the chrome UI and return any action requested by the user.
    pub fn build(
        &mut self,
        ctx: &egui::Context,
        can_go_back: bool,
        can_go_forward: bool,
    ) -> Option<ChromeAction> {
        let mut action = None;

        egui::TopBottomPanel::top("chrome_bar")
            .exact_height(CHROME_HEIGHT)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Back button.
                    let back_btn = ui.add_enabled(can_go_back, egui::Button::new("<"));
                    if back_btn.clicked() {
                        action = Some(ChromeAction::Back);
                    }

                    // Forward button.
                    let fwd_btn = ui.add_enabled(can_go_forward, egui::Button::new(">"));
                    if fwd_btn.clicked() {
                        action = Some(ChromeAction::Forward);
                    }

                    // Reload button.
                    if ui.button("R").clicked() {
                        action = Some(ChromeAction::Reload);
                    }

                    // Address bar (fills remaining space).
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.address_text)
                            .desired_width(ui.available_width()),
                    );
                    self.address_focused = response.has_focus();

                    // Navigate on Enter.
                    if response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && !self.address_text.is_empty()
                    {
                        action = Some(ChromeAction::Navigate(self.address_text.clone()));
                    }
                });
            });

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_state_new_no_url() {
        let state = ChromeState::new(None);
        assert_eq!(state.address_text, "");
        assert!(!state.address_focused);
    }

    #[test]
    fn chrome_state_new_with_url() {
        let url = url::Url::parse("https://example.com/path").unwrap();
        let state = ChromeState::new(Some(&url));
        assert_eq!(state.address_text, "https://example.com/path");
    }

    #[test]
    fn chrome_state_set_url() {
        let mut state = ChromeState::new(None);
        assert_eq!(state.address_text, "");

        let url = url::Url::parse("https://example.com/new").unwrap();
        state.set_url(&url);
        assert_eq!(state.address_text, "https://example.com/new");
    }
}
