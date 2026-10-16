use std::fmt::Debug;

use facet::{Facet, Partial};

use crate::{ValidationMessage, field_attrs::FieldAttrs};

pub trait Field: Clone + Debug + PartialEq + 'static {
    type Value: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static;
    type Context;

    fn parse(&self, raw: &str) -> Result<Self::Value, ValidationMessage> {
        parse_scalar(raw)
    }

    fn check(&self, value: &Self::Value) -> Vec<ValidationMessage> {
        Vec::new()
    }

    fn widget_attrs(&self) -> FieldAttrs { FieldAttrs::default() }

    fn default_widget(&self, optional: bool) -> Box<dyn Widget>;
    
    async fn check_on_server(&self, value: &Self::Value, ctx: &Self::Context) -> Vec<ValidationMessage>;
}

// TODO: move
pub trait Widget {}

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