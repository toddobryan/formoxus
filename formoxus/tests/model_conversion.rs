//! Minimal repro for `#[form(model = X)]` where `X`'s fields don't line up
//! 1:1 by name/order with the form's own fields — the simplest possible case
//! that forces the "Model != Self" question at the *form* level (no embedded
//! `FieldSet` at all, unlike `generic_form.rs`'s `Wrapper<T>`). `XForm` and
//! `X` have the same two fields, reordered and renamed, so the derive's
//! existing by-name shorthand construction (`Self { field, ... }`) can't work
//! unmodified — the goal is to see exactly what breaks and confirm the
//! `From<XForm> for X` / `From<X> for XForm` design holds up.

use formoxus::prelude::*;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct X {
    pub second: String,
    pub first: String,
}

#[derive(Form, Debug)]
#[form(model = X, button(type = "submit", name = submit))]
#[allow(dead_code)]
pub struct XForm {
    pub first: String,
    pub second: String,
}

impl From<XForm> for X {
    fn from(form: XForm) -> Self {
        X {
            second: form.second,
            first: form.first,
        }
    }
}

impl From<X> for XForm {
    fn from(x: X) -> Self {
        XForm {
            first: x.first,
            second: x.second,
        }
    }
}

#[test]
fn a_form_with_a_renamed_reordered_model_round_trips() {
    fn assert_form<F: Form>() {}
    assert_form::<XForm>();
}
