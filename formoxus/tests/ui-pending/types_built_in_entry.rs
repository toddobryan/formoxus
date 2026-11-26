//! A list cannot give a built-in type a field of its own: `String`, the
//! numbers and `bool` read the same in every form (decision AF).
use formoxus::fields::StringField;

formoxus::types! {
    struct AppTypes;
    String => field StringField,
}

fn main() {}
