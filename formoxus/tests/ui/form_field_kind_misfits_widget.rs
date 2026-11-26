//! `field:` and `widget:` together, where the widget cannot render the
//! field's `KIND`: a text field shown as a checkbox. With `field:` given, the
//! check is against `<F as Field>::KIND`, not the model type's own kind,
//! since a dev's field may read a struct from text. The error should point at
//! the widget, as the existing "`checkbox` cannot render" checks do.
#![allow(unused_imports)] use facet::Facet;
use formoxus::attrs::FieldType;
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::widgets::{AnyWidget, TextInput};
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Span {
    start: u32,
    end: u32,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Highlight {
    span: Span,
}

/// Reads a `Span` from text like `3-7`.
#[derive(Clone, Debug, PartialEq)]
struct SpanField;

impl Field for SpanField {
    type Value = Span;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn parse(&self, _raw: &str) -> Result<Span, ValidationMessage> {
        Ok(Span { start: 0, end: 0 })
    }

    fn format(&self, span: &Span) -> String {
        format!("{}-{}", span.start, span.end)
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

fn main() {
    let _ = empty_form::<Highlight>(form! {
        Highlight {
            span => { field: SpanField, widget: checkbox },
        }
    });
}
