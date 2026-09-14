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
use formoxus::reflect::{empty_form, form_for, use_form};
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
fn a_form_validator_rejects_a_model_whose_fields_are_each_valid() {
    // The point of a *form* validator: every field passes on its own, so no
    // member reports an error, and the only thing that can reject this model is
    // a check over the whole of it.
    let shouted = Article { headline: "TREES ARE GOOD".to_string(), words: 400 };
    let mut state = formoxus::reflect::form_for(
        &shouted,
        form2! { Article { validator: headline_is_not_shouted } },
    );

    expect_that!(state.validate(), none());
    expect_that!(state.has_errors(), eq(true));
}

// ══ Field specs reaching the built tree ══════════════════════════════════
//
// The gap these close: the macro crate's tests stop at the emitted tokens, and
// `src/reflect/tests/specs.rs` deliberately builds its `FormSpec`s by hand so it
// stays honest about testing the *builder*. Neither would notice `expand()`
// emitting a wrong key string, or dropping the field arm of the chain. These go
// through `form2!` and read the DOM, which is the only unambiguous evidence that
// an override arrived.

use dioxus::prelude::*;

/// Render a component to HTML with a real Dioxus runtime behind it.
///
/// The crate's own `reflect::tests::render_to_html` is `#[cfg(test)]`, so it is
/// invisible from an integration test; this is the same three lines. A runtime is
/// needed because `FormState::render` wants a `ValuesByPath`, which only
/// `use_store` can mint, and that is a hook.
fn render_to_html(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venue {
    street: String,
    city: String,
}

/// Wide enough to reach every addressing shape in one model: a scalar, a nested
/// struct, a list, and a `bool` whose derived control is a checkbox (so a `select`
/// override is visibly different).
#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    name: String,
    secret: String,
    venue: Venue,
    stops: Vec<String>,
    confirmed: bool,
}

fn a_trip() -> Trip {
    Trip {
        name: "Spring tour".to_string(),
        secret: "hunter2".to_string(),
        venue: Venue { street: "123 Main St".to_string(), city: "Springfield".to_string() },
        stops: vec!["a@example.com".to_string(), "b@example.com".to_string()],
        confirmed: true,
    }
}

// ── A label ──────────────────────────────────────────────────────────────

#[component]
fn LabelledName() -> Element {
    use_form(|| empty_form(form2! {
        Trip {
            name => { label: "Trip name" },
        }
    }))
    .render()
}

#[gtest]
fn a_label_from_the_macro_reaches_the_markup() {
    let html = render_to_html(LabelledName);
    expect_that!(html, contains_substring("Trip name"));
    // `default_label` would have made this "Name", so its absence is what shows
    // the override won rather than merely coexisting.
    expect_that!(html, not(contains_substring(">Name<")));
}

// ── A control ────────────────────────────────────────────────────────────

#[component]
fn PasswordSecret() -> Element {
    use_form(|| form_for(
        &a_trip(),
        form2! {
            Trip {
                secret => { control: password },
            }
        },
    ))
    .render()
}

#[gtest]
fn a_control_from_the_macro_changes_the_rendered_input() {
    let html = render_to_html(PasswordSecret);
    expect_that!(html, contains_substring("type=\"password\""));
    // The value round-trips like any other field's — `control: password` changes
    // the masking, not the binding.
    expect_that!(html, contains_substring("hunter2"));
    // A sibling scalar keeps its default, so the override landed on one field.
    expect_that!(html, contains_substring("Spring tour"));
}

#[component]
fn SelectedBool() -> Element {
    use_form(|| form_for(
        &a_trip(),
        form2! {
            Trip {
                confirmed => { control: select },
            }
        },
    ))
    .render()
}

#[gtest]
fn a_control_can_replace_a_derived_checkbox_with_a_select() {
    // A `bool` derives a checkbox. Overriding to `select` crosses a bigger gap
    // than one `<input type=…>` to another — a different component entirely — so
    // it checks the dispatch and not just the attribute.
    let html = render_to_html(SelectedBool);
    expect_that!(html, contains_substring("<select"));
    expect_that!(html, not(contains_substring("type=\"checkbox\"")));
}

// ── Both keys at once ────────────────────────────────────────────────────

#[component]
fn BothKeys() -> Element {
    use_form(|| empty_form(form2! {
        Trip {
            secret => { control: password, label: "Passphrase" },
        }
    }))
    .render()
}

#[gtest]
fn a_field_body_may_carry_both_keys() {
    // Two `.with_…()` calls against ONE map key. If `expand` built the key
    // differently for each, the second would land on a field that does not exist
    // and this would show only one of the two.
    let html = render_to_html(BothKeys);
    expect_that!(html, contains_substring("type=\"password\""));
    expect_that!(html, contains_substring("Passphrase"));
}

// ── A nested path ────────────────────────────────────────────────────────

#[component]
fn NestedLabel() -> Element {
    use_form(|| empty_form(form2! {
        Trip {
            venue.city => { label: "Town" },
        }
    }))
    .render()
}

#[gtest]
fn a_dotted_path_reaches_exactly_one_nested_field() {
    // The two failures this separates: a container that doesn't recurse loses the
    // override entirely, and one that recurses with the wrong prefix smears it
    // across siblings. Asserting the sibling kept "Street" rules out the second.
    let html = render_to_html(NestedLabel);
    expect_that!(html, contains_substring("Town"));
    expect_that!(html, contains_substring("Street"));
    expect_that!(html, not(contains_substring(">City<")));
}

// ── `[]`, the row selector ───────────────────────────────────────────────

#[component]
fn ListLabelAndRowControls() -> Element {
    // Both halves of the list vocabulary in one spec: `stops` addresses the
    // `ListSet` itself (its legend), `stops[]` every row. That they can coexist is
    // the reason the parser compares on the rendered key rather than on idents.
    use_form(|| form_for(
        &a_trip(),
        form2! {
            Trip {
                stops => { label: "Stops along the way" },
                stops[] => { control: email },
            }
        },
    ))
    .render()
}

#[gtest]
fn a_row_selector_reaches_every_row_and_the_list_keeps_its_own_label() {
    let html = render_to_html(ListLabelAndRowControls);
    // The list's own label, on the `fieldset`'s `legend`.
    expect_that!(html, contains_substring("<legend>Stops along the way</legend>"));
    // And every row got the control — two rows in `a_trip`, so exactly two.
    expect_that!(html.matches("type=\"email\"").count(), eq(2));
    // Rows still hold their values: the override is presentational only.
    expect_that!(html, contains_substring("a@example.com"));
    expect_that!(html, contains_substring("b@example.com"));
}

// ── The whole chain in one invocation ────────────────────────────────────

#[component]
fn EverythingAtOnce() -> Element {
    use_form(|| form_for(
        &a_trip(),
        form2! {
            Trip {
                title: "Edit trip",
                validator: trip_has_a_name,
                name => { label: "Trip name" },
                secret => { control: password, label: "Passphrase" },
                venue.city => { label: "Town" },
                stops => { label: "Stops" },
                stops[] => { control: email },
            }
        },
    ))
    .render()
}

fn trip_has_a_name(t: &Trip) -> Vec<formoxus::error::FormError> {
    if t.name.is_empty() {
        vec![formoxus::error::FormError("a trip needs a name".to_string())]
    } else {
        Vec::new()
    }
}

#[gtest]
fn every_entry_kind_coexists_in_one_spec() {
    // Six entries of four kinds against one model. The failure this is really
    // watching for is an entry being dropped — a builder chain assembled in the
    // wrong order, or `#(#fields)*` left out — which no single-entry test above
    // would catch.
    let html = render_to_html(EverythingAtOnce);
    expect_that!(html, contains_substring("Trip name"));
    expect_that!(html, contains_substring("Passphrase"));
    expect_that!(html, contains_substring("Town"));
    expect_that!(html, contains_substring("<legend>Stops</legend>"));
    expect_that!(html.matches("type=\"email\"").count(), eq(2));
    expect_that!(html, contains_substring("type=\"password\""));
}

// ── The rest of the validator's contract ─────────────────────────────────

#[gtest]
fn a_passing_validator_lets_the_model_through() {
    // The control for the rejection test: a validator that returns no errors must
    // not be mistaken for "no validator", and must not swallow the model.
    let quiet = Article { headline: "Trees are good".to_string(), words: 400 };
    let mut state = formoxus::reflect::form_for(
        &quiet,
        form2! { Article { validator: headline_is_not_shouted } },
    );
    expect_that!(state.validate(), some(eq(&quiet)));
    expect_that!(state.has_errors(), eq(false));
}

#[gtest]
fn a_validators_message_lands_where_form_errors_render() {
    // Not just "returns None" — the reason has to reach the same `errors` vec that
    // `FormState::render` draws its `form-error` markup from, which is what makes
    // it visible to a user. (That the vec renders is covered in-crate by
    // `reflect::tests::forms::a_pushed_error_renders`.)
    let shouted = Article { headline: "TREES ARE GOOD".to_string(), words: 400 };
    let mut state = formoxus::reflect::form_for(
        &shouted,
        form2! { Article { validator: headline_is_not_shouted } },
    );
    let _ = state.validate();
    expect_that!(
        state.errors.iter().map(|e| e.0.clone()).collect::<Vec<_>>(),
        elements_are![eq("headline is all caps")]
    );
}

#[gtest]
fn a_second_validate_clears_the_first_ones_verdict() {
    // A form the user then fixes must be accepted. Without the clear at the top of
    // `validate`, the first rejection would be permanent.
    let shouted = Article { headline: "TREES ARE GOOD".to_string(), words: 400 };
    let mut state = formoxus::reflect::form_for(
        &shouted,
        form2! { Article { validator: headline_is_not_shouted } },
    );
    expect_that!(state.validate(), none());

    state.apply(&std::collections::HashMap::from([
        ("headline".to_string(), "Trees are good".to_string()),
        ("words".to_string(), "400".to_string()),
    ]));
    expect_that!(state.validate(), some(anything()));
    expect_that!(state.has_errors(), eq(false));
}

/// Counts its calls, so a test can assert it did NOT run. Has its own static
/// rather than sharing one, since tests run in parallel.
static SHORT_CIRCUIT_CALLS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn counting_validator(_: &Article) -> Vec<formoxus::error::FormError> {
    SHORT_CIRCUIT_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Vec::new()
}

#[gtest]
fn a_field_error_stops_the_validator_from_running_at_all() {
    // It CANNOT run: it takes `&T`, and there is no `T` to hand it until every
    // field has parsed. Worth pinning as a behaviour rather than leaving it
    // implicit, because a validator may reasonably assume its input is
    // well-formed — `words` here never parsed, so no `Article` exists.
    let mut state = formoxus::reflect::form_for(
        &Article { headline: "Trees".to_string(), words: 1 },
        form2! { Article { validator: counting_validator } },
    );
    state.apply(&std::collections::HashMap::from([
        ("headline".to_string(), "Trees".to_string()),
        ("words".to_string(), "not a number".to_string()),
    ]));

    expect_that!(state.validate(), none());
    expect_that!(state.has_errors(), eq(true));
    expect_that!(
        SHORT_CIRCUIT_CALLS.load(std::sync::atomic::Ordering::SeqCst),
        eq(0)
    );
}
