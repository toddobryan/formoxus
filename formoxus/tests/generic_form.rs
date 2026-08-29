//! Minimal repro for a `Form` generic over an embedded `FieldSet` type
//! (`Wrapper<T: FieldSet>`), isolated from the real app's `QuestionForm<T>`
//! (see `web/src/views/questions/forms.rs`, currently commented out while
//! this gets sorted here first). The goal is to pin down exactly which
//! `FieldSet`/`FieldSetState` trait bounds are missing by watching what the
//! compiler actually demands, one error at a time, rather than guessing.

use formoxus::prelude::*;

#[derive(FieldSet, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct Inner {
    name: String,
}

#[derive(Form, Clone, Debug)]
#[form(button(type = "submit", name = submit))]
#[allow(dead_code)]
pub struct Wrapper<T>
where
    T: FieldSet,
    T::State: FieldSetState<Model = T>,
{
    label: String,
    #[form(field_set)]
    data: T,
}

#[test]
fn a_generic_form_compiles_over_a_concrete_field_set() {
    fn assert_form<F: Form>() {}
    assert_form::<Wrapper<Inner>>();
}
