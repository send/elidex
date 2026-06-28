//! `CharacterData` interface methods, `Attr` node handlers, `DocumentType` handlers,
//! and additional `Document` property handlers.

mod attr;
mod char_data_handlers;
mod doctype;
mod document_props;
pub mod split_text;

pub use attr::{
    CreateAttribute, GetAttrName, GetAttrSpecified, GetAttrValue, GetAttributeNode,
    GetOwnerElement, RemoveAttributeNode, SetAttrValue, SetAttributeNode,
};
// Re-exported for cross-module callers that produce characterData records
// (the `node_methods` textContent/nodeValue setters here; B1.3-ii's Range
// sites next). `get_char_data` is `pub(crate)` inside the private
// `char_data_handlers` module, so siblings cannot name it without this.
pub(crate) use char_data_handlers::get_char_data;
pub use char_data_handlers::{utf16_len, utf16_to_byte_offset};
pub use char_data_handlers::{
    AppendData, DeleteData, GetData, GetLength, InsertData, ReplaceData, SetData, SplitText,
    SubstringData,
};
pub use doctype::{GetDoctype, GetDoctypeName, GetDoctypePublicId, GetDoctypeSystemId};
pub use document_props::{
    first_body_or_frameset_child, CreateComment, CreateDocumentFragment, GetBody, GetCharacterSet,
    GetCompatMode, GetDocumentBaseURI, GetDocumentElement, GetDocumentUrl, GetHead, GetNodeBaseURI,
    GetReadyState, GetTitle, SetTitle,
};

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;
