//! A `replace` whose field's `KIND` differs from the included list's entry is
//! refused (decision AB.8): the library's forms render `Stamp` as text, and an
//! `Int` field would almost certainly be a mistake. The message names both
//! lists and both kinds.
use formoxus::attrs::FieldType;
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::widgets::{AnyWidget, NumberInput};
use formoxus_fixture_lib::Stamp;

#[derive(Clone, Debug, Default, PartialEq)]
struct NumberedStamp;

impl Field for NumberedStamp {
    type Value = Stamp;
    type Context = ();
    const KIND: FieldType = FieldType::Int;

    fn parse(&self, raw: &str) -> Result<Stamp, ValidationMessage> {
        Ok(Stamp(raw.to_string()))
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        NumberInput.into()
    }
}

formoxus::types! {
    struct AppTypes;
    include formoxus_fixture_lib::FixtureTypes;
    replace formoxus_fixture_lib::Stamp => field NumberedStamp,
}

fn main() {}
