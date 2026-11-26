//! Every list already holds the built-in types (decision AF), so including
//! `BuiltIns` would add nothing; it is refused rather than quietly allowed,
//! and `BuiltIns` writes no replay macro to include.
formoxus::types! {
    struct AppTypes;
    include formoxus::BuiltIns;
}

fn main() {}
