//! HTML tag → AccessKit Role mapping.
//!
//! Maps HTML element tag names to their corresponding ARIA roles
//! per the HTML-AAM specification (simplified).

use accesskit::Role;
use elidex_form::FormControlKind;

/// Map an HTML tag name to an AccessKit Role.
///
/// Returns `Role::GenericContainer` for unmapped tags.
#[must_use]
pub(crate) fn tag_to_role(tag: &str) -> Role {
    match tag {
        "a" => Role::Link,
        "article" => Role::Article,
        "aside" => Role::Complementary,
        "button" => Role::Button,
        "canvas" => Role::Canvas,
        // footer, form, header, section: context-dependent — handled in tree.rs.
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Role::Heading,
        "hr" => Role::Splitter,
        "img" => Role::Image,
        "input" => Role::TextInput,
        "label" => Role::Label,
        "li" => Role::ListItem,
        "main" => Role::Main,
        "nav" => Role::Navigation,
        "ol" | "ul" => Role::List,
        "option" => Role::ListBoxOption,
        "p" => Role::Paragraph,
        "code" => Role::Code,
        "progress" => Role::ProgressIndicator,
        "select" => Role::ListBox,
        "table" => Role::Table,
        "tbody" | "thead" | "tfoot" => Role::RowGroup,
        "td" => Role::Cell,
        "textarea" => Role::MultilineTextInput,
        "th" => Role::ColumnHeader,
        "tr" => Role::Row,
        _ => Role::GenericContainer,
    }
}

/// Map an ARIA role string to an AccessKit Role.
///
/// Handles the standard ARIA role values defined in WAI-ARIA 1.2.
/// Returns `None` for unrecognized role strings.
#[must_use]
pub(crate) fn aria_role_from_str(role: &str) -> Option<Role> {
    match role.trim().to_ascii_lowercase().as_str() {
        "alert" => Some(Role::Alert),
        "alertdialog" => Some(Role::AlertDialog),
        "application" => Some(Role::Application),
        "article" => Some(Role::Article),
        "banner" => Some(Role::Banner),
        "button" => Some(Role::Button),
        "cell" | "gridcell" => Some(Role::Cell),
        "checkbox" => Some(Role::CheckBox),
        "columnheader" => Some(Role::ColumnHeader),
        "combobox" => Some(Role::ComboBox),
        "complementary" => Some(Role::Complementary),
        "contentinfo" => Some(Role::ContentInfo),
        "dialog" => Some(Role::Dialog),
        "document" => Some(Role::Document),
        "feed" => Some(Role::Feed),
        "form" => Some(Role::Form),
        "grid" => Some(Role::Grid),
        "group" => Some(Role::Group),
        "heading" => Some(Role::Heading),
        "img" => Some(Role::Image),
        "link" => Some(Role::Link),
        "list" => Some(Role::List),
        "listbox" => Some(Role::ListBox),
        "listitem" => Some(Role::ListItem),
        "log" => Some(Role::Log),
        "main" => Some(Role::Main),
        "marquee" => Some(Role::Marquee),
        "menu" => Some(Role::Menu),
        "menubar" => Some(Role::MenuBar),
        "menuitem" => Some(Role::MenuItem),
        "menuitemcheckbox" => Some(Role::MenuItemCheckBox),
        "menuitemradio" => Some(Role::MenuItemRadio),
        "navigation" => Some(Role::Navigation),
        // GenericContainer is AccessKit's equivalent of ARIA none/presentation.
        "none" | "presentation" => Some(Role::GenericContainer),
        "note" => Some(Role::Note),
        "option" => Some(Role::ListBoxOption),
        "progressbar" => Some(Role::ProgressIndicator),
        "radio" => Some(Role::RadioButton),
        "radiogroup" => Some(Role::RadioGroup),
        "region" => Some(Role::Region),
        "row" => Some(Role::Row),
        "rowgroup" => Some(Role::RowGroup),
        "rowheader" => Some(Role::RowHeader),
        "search" => Some(Role::Search),
        "separator" => Some(Role::Splitter),
        "slider" => Some(Role::Slider),
        "spinbutton" => Some(Role::SpinButton),
        "status" => Some(Role::Status),
        "switch" => Some(Role::Switch),
        "tab" => Some(Role::Tab),
        "table" => Some(Role::Table),
        "tablist" => Some(Role::TabList),
        "tabpanel" => Some(Role::TabPanel),
        "term" => Some(Role::Term),
        "textbox" => Some(Role::TextInput),
        "timer" => Some(Role::Timer),
        "toolbar" => Some(Role::Toolbar),
        "tooltip" => Some(Role::Tooltip),
        "tree" => Some(Role::Tree),
        "treegrid" => Some(Role::TreeGrid),
        "treeitem" => Some(Role::TreeItem),
        _ => None,
    }
}

/// Get the heading level (1–6) from a heading tag name.
///
/// Returns `None` for non-heading tags.
#[must_use]
pub(crate) fn heading_level(tag: &str) -> Option<usize> {
    match tag {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

/// Map a `FormControlKind` to an AccessKit Role per HTML-AAM.
#[must_use]
pub(crate) fn form_control_role(kind: FormControlKind) -> Role {
    match kind {
        FormControlKind::TextArea => Role::MultilineTextInput,
        FormControlKind::Number => Role::SpinButton,
        FormControlKind::Range => Role::Slider,
        FormControlKind::Checkbox => Role::CheckBox,
        FormControlKind::Radio => Role::RadioButton,
        FormControlKind::Select => Role::ComboBox,
        FormControlKind::SubmitButton
        | FormControlKind::ResetButton
        | FormControlKind::Button => Role::Button,
        FormControlKind::Hidden => Role::GenericContainer,
        FormControlKind::Output => Role::Status,
        FormControlKind::Meter => Role::Meter,
        FormControlKind::Progress => Role::ProgressIndicator,
        // All text-like inputs map to TextInput.
        FormControlKind::TextInput
        | FormControlKind::Password
        | FormControlKind::Email
        | FormControlKind::Url
        | FormControlKind::Tel
        | FormControlKind::Search
        | FormControlKind::Color
        | FormControlKind::Date
        | FormControlKind::DatetimeLocal
        | FormControlKind::File => Role::TextInput,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_to_role_mapping() {
        let cases = [
            // Common tags.
            ("a", Role::Link),
            ("article", Role::Article),
            ("aside", Role::Complementary),
            ("button", Role::Button),
            ("canvas", Role::Canvas),
            ("h1", Role::Heading),
            ("h3", Role::Heading),
            ("h6", Role::Heading),
            ("hr", Role::Splitter),
            ("img", Role::Image),
            ("input", Role::TextInput),
            ("label", Role::Label),
            ("li", Role::ListItem),
            ("main", Role::Main),
            ("nav", Role::Navigation),
            ("ol", Role::List),
            ("ul", Role::List),
            ("option", Role::ListBoxOption),
            ("p", Role::Paragraph),
            ("code", Role::Code),
            ("progress", Role::ProgressIndicator),
            ("select", Role::ListBox),
            ("table", Role::Table),
            ("tbody", Role::RowGroup),
            ("td", Role::Cell),
            ("textarea", Role::MultilineTextInput),
            ("th", Role::ColumnHeader),
            ("tr", Role::Row),
            // Unknown tags → GenericContainer.
            ("div", Role::GenericContainer),
            ("span", Role::GenericContainer),
            ("custom-element", Role::GenericContainer),
            ("pre", Role::GenericContainer),
            // Context-dependent tags (handled in tree.rs) → GenericContainer.
            ("header", Role::GenericContainer),
            ("footer", Role::GenericContainer),
            ("section", Role::GenericContainer),
            ("form", Role::GenericContainer),
        ];

        for (tag, expected) in cases {
            assert_eq!(tag_to_role(tag), expected, "tag: {tag}");
        }
    }

    #[test]
    fn heading_level_mapping() {
        let cases = [
            ("h1", Some(1)),
            ("h2", Some(2)),
            ("h3", Some(3)),
            ("h4", Some(4)),
            ("h5", Some(5)),
            ("h6", Some(6)),
            ("div", None),
            ("p", None),
        ];

        for (tag, expected) in cases {
            assert_eq!(heading_level(tag), expected, "tag: {tag}");
        }
    }

    #[test]
    fn aria_role_mapping() {
        let cases = [
            ("alert", Some(Role::Alert)),
            ("button", Some(Role::Button)),
            ("dialog", Some(Role::Dialog)),
            ("link", Some(Role::Link)),
            ("navigation", Some(Role::Navigation)),
            ("none", Some(Role::GenericContainer)),
            ("presentation", Some(Role::GenericContainer)),
            ("textbox", Some(Role::TextInput)),
            // Case-insensitive.
            ("BUTTON", Some(Role::Button)),
            (" navigation ", Some(Role::Navigation)),
            // Unknown roles.
            ("invalid", None),
            ("", None),
        ];

        for (role_str, expected) in cases {
            assert_eq!(aria_role_from_str(role_str), expected, "role: {role_str:?}");
        }
    }

    #[test]
    fn form_control_role_mapping() {
        let cases = [
            (FormControlKind::TextInput, Role::TextInput),
            (FormControlKind::Password, Role::TextInput),
            (FormControlKind::TextArea, Role::MultilineTextInput),
            (FormControlKind::Checkbox, Role::CheckBox),
            (FormControlKind::Radio, Role::RadioButton),
            (FormControlKind::Select, Role::ComboBox),
            (FormControlKind::SubmitButton, Role::Button),
            (FormControlKind::ResetButton, Role::Button),
            (FormControlKind::Button, Role::Button),
            (FormControlKind::Number, Role::SpinButton),
            (FormControlKind::Range, Role::Slider),
            (FormControlKind::Hidden, Role::GenericContainer),
            (FormControlKind::Output, Role::Status),
            (FormControlKind::Meter, Role::Meter),
            (FormControlKind::Progress, Role::ProgressIndicator),
        ];
        for (kind, expected) in cases {
            assert_eq!(form_control_role(kind), expected, "kind: {kind:?}");
        }
    }
}
