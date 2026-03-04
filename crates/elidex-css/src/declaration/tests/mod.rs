use super::*;
use elidex_plugin::{CssColor, LengthUnit};

mod grid_table;
mod shorthand;
mod values;

fn parse_decls(css: &str) -> Vec<Declaration> {
    parse_declaration_block(css)
}

fn parse_single(property: &str, value: &str) -> Vec<Declaration> {
    parse_decls(&format!("{property}: {value}"))
}
