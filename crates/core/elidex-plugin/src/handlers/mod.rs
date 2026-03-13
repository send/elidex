//! Built-in plugin handler implementations and factory functions.

mod html;
mod layout;

pub use html::create_html_element_registry;
pub use layout::create_layout_registry;
