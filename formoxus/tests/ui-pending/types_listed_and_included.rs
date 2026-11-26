//! Listing a type an included list already has is refused, pointing at
//! `replace` (decision AB.4.4), rather than leaving rustc's bare E0119.
use formoxus_fixture_lib::Badge;

formoxus::types! {
    struct AppTypes;
    include formoxus_fixture_lib::FixtureTypes;
    Badge => newtype String,
}

fn main() {}
