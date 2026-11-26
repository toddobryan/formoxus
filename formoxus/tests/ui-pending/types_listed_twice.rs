//! A type listed twice in one `types!` is refused, naming the type
//! (`CUSTOM_FIELDS_PLAN.md` 5.9.2.5). It was a run-time panic when
//! `TypeRegistry::field_for` was called twice; a list makes it a compile error.
use facet::Facet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Note(String);

formoxus::types! {
    struct AppTypes;
    Note => newtype String,
    Note => newtype String,
}

fn main() {}
