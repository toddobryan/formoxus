//! A `widget:` that cannot render a listed type's field is a compile error:
//! `form!` sees the list, so it knows `TimeLimit` reads text. Before AB this
//! compiled and the build panicked (decision O.2, the suite's
//! `a_widget_that_cannot_render_the_registered_fields_kind_panics_at_build`).
use facet::Facet;
use formoxus::attrs::FieldType;
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::prelude::*;
use formoxus::widgets::{AnyWidget, TextInput};

#[derive(Facet, Clone, Debug, PartialEq)]
struct TimeLimit(u32);

#[derive(Clone, Debug, Default, PartialEq)]
struct TimeLimitField;

impl Field for TimeLimitField {
    type Value = TimeLimit;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn parse(&self, raw: &str) -> Result<TimeLimit, ValidationMessage> {
        raw.parse().map(TimeLimit).map_err(|_| "not a time".into())
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

formoxus::types! {
    struct AppTypes;
    TimeLimit => field TimeLimitField,
}
use AppTypes as FormoxusTypes;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    limit: TimeLimit,
}

fn main() {
    let _ = empty_form(form! { Quiz { limit => { widget: checkbox } } });
}
