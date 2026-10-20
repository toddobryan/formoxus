use std::fmt::Debug;

use facet::{Facet, Partial, Peek};

use crate::{
    ValidationMessage,
    field_attrs::FieldAttrs,
    widgets::{AnyWidget, Choice},
};

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue<T: Clone + Debug + PartialEq> {
    Empty,
    Valid(T),
    Invalid {
        raw: String,
        error: ValidationMessage,
    },
}

pub trait Field: Clone + Debug + PartialEq + 'static {
    type Value: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static;
    type Context;

    fn parse(&self, raw: &str) -> Result<Self::Value, ValidationMessage> {
        parse_scalar(raw)
    }

    fn check(&self, _value: &Self::Value) -> Vec<ValidationMessage> {
        Vec::new()
    }

    fn widget_attrs(&self) -> FieldAttrs {
        FieldAttrs::default()
    }

    #[expect(
        clippy::todo,
        reason = "no `Widget` can be built until Step 5 of `FIELDS_PLAN.md`; \
        rendering still goes through `WidgetType`"
    )]
    fn default_widget(&self, _optional: bool) -> AnyWidget {
        todo!("Step 5: rendering still goes through `WidgetType`")
    }

    fn check_on_server(
        &self,
        _value: &Self::Value,
        _ctx: &Self::Context,
    ) -> impl Future<Output = Vec<ValidationMessage>> + Send {
        async { Vec::new() }
    }

    fn choices(&self) -> Option<Vec<Choice>> {
        None
    }
}

/// Parse a raw input string into `X` using `X`'s own facet parse vtable —
/// the runtime equivalent of the `T: FromStr` bound the macro-based version
/// leaned on.
///
/// **`pub` only so the test suite can reach it from `tests/`, and `doc(hidden)`
/// because that is the whole reason.** It is an implementation detail of how a
/// leaf converts, not a service this crate offers; treat its signature as
/// unstable.
#[doc(hidden)]
pub fn parse_scalar<X>(raw: &str) -> Result<X, ValidationMessage>
where
    X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    let partial = Partial::alloc::<X>().map_err(|e| ValidationMessage(e.to_string()))?;
    let partial = partial
        .parse_from_str(raw)
        .map_err(|_| ValidationMessage(format!("{raw:?} isn't a valid {}", X::SHAPE)))?;
    partial
        .build()
        .map_err(|e| ValidationMessage(e.to_string()))?
        .materialize::<X>()
        .map_err(|e| ValidationMessage(e.to_string()))
}

pub(crate) fn populate<X>(peek: Option<Peek<'_, 'static>>) -> FieldValue<X>
where
    X: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    match peek {
        None => FieldValue::Empty,
        // `""` IS absence, and that has to hold at BOTH boundaries. `distribute_values`
        // already collapses an empty input to `Empty`; without the same collapse
        // here, populating kept `Some("")` alive and `leaves() -> apply()` silently
        // stopped being an identity — the very invariant the uncontrolled design
        // rests on. Comparing the *display* string is what makes the two agree
        // exactly, since that's the string `raw_value` would have emitted.
        //
        // Only `String` can actually reach this: `true`/`0`/`0.0` are never empty.
        // The cost is that a required `String` holding `""` can't round-trip — but
        // that's HTML5's rule, not ours (an empty required input is `valueMissing`),
        // so no browser form could round-trip it either. Failing the same way on
        // both paths beats depending on which path the value arrived through.
        Some(p) if p.to_string().is_empty() => FieldValue::Empty,
        Some(p) => FieldValue::Valid(
            p.get::<X>()
                .expect("scalar_type matched, so this get should be the right type")
                .clone(),
        ),
    }
}
