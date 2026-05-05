//! `CharacterData` interface methods, `Attr` node handlers, `DocumentType` handlers,
//! and additional `Document` property handlers.

mod attr;
mod char_data_handlers;
mod doctype;
mod document_props;

pub use attr::{
    CreateAttribute, GetAttrName, GetAttrSpecified, GetAttrValue, GetAttributeNode,
    GetOwnerElement, RemoveAttributeNode, SetAttrValue, SetAttributeNode,
};
pub(crate) use char_data_handlers::{utf16_len, utf16_to_byte_offset};
pub use char_data_handlers::{
    AppendData, DeleteData, GetData, GetLength, InsertData, ReplaceData, SetData, SplitText,
    SubstringData,
};
pub use doctype::{GetDoctype, GetDoctypeName, GetDoctypePublicId, GetDoctypeSystemId};
pub use document_props::{
    first_body_or_frameset_child, CreateComment, CreateDocumentFragment, GetBody, GetCharacterSet,
    GetCompatMode, GetDocumentElement, GetDocumentUrl, GetHead, GetReadyState, GetTitle, SetTitle,
};

#[cfg(test)]
#[allow(unused_must_use)]
mod tests;
