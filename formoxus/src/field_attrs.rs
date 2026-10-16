use formoxus_spec::{AttrKey, AttrValue};
use indexmap::IndexMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldAttrs(IndexMap<AttrKey, AttrValue>);