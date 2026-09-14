//! Integration tests for the reflection path.
//!
//! Most reflect tests live inside the crate (`src/reflect/tests/`) because they
//! reach crate-private items. This target exists for the ones that CAN'T:
//! anything exercising formoxus's own macros has to be written the way a
//! consumer writes it, from a crate where the path `formoxus::` resolves — which
//! rules out formoxus itself.
//!
//! Submodules live in `tests/reflect/` and need `#[path]`, because cargo only
//! auto-discovers `tests/*.rs` and would otherwise look for `tests/<name>.rs`.

use facet::Facet;
use formoxus::form2;
use formoxus::reflect::empty_form;
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Article {
    headline: String,
    words: u32,
}

/// The title of a form as `form2!` finally stored it.
///
/// Goes through `empty_form` because `FormSpec`'s fields are private — and
/// that's the right route anyway: it's the one a page takes.
fn title_of(spec: formoxus::reflect::form::FormSpec<Article>) -> Option<String> {
    empty_form::<Article>(spec).title()
}

// ── `title:` takes an expression, not a literal ──────────────────────────
//
// Each case below is a *different coercion* into `with_title(&str)`, which is
// the thing that could silently stop working if the emitted call changed. They
// are separate tests rather than one, because a coercion failure is a compile
// error: collapsing them would mean one bad case takes the other eight with it
// and the failure names the wrong thing.

const HEADLINE: &str = "From a const";

static STATIC_HEADLINE: &str = "From a static";

fn owned_title() -> String {
    "From a String fn".to_string()
}

fn borrowed_title() -> &'static str {
    "From a &str fn"
}

#[gtest]
fn a_literal_title_works() {
    expect_that!(
        title_of(form2! { Article { title: "A literal" } }),
        some(eq("A literal"))
    );
}

#[gtest]
fn a_const_title_works() {
    expect_that!(
        title_of(form2! { Article { title: HEADLINE } }),
        some(eq("From a const"))
    );
}

#[gtest]
fn a_static_title_works() {
    expect_that!(
        title_of(form2! { Article { title: STATIC_HEADLINE } }),
        some(eq("From a static"))
    );
}

#[gtest]
fn a_fn_returning_string_works() {
    // The `&String -> &str` coercion, and the one whose temporary has the
    // shortest life: it must survive only until `with_title` copies it.
    expect_that!(
        title_of(form2! { Article { title: owned_title() } }),
        some(eq("From a String fn"))
    );
}

#[gtest]
fn a_fn_returning_str_works() {
    expect_that!(
        title_of(form2! { Article { title: borrowed_title() } }),
        some(eq("From a &str fn"))
    );
}

#[gtest]
fn a_format_call_works() {
    let count = 3;
    expect_that!(
        title_of(form2! { Article { title: format!("{count} drafts") } }),
        some(eq("3 drafts"))
    );
}

#[gtest]
fn a_local_binding_works() {
    // The property that motivated an `Expr`: the macro expands where it is
    // written, so the title can come from a value that does not exist until
    // the form is built — a fetched resource, a route parameter, a signal.
    let from_the_page = String::from("Editing “Trees”");
    expect_that!(
        title_of(form2! { Article { title: from_the_page } }),
        some(eq("Editing “Trees”"))
    );
}

#[gtest]
fn a_conditional_expression_works() {
    let creating = false;
    expect_that!(
        title_of(form2! { Article { title: if creating { "New article" } else { "Edit article" } } }),
        some(eq("Edit article"))
    );
}

#[gtest]
fn a_spec_may_have_no_title() {
    expect_that!(title_of(form2! { Article {} }), none());
}

// ── `validator:` ─────────────────────────────────────────────────────────

fn headline_is_not_shouted(a: &Article) -> Vec<formoxus::error::FormError> {
    if a.headline.chars().all(|c| !c.is_lowercase()) {
        vec![formoxus::error::FormError("headline is all caps".to_string())]
    } else {
        Vec::new()
    }
}

#[gtest]
fn a_validator_fn_item_coerces_to_the_fn_pointer() {
    // Compile-only: `FormSpec::validator` is private and `FormState::validate`
    // does not yet call it, so there is nothing to assert about behaviour. What
    // this pins is that a plain fn item still reaches `with_validator(fn(&T) ->
    // Vec<FormError>)` — the coercion an expanded call depends on.
    expect_that!(
        title_of(form2! {
            Article {
                title: "With a validator",
                validator: headline_is_not_shouted,
            }
        }),
        some(eq("With a validator"))
    );
}

#[gtest]
#[ignore = "NOT WIRED UP: FormState::validate never calls spec.validator — see form.rs::validate"]
fn a_form_validator_rejects_a_model_whose_fields_are_each_valid() {
    // The point of a *form* validator: every field passes on its own, so no
    // member reports an error, and the only thing that can reject this model is
    // a check over the whole of it.
    //
    // Un-ignore this when `validate` runs the validator. It should then go green
    // with no change to the test — if it needs changing, the wiring is wrong.
    let shouted = Article { headline: "TREES ARE GOOD".to_string(), words: 400 };
    let mut state = formoxus::reflect::form_for(
        &shouted,
        form2! { Article { validator: headline_is_not_shouted } },
    );

    expect_that!(state.validate(), none());
    expect_that!(state.has_errors(), eq(true));
}
