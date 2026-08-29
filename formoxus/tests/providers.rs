//! End-to-end test for the Provider mechanism — `#[form(component = ...,
//! provided)]` and how a `Provider<C>` flows through the generated
//! `...Providers` struct, including through `#[form(field_set)]` nesting
//! (both a single nested field set and a repeating group). No Dioxus runtime
//! is exercised (no test calls `render`), but compiling this file already
//! proves every generated `render` body — including the nested `Providers`
//! type projections — type-checks, the same reasoning `field_set_list.rs`
//! and `field_set_embedding.rs` rely on.

use formoxus::prelude::*;

/// A trivial provided widget — never actually rendered by this test, just
/// needs to exist so `#[form(component = ChoicePicker, provided)]` has
/// something to point at.
pub struct ChoicePicker;

impl ProvidedWidget<String> for ChoicePicker {
    type Choices = Vec<String>;

    fn render(
        _field: dioxus::stores::Store<FormField<String>>,
        _props: FieldProps,
        _provide: Provider<Vec<String>>,
    ) -> dioxus::core::Element {
        unimplemented!("never called — this test only proves the codegen compiles")
    }
}

#[derive(FieldSet, Debug, Clone, Default)]
#[allow(dead_code)]
pub struct WithProvidedField {
    #[form(component = ChoicePicker, provided)]
    choice: String,
}

#[derive(Form, Debug)]
#[form(button(type = "submit", name = submit))]
#[allow(dead_code)]
pub struct OuterForm {
    #[form(component = ChoicePicker, provided)]
    top_level_choice: String,
    #[form(field_set)]
    nested_single: WithProvidedField,
    #[form(field_set)]
    nested_list: Vec<WithProvidedField>,
}

/// The generated `OuterFormProviders` has exactly the shape expected: one
/// `Provider<Vec<String>>` slot for the field marked directly `provided`, and
/// one nested `WithProvidedFieldProviders` slot per embedded field set — the
/// *same* nested type for both the singular and repeating-group field, since
/// a repeating group shares one set of providers across every row rather than
/// needing one per row.
#[test]
fn providers_struct_has_the_expected_shape() {
    let _providers = OuterFormProviders {
        top_level_choice: provider(|| async { vec!["a".to_string()] }),
        nested_single: WithProvidedFieldProviders {
            choice: provider(|| async { vec!["b".to_string()] }),
        },
        nested_list: WithProvidedFieldProviders {
            choice: provider(|| async { vec!["c".to_string()] }),
        },
    };
}

/// A form with no provided fields and no embedded field sets gets `Providers
/// = ()` — the common case pays nothing for this mechanism existing.
#[derive(Form, Debug)]
#[form(button(type = "submit", name = submit))]
#[allow(dead_code)]
pub struct PlainForm {
    name: String,
}

#[test]
fn a_form_with_nothing_provided_has_unit_providers() {
    fn assert_unit_providers<F: FormState<Providers = ()>>() {}
    assert_unit_providers::<PlainFormState>();
}
