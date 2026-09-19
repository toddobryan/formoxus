//! `""` IS absence, at both boundaries.

use crate::*;
use facet::Facet;
use std::{collections::HashMap, fmt::Debug};
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Optional {
    body: Option<String>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Required {
    body: String,
}

/// Push a form's own leaves back through the widget boundary and revalidate —
/// the path a real submit takes, as opposed to validating the populated form.
fn through_the_dom<T>(form: &FormState<T>, mut reloaded: FormState<T>) -> Option<T>
where
    T: Clone + Debug + PartialEq + Facet<'static>,
{
    let collected: HashMap<String, String> = form.leaves().into_iter().collect();
    reloaded.apply(&collected);
    reloaded.validate()
}

#[gtest]
fn populating_some_empty_collapses_to_none() {
    // Regression: populating used to keep `Valid("")` here, so this returned
    // `Some("")` while the DOM path returned `None` for the same model.
    let mut form = form_for(&Optional {
        body: Some(String::new()),
    }, FormSpec::default());
    expect_that!(form.validate(), some(eq(&Optional { body: None })));
}

#[gtest]
fn some_empty_agrees_on_both_paths() {
    let value = Optional {
        body: Some(String::new()),
    };
    let form = form_for(&value, FormSpec::default());
    let populated = form_for(&value, FormSpec::default()).validate();
    let dom = through_the_dom(&form, empty_form::<Optional>(FormSpec::default()));

    expect_that!(populated, eq(&dom));
    expect_that!(populated, some(eq(&Optional { body: None })));
}

#[gtest]
fn a_required_empty_string_fails_on_both_paths() {
    // The genuine cost of the rule, made explicit: a model holding `""` in a
    // required field can't round-trip. That's HTML5's constraint, not ours —
    // and it now fails the same way whichever path it takes, instead of
    // passing when populated and erroring through the DOM.
    let value = Required {
        body: String::new(),
    };
    let form = form_for(&value, FormSpec::default());
    let populated = form_for(&value, FormSpec::default()).validate();
    let dom = through_the_dom(&form, empty_form::<Required>(FormSpec::default()));

    expect_that!(populated, none());
    expect_that!(dom, none());
}

#[gtest]
fn none_and_some_empty_are_indistinguishable() {
    // Both directions of the same coin: populating with `None` and `Some("")`
    // produce the same form, so nothing downstream can tell them apart.
    let from_none = form_for(&Optional { body: None }, FormSpec::default());
    let from_empty = form_for(&Optional {
        body: Some(String::new()),
    }, FormSpec::default());
    expect_that!(from_none.leaves(), eq(&from_empty.leaves()));
}

#[gtest]
fn non_empty_strings_are_untouched() {
    // Guard on the collapse: it must catch `""` and nothing else. A string of
    // spaces is a real value — trimming is a validator's job, not populating's.
    let value = Optional {
        body: Some("   ".to_string()),
    };
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));

    let value = Required {
        body: "hello".to_string(),
    };
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}

#[gtest]
fn zero_valued_scalars_are_not_empty() {
    // The collapse keys on the *display* string, so it must not swallow
    // falsy-looking numbers and bools — `0`/`0.0`/`false` all render
    // non-empty and stay `Valid`.
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Falsy {
        count: i32,
        ratio: f64,
        flag: bool,
    }

    let value = Falsy {
        count: 0,
        ratio: 0.0,
        flag: false,
    };
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}
