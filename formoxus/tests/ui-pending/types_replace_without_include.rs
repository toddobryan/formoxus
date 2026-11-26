//! A `replace` needs its type to come from an included list (decision AF.3):
//! with nothing included there is nothing to replace, and the line would
//! otherwise read as an override that never happens.
use formoxus_fixture_lib::Stamp;

formoxus::types! {
    struct AppTypes;
    replace Stamp => newtype String,
}

fn main() {}
