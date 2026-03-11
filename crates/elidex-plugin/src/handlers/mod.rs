//! Built-in plugin handler implementations and factory functions.

mod css;
mod html;
mod layout;

pub use css::create_css_property_registry;
pub use html::create_html_element_registry;
pub use layout::create_layout_registry;
