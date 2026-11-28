use std::fmt::Debug;

use facet::{Facet, Partial, Peek};
use formoxus_spec::FieldType;

use crate::{
    error::ValidationMessage, field_attrs::FieldAttrs, fields::Choice, widgets::AnyWidget,
};

/// The token [`Field::choices`] takes, which only formoxus can build: outside
/// the crate that method can be neither called nor overridden, so only the
/// blanket impl for a [`ChoiceField`](crate::fields::ChoiceField) (and
/// [`Newtype`](crate::fields::Newtype), passing it through) ever says a field
/// has choices (`CUSTOM_FIELDS_PLAN.md` decision AU).
///
/// `pub` in a private module, the sealed-method pattern: nameable inside the
/// crate, unreachable outside it.
mod sealed {
    #[derive(Clone, Copy, Debug)]
    pub struct Sealed(());

    impl Sealed {
        pub(crate) const fn new() -> Self {
            Self(())
        }
    }
}
pub(crate) use sealed::Sealed;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue<T: Clone + Debug + PartialEq> {
    Empty,
    Valid(T),
    Invalid {
        raw: String,
        error: ValidationMessage,
    },
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a formoxus `Field`",
    label = "`field:` takes a type that implements `formoxus::field::Field`",
    note = "for a value chosen from a list, implement `formoxus::fields::ChoiceField` instead: \
            it makes the type a `Field`"
)]
pub trait Field: Clone + Debug + PartialEq + 'static {
    type Value: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static;
    type Context;

    /// The kind of text this field reads and writes: text, an integer, a
    /// float or a bool.
    ///
    /// This describes the field's text, not its `Value`. A field that reads
    /// `3-7` into a `Span { start, end }` is
    /// [`Text`](crate::attrs::FieldType::Text), even though `Span` is a
    /// struct, because what the user types is text. Formoxus cannot work this
    /// out from `Value`'s shape, so the field says it.
    ///
    /// Two things read it:
    ///
    /// 1. `form!` checks at compile time that the widget named next to a
    ///    `field:` can render it: the widget's
    ///    [`ACCEPTS`](crate::widgets::LeafWidget::ACCEPTS) must contain this
    ///    kind. That check runs in a `const` item, and a `const` item can read
    ///    an associated const but cannot call a method, which is why this is a
    ///    const and not a method.
    /// 2. The bounds the field emits (`min`, `max`) are written as integers for
    ///    an [`Int`](crate::attrs::FieldType::Int) field and as floats for
    ///    any other.
    ///
    /// [`File`](crate::attrs::FieldType::File) is never a field's kind: an
    /// upload is its own node, not a `Field`.
    ///
    /// There is no default, so a dev writing a `Field` chooses its kind
    /// instead of inheriting `Text` without noticing.
    const KIND: FieldType;

    /// The text `value` is written as: what fills the input when a form is
    /// filled from an existing value. The reverse of [`parse`](Self::parse).
    ///
    /// It must round-trip with `parse`: `parse(&format(v))` gives back `v`.
    /// Editing a saved value refills the input with this text and reads it
    /// back with `parse` on submit, so text the field would refuse makes the
    /// value impossible to save unchanged. A field that overrides `parse` to
    /// read its own syntax (`3-7` for a span) overrides this too.
    ///
    /// A value written as `""` counts as no value. Filling a form with it
    /// gives [`FieldValue::Empty`], the same as an empty input, because in
    /// formoxus `""` is absence.
    ///
    /// The default writes the value through facet's display vtable rather
    /// than a `Display` bound, the mirror of the default `parse` going the
    /// other way. Not `Debug`: it quotes strings, and `parse` would read the
    /// quotes back as part of the value.
    fn format(&self, value: &Self::Value) -> String {
        Peek::new(value).to_string()
    }

    fn parse(&self, raw: &str) -> Result<Self::Value, ValidationMessage> {
        parse_scalar(raw)
    }

    fn check(&self, _value: &Self::Value) -> Vec<ValidationMessage> {
        Vec::new()
    }

    fn widget_attrs(&self) -> FieldAttrs {
        FieldAttrs::default()
    }

    /// The widget this field renders as when `form!` names none.
    ///
    /// `optional` is whether the model's field is an `Option`, which `Value`
    /// cannot say: `Option` peeling wraps a field rather than parameterizing
    /// it, so `bool` and `Option<bool>` both arrive as a `BoolField`.
    ///
    /// No default body, so a dev writing a `Field` chooses its widget rather
    /// than inheriting a text input without noticing.
    fn default_widget(&self, optional: bool) -> AnyWidget;

    fn check_on_server(
        &self,
        _value: &Self::Value,
        _ctx: &Self::Context,
    ) -> impl Future<Output = Vec<ValidationMessage>> + Send {
        async { Vec::new() }
    }

    /// Whether this field offers a list to choose from: `true` only for a
    /// [`ChoiceField`](crate::fields::ChoiceField), whose blanket impl sets
    /// it, and for a [`Newtype`](crate::fields::Newtype) over one. A const,
    /// not a method, because `form!` asks it at compile time: a `field:` that
    /// brings choices lets a widget that takes choices (`radio_group`) through
    /// whatever the field's `KIND`. Leave it alone in a field of your own; to
    /// offer choices, implement `ChoiceField` instead.
    const HAS_CHOICES: bool = false;

    /// The list, for the widget to render and for a set to show each item's
    /// label. Sealed (decision AU): only formoxus calls it, and only a
    /// `ChoiceField`'s blanket impl and `Newtype` answer anything but `None`.
    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn choices(&self, _: Sealed) -> Option<Vec<Choice>> {
        None
    }

    /// Whether the raw text is trimmed of leading and trailing whitespace
    /// before it is read (#16): where it becomes a value, never in the input,
    /// which keeps what was typed. `password` is whether the field renders as
    /// a password input, where a space is part of the secret, so a field
    /// that trims text should not trim it.
    ///
    /// `false` by default: a `bool`'s or a choice's raw text is written by a
    /// control formoxus renders, and is read exactly as sent.
    fn trims(&self, _password: bool) -> bool {
        false
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

pub(crate) fn populate<F: Field>(
    field: &F,
    peek: Option<Peek<'_, 'static>>,
) -> FieldValue<F::Value> {
    match peek {
        None => FieldValue::Empty,
        // `""` IS absence, and that has to hold at BOTH boundaries. `distribute_values`
        // already collapses an empty input to `Empty`; without the same collapse
        // here, populating kept `Some("")` alive and `leaves() -> apply()` silently
        // stopped being an identity — the very invariant the uncontrolled design
        // rests on. Comparing the field's own `format` is what makes the two agree
        // exactly, since that's the string `raw_value` would have emitted.
        //
        // Of the built-in fields, only `StringField` can reach this:
        // `true`/`0`/`0.0` are never empty. A dev's field can, for any value its
        // `format` writes as `""`, and that value populates as `Empty` too.
        // The cost is that a required `String` holding `""` can't round-trip — but
        // that's HTML5's rule, not ours (an empty required input is `valueMissing`),
        // so no browser form could round-trip it either. Failing the same way on
        // both paths beats depending on which path the value arrived through.
        Some(p) => {
            let value = p
                .get::<F::Value>()
                .expect("a leaf's field holds its path's own type, so this get should be that type")
                .clone();
            if field.format(&value).is_empty() {
                FieldValue::Empty
            } else {
                FieldValue::Valid(value)
            }
        }
    }
}
