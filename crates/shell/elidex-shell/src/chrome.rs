//! Browser chrome UI (address bar, navigation buttons, tab bar).
//!
//! Renders an egui overlay at the top of the window containing
//! back/forward buttons, a reload button, an address bar, and a tab bar.

use elidex_plugin::Point;

use crate::app::tab::TabId;

/// Height of the chrome bar in logical pixels.
pub const CHROME_HEIGHT: f32 = 36.0;

/// Height of the tab bar in logical pixels (horizontal mode).
pub const TAB_BAR_HEIGHT: f32 = 28.0;

/// Width of the tab sidebar in logical pixels (vertical mode).
pub const TAB_SIDEBAR_WIDTH: f32 = 200.0;

/// Tab bar position relative to the content area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TabBarPosition {
    /// Horizontal tab bar above the address bar.
    #[default]
    Top,
    /// Vertical tab sidebar on the left.
    Left,
    /// Vertical tab sidebar on the right.
    Right,
}

/// Chrome UI state.
pub struct ChromeState {
    /// Current text in the address bar.
    pub address_text: String,
    /// Whether the address bar is focused (suppresses content key events).
    pub address_focused: bool,
    /// Tab bar position.
    pub tab_bar_position: TabBarPosition,
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
    /// Open a new tab.
    NewTab,
    /// Close a specific tab.
    CloseTab(TabId),
    /// Switch to a specific tab.
    SwitchTab(TabId),
}

/// Info about a tab, passed to the tab bar builder.
pub struct TabBarInfo {
    /// Tab identifier.
    pub id: TabId,
    /// Tab title text (owned for borrow-free passing).
    pub title: String,
    /// Whether this is the currently active tab.
    pub is_active: bool,
}

impl ChromeState {
    /// Create a new chrome state, optionally seeded with the current URL.
    pub fn new(url: Option<&url::Url>) -> Self {
        Self {
            address_text: url.map_or_else(String::new, ToString::to_string),
            address_focused: false,
            tab_bar_position: TabBarPosition::default(),
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

/// Build the tab bar UI and return any action requested.
///
/// Renders either a horizontal top panel or a side panel depending on `position`.
pub fn build_tab_bar(
    ctx: &egui::Context,
    tabs: &[TabBarInfo],
    position: TabBarPosition,
) -> Option<ChromeAction> {
    match position {
        TabBarPosition::Top => build_tab_bar_top(ctx, tabs),
        TabBarPosition::Left => build_tab_bar_side(ctx, tabs, "tab_sidebar_left", true),
        TabBarPosition::Right => build_tab_bar_side(ctx, tabs, "tab_sidebar_right", false),
    }
}

/// Render a single tab button (label + close [x]) and return any action.
fn render_tab_button(
    ui: &mut egui::Ui,
    tab: &TabBarInfo,
    max_title_len: usize,
) -> Option<ChromeAction> {
    let label = truncate_title(&tab.title, max_title_len);
    let btn = if tab.is_active {
        ui.add(egui::Button::new(egui::RichText::new(&label).strong()))
    } else {
        ui.button(&label)
    };
    if btn.clicked() && !tab.is_active {
        return Some(ChromeAction::SwitchTab(tab.id));
    }
    if ui.small_button("x").clicked() {
        return Some(ChromeAction::CloseTab(tab.id));
    }
    None
}

fn build_tab_bar_top(ctx: &egui::Context, tabs: &[TabBarInfo]) -> Option<ChromeAction> {
    let mut action = None;

    egui::TopBottomPanel::top("tab_bar")
        .exact_height(TAB_BAR_HEIGHT)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                for tab in tabs {
                    if let Some(a) = render_tab_button(ui, tab, 20) {
                        action = Some(a);
                    }
                    ui.separator();
                }

                // New tab button.
                if ui.button("+").clicked() {
                    action = Some(ChromeAction::NewTab);
                }
            });
        });

    action
}

fn build_tab_bar_side(
    ctx: &egui::Context,
    tabs: &[TabBarInfo],
    panel_id: &str,
    is_left: bool,
) -> Option<ChromeAction> {
    let mut action = None;

    let id = egui::Id::new(panel_id);
    let panel = if is_left {
        egui::SidePanel::left(id)
    } else {
        egui::SidePanel::right(id)
    };

    panel.exact_width(TAB_SIDEBAR_WIDTH).show(ctx, |ui| {
        for tab in tabs {
            ui.horizontal(|ui| {
                if let Some(a) = render_tab_button(ui, tab, 25) {
                    action = Some(a);
                }
            });
        }

        // New tab button.
        if ui.button("+").clicked() {
            action = Some(ChromeAction::NewTab);
        }
    });

    action
}

/// Truncate a title to at most `max_len` characters, appending "..." if needed.
fn truncate_title(title: &str, max_len: usize) -> String {
    let char_count = title.chars().count();
    if char_count <= max_len {
        title.to_string()
    } else if max_len < 3 {
        title.chars().take(max_len).collect()
    } else {
        let truncated: String = title.chars().take(max_len - 3).collect();
        format!("{truncated}...")
    }
}

/// Compute the content area offset based on tab bar position.
///
/// Returns the offset that should be subtracted from mouse coordinates
/// to get content-relative positions.
#[must_use]
pub fn chrome_content_offset(position: TabBarPosition) -> Point {
    match position {
        TabBarPosition::Top => Point::new(0.0, TAB_BAR_HEIGHT + CHROME_HEIGHT),
        TabBarPosition::Left => Point::new(TAB_SIDEBAR_WIDTH, CHROME_HEIGHT),
        TabBarPosition::Right => Point::new(0.0, CHROME_HEIGHT),
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
        assert_eq!(state.tab_bar_position, TabBarPosition::Top);
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

    #[test]
    fn content_offset_top() {
        let p = chrome_content_offset(TabBarPosition::Top);
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, TAB_BAR_HEIGHT + CHROME_HEIGHT);
    }

    #[test]
    fn content_offset_left() {
        let p = chrome_content_offset(TabBarPosition::Left);
        assert_eq!(p.x, TAB_SIDEBAR_WIDTH);
        assert_eq!(p.y, CHROME_HEIGHT);
    }

    #[test]
    fn content_offset_right() {
        let p = chrome_content_offset(TabBarPosition::Right);
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, CHROME_HEIGHT);
    }

    #[test]
    fn truncate_title_short() {
        assert_eq!(truncate_title("Hello", 20), "Hello");
    }

    #[test]
    fn truncate_title_long() {
        let long = "This is a very long tab title that should be truncated";
        let result = truncate_title(long, 20);
        assert!(result.chars().count() <= 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn tab_bar_position_default() {
        assert_eq!(TabBarPosition::default(), TabBarPosition::Top);
    }
}
