//! `form!` from the outside, as a consumer writes it.
//!
//! These could not be written anywhere else: the macro expands to absolute
//! `::formoxus::` paths, which only resolve from a crate that is not formoxus.
//! Inside the library they would not compile, so this file is the only place
//! the expansion is exercised end to end rather than as tokens.

use std::collections::HashMap;

use facet::Facet;
use formoxus::form;
use formoxus::{FormErrors, Submission, empty_form, form_for, use_form};
use googletest::prelude::*;

use super::render_to_html;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Article {
    headline: String,
    words: u32,
}

/// The title of a form as `form!` finally stored it.
///
/// Goes through `empty_form` because `FormSpec`'s fields are private — and
/// that's the right route anyway: it's the one a page takes.
fn title_of(spec: form::FormSpec<Article>) -> Option<String> {
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
        title_of(form! { Article { title: "A literal" } }),
        some(eq("A literal"))
    );
}

#[gtest]
fn a_const_title_works() {
    expect_that!(
        title_of(form! { Article { title: HEADLINE } }),
        some(eq("From a const"))
    );
}

#[gtest]
fn a_static_title_works() {
    expect_that!(
        title_of(form! { Article { title: STATIC_HEADLINE } }),
        some(eq("From a static"))
    );
}

#[gtest]
fn a_fn_returning_string_works() {
    // The `&String -> &str` coercion, and the one whose temporary has the
    // shortest life: it must survive only until `with_title` copies it.
    expect_that!(
        title_of(form! { Article { title: owned_title() } }),
        some(eq("From a String fn"))
    );
}

#[gtest]
fn a_fn_returning_str_works() {
    expect_that!(
        title_of(form! { Article { title: borrowed_title() } }),
        some(eq("From a &str fn"))
    );
}

#[gtest]
fn a_format_call_works() {
    let count = 3;
    expect_that!(
        title_of(form! { Article { title: format!("{count} drafts") } }),
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
        title_of(form! { Article { title: from_the_page } }),
        some(eq("Editing “Trees”"))
    );
}

#[gtest]
fn a_conditional_expression_works() {
    let creating = false;
    expect_that!(
        title_of(
            form! { Article { title: if creating { "New article" } else { "Edit article" } } }
        ),
        some(eq("Edit article"))
    );
}

#[gtest]
fn a_spec_may_have_no_title() {
    expect_that!(title_of(form! { Article {} }), none());
}

// ── `validator:` ─────────────────────────────────────────────────────────

fn headline_is_not_shouted(a: &Article) -> Vec<formoxus::error::FormError> {
    if a.headline.chars().all(|c| !c.is_lowercase()) {
        vec![formoxus::error::FormError(
            "headline is all caps".to_string(),
        )]
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
        title_of(form! {
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
    let shouted = Article {
        headline: "TREES ARE GOOD".to_string(),
        words: 400,
    };
    let mut state = form_for(
        &shouted,
        form! { Article { validator: headline_is_not_shouted } },
    );

    expect_that!(state.validate(), none());
    expect_that!(state.has_errors(), eq(true));
}

// ══ Field specs reaching the built tree ══════════════════════════════════
//
// The gap these close: the macro crate's tests stop at the emitted tokens, and
// `specs.rs` deliberately builds its `FormSpec`s by hand so it
// stays honest about testing the *builder*. Neither would notice `expand()`
// emitting a wrong key string, or dropping the field arm of the chain. These go
// through `form!` and read the DOM, which is the only unambiguous evidence that
// an override arrived.

use dioxus::prelude::*;

/// Render a component to HTML with a real Dioxus runtime behind it.
///

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venue {
    street: String,
    city: String,
}

/// Wide enough to reach every addressing shape in one model: a scalar, a nested
/// struct, a list, and a `bool` whose derived widget is a checkbox (so a `select`
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
        venue: Venue {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
        },
        stops: vec!["a@example.com".to_string(), "b@example.com".to_string()],
        confirmed: true,
    }
}

// ── A label ──────────────────────────────────────────────────────────────

#[component]
fn LabelledName() -> Element {
    use_form(|| {
        empty_form(form! {
            Trip {
                name => { label: "Trip name" },
            }
        })
    })
    .render_fragment()
}

#[gtest]
fn a_label_from_the_macro_reaches_the_markup() {
    let html = render_to_html(LabelledName);
    expect_that!(html, contains_substring("Trip name"));
    // `default_label` would have made this "Name", so its absence is what shows
    // the override won rather than merely coexisting.
    expect_that!(html, not(contains_substring(">Name<")));
}

// ── A widget ────────────────────────────────────────────────────────────

#[component]
fn PasswordSecret() -> Element {
    use_form(|| {
        form_for(
            &a_trip(),
            form! {
                Trip {
                    secret => { widget: password },
                }
            },
        )
    })
    .render_fragment()
}

#[gtest]
fn a_widget_from_the_macro_changes_the_rendered_input() {
    let html = render_to_html(PasswordSecret);
    expect_that!(html, contains_substring("type=\"password\""));
    // The value round-trips like any other field's — `widget: password` changes
    // the masking, not the binding.
    expect_that!(html, contains_substring("hunter2"));
    // A sibling scalar keeps its default, so the override landed on one field.
    expect_that!(html, contains_substring("Spring tour"));
}

#[component]
fn SelectedBool() -> Element {
    use_form(|| {
        form_for(
            &a_trip(),
            form! {
                Trip {
                    confirmed => { widget: select },
                }
            },
        )
    })
    .render_fragment()
}

#[gtest]
fn a_widget_can_replace_a_derived_checkbox_with_a_select() {
    // A `bool` derives a checkbox. Overriding to `select` crosses a bigger gap
    // than one `<input type=…>` to another — a different component entirely — so
    // it checks the dispatch and not just the attribute.
    let html = render_to_html(SelectedBool);
    expect_that!(html, contains_substring("<select"));
    expect_that!(html, not(contains_substring("type=\"checkbox\"")));
}

#[component]
fn TextareaSecret() -> Element {
    use_form(|| {
        form_for(
            &a_trip(),
            form! {
                Trip {
                    secret => { widget: textarea },
                }
            },
        )
    })
    .render_fragment()
}

#[gtest]
fn a_widget_can_override_text_to_a_textarea() {
    // `render_widget`'s panic is survivable under SSR — a sibling field's
    // markup keeps showing up even when this one fails to render — so this
    // asserts directly on the overridden field rather than on the page as a
    // whole. See `.claude/memory/next_up_two_todos.md`.
    let html = render_to_html(TextareaSecret);
    expect_that!(html, contains_substring("<textarea"));
    expect_that!(html, contains_substring("hunter2"));
    expect_that!(html, not(contains_substring("type=\"password\"")));
}

// ── Both keys at once ────────────────────────────────────────────────────

#[component]
fn BothKeys() -> Element {
    use_form(|| {
        empty_form(form! {
            Trip {
                secret => { widget: password, label: "Passphrase" },
            }
        })
    })
    .render_fragment()
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
    use_form(|| {
        empty_form(form! {
            Trip {
                venue.city => { label: "Town" },
            }
        })
    })
    .render_fragment()
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
fn ListLabelAndRowWidgets() -> Element {
    // Both halves of the list vocabulary in one spec: `stops` addresses the
    // `ListSet` itself (its legend), `stops[]` every row. That they can coexist is
    // the reason the parser compares on the rendered key rather than on idents.
    use_form(|| {
        form_for(
            &a_trip(),
            form! {
                Trip {
                    stops => { label: "Stops along the way" },
                    stops[] => { widget: email },
                }
            },
        )
    })
    .render_fragment()
}

#[gtest]
fn a_row_selector_reaches_every_row_and_the_list_keeps_its_own_label() {
    let html = render_to_html(ListLabelAndRowWidgets);
    // The list's own label, on the `fieldset`'s `legend`.
    expect_that!(
        html,
        contains_substring("<legend>Stops along the way</legend>")
    );
    // And every row got the widget — two rows in `a_trip`, so exactly two.
    expect_that!(html.matches("type=\"email\"").count(), eq(2));
    // Rows still hold their values: the override is presentational only.
    expect_that!(html, contains_substring("a@example.com"));
    expect_that!(html, contains_substring("b@example.com"));
}

// ── The whole chain in one invocation ────────────────────────────────────

#[component]
fn EverythingAtOnce() -> Element {
    use_form(|| {
        form_for(
            &a_trip(),
            form! {
                Trip {
                    title: "Edit trip",
                    validator: trip_has_a_name,
                    name => { label: "Trip name" },
                    secret => { widget: password, label: "Passphrase" },
                    venue.city => { label: "Town" },
                    stops => { label: "Stops" },
                    stops[] => { widget: email },
                }
            },
        )
    })
    .render_fragment()
}

fn trip_has_a_name(t: &Trip) -> Vec<formoxus::error::FormError> {
    if t.name.is_empty() {
        vec![formoxus::error::FormError(
            "a trip needs a name".to_string(),
        )]
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
    // The widget for the rejection test: a validator that returns no errors must
    // not be mistaken for "no validator", and must not swallow the model.
    let quiet = Article {
        headline: "Trees are good".to_string(),
        words: 400,
    };
    let mut state = form_for(
        &quiet,
        form! { Article { validator: headline_is_not_shouted } },
    );
    expect_that!(state.validate(), some(eq(&quiet)));
    expect_that!(state.has_errors(), eq(false));
}

#[gtest]
fn a_validators_message_lands_where_form_errors_render() {
    // Not just "returns None" — the reason has to reach the same `errors` vec that
    // `FormState::render` draws its `form-error` markup from, which is what makes
    // it visible to a user. (That the vec renders is covered in-crate by
    // `tests::forms::a_pushed_error_renders`.)
    let shouted = Article {
        headline: "TREES ARE GOOD".to_string(),
        words: 400,
    };
    let mut state = form_for(
        &shouted,
        form! { Article { validator: headline_is_not_shouted } },
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
    let shouted = Article {
        headline: "TREES ARE GOOD".to_string(),
        words: 400,
    };
    let mut state = form_for(
        &shouted,
        form! { Article { validator: headline_is_not_shouted } },
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
static SHORT_CIRCUIT_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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
    let mut state = form_for(
        &Article {
            headline: "Trees".to_string(),
            words: 1,
        },
        form! { Article { validator: counting_validator } },
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

// ── `custom(Widget)` ─────────────────────────────────────────────────────
//
// The escape hatch for a value kind no built-in widget can serve. These are
// here, and not as an inline `mod tests`, for the usual reason: a custom
// widget is a component in the CONSUMING crate, which is exactly what the
// macro has to expand against.

/// Stands in for `ui::MarkdownInput` and the source picker — a component taking
/// the `(values, props)` pair every built-in widget gets.
///
/// It reads `props` to prove the boundary arrives intact, and calls `use_hook`
/// to prove the widget gets a component scope of its own. That second part is
/// the load-bearing one: the real widgets need `use_resource` (to fetch a
/// picker's choices) and `use_signal` (to hold a preview toggle), which a plain
/// function call from `render_widget` could not provide.
#[component]
fn ShoutyWidget(values: formoxus::ValuesByPath, props: formoxus::widgets::FieldProps) -> Element {
    let _ = values;
    let marker = use_hook(|| "scope-ok");
    let label = props.label.clone().unwrap_or_default();
    rsx! {
        div { class: "shouty", "data-marker": "{marker}", "data-path": "{props.path}",
            "{label.to_uppercase()}"
        }
    }
}

#[component]
fn CustomSecret() -> Element {
    use_form(|| {
        form_for(
            &a_trip(),
            form! {
                Trip {
                    secret => { widget: custom(ShoutyWidget) },
                }
            },
        )
    })
    .render_fragment()
}

#[gtest]
fn a_custom_widget_replaces_the_default_widget_entirely() {
    let html = render_to_html(CustomSecret);
    expect_that!(html, contains_substring(r#"class="shouty""#));
    expect_that!(
        html,
        not(contains_substring(r#"name="secret""#)),
        "the custom widget replaces the input, it doesn't render alongside it"
    );
}

#[gtest]
fn a_custom_widget_receives_the_field_props() {
    // `path` is what the widget writes back through, and `label` is the derived
    // Title Case one — so this pins that the boundary arrives populated, not
    // defaulted.
    let html = render_to_html(CustomSecret);
    expect_that!(html, contains_substring(r#"data-path="secret""#));
    expect_that!(html, contains_substring("SECRET"));
}

#[gtest]
fn a_custom_widget_gets_its_own_component_scope() {
    // `use_hook` panics outside a component. Rendering at all is the assertion;
    // the marker just makes the failure legible if the mechanism ever changes to
    // calling the widget as a plain function.
    let html = render_to_html(CustomSecret);
    expect_that!(html, contains_substring(r#"data-marker="scope-ok""#));
}

#[gtest]
fn sibling_fields_still_render_their_normal_widgets() {
    // A custom widget is per-field. Nothing about naming one for `secret`
    // should disturb how `name` or `confirmed` render.
    let html = render_to_html(CustomSecret);
    expect_that!(html, contains_substring(r#"name="name""#));
    expect_that!(html, contains_substring(r#"type="checkbox""#));
}

// ── Keyword-named fields ─────────────────────────────────────────────────

/// A model that could not be named in `form!` at all until now. `r#type` is an
/// ordinary Rust field name and a common one in anything mirroring a JSON API,
/// so a form library that cannot address it has a hole in its reach.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Token {
    r#type: String,
    value: String,
}

#[component]
fn RawSpelling() -> Element {
    use_form(|| empty_form(form! { Token { r#type => { label: "Kind" } } })).render_fragment()
}

#[component]
fn BareSpelling() -> Element {
    use_form(|| empty_form(form! { Token { type => { label: "Kind" } } })).render_fragment()
}

/// The macro-side tests pin the tokens this emits; only a consumer test pins
/// that they COMPILE. The witness expands to `__s.r#type` field access, which
/// is a syntax error if the ident is not raw — and this is the only place that
/// would show.
///
/// That the label arrives is the second half: the spec key has to be `type`,
/// the name facet reports, not `r#type` as written. `default_label` would have
/// produced "Type", so its absence is what shows the override landed on the
/// right field rather than merely coexisting with it.
#[gtest]
fn a_keyword_named_field_is_addressable_by_its_raw_spelling() {
    let html = render_to_html(RawSpelling);
    expect_that!(html, contains_substring("Kind"));
    expect_that!(html, not(contains_substring(">Type<")));
}

/// And bare, which is what an author is likelier to type inside a macro, since
/// nothing there forces the `r#`. Both spellings have to reach the same field.
#[gtest]
fn a_keyword_named_field_is_addressable_bare() {
    let html = render_to_html(BareSpelling);
    expect_that!(html, contains_substring("Kind"));
    expect_that!(html, not(contains_substring(">Type<")));
}

// ── Constraints ──────────────────────────────────────────────────────────

fn wire(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn failing_paths(errors: &FormErrors) -> Vec<String> {
    errors.fields.iter().map(|(p, _)| p.clone()).collect()
}

/// The whole chain the macro was the last missing link in: `form!` -> the
/// `.with_constraints` call -> `FieldSpec` -> `apply_specs` -> `FormField` ->
/// `ValueKind::check`. `specs.rs` covers the same ground through a hand-built
/// `FormSpec`; this is the only test that proves the macro reaches it.
#[gtest]
fn constraints_from_the_macro_reach_validation() {
    let spec = || {
        form! {
            Article {
                headline => { max_length: 5 },
                words => { min: 10, max: 100 },
            }
        }
    };

    let ok = wire(&[("headline", "Short"), ("words", "42")]);
    expect_that!(
        Submission::accept(spec(), &ok).is_ok(),
        eq(true),
        "both values are inside the stated bounds"
    );

    let bad = wire(&[("headline", "Much too long"), ("words", "3")]);
    let errors = Submission::accept(spec(), &bad).expect_err("both values are outside them");
    expect_that!(
        failing_paths(&errors),
        unordered_elements_are![eq("headline"), eq("words")]
    );
}

/// `min: 10` on a `u32` is an unsuffixed integer literal, so it arrives as
/// `Bound::Int` — and a float field would need it widened. This is the same
/// conversion `a_bound_is_converted_rather_than_classified` pins in tokens,
/// seen from the far end.
#[gtest]
fn an_integer_bound_written_bare_lands_on_the_right_field() {
    let spec = || form! { Article { words => { min: 10 } } };
    let under = wire(&[("headline", "Fine"), ("words", "9")]);
    let errors = Submission::accept(spec(), &under).expect_err("9 is below the stated minimum");
    expect_that!(failing_paths(&errors), elements_are![eq("words")]);
}

/// A pattern is a literal so the macro can hand it to `regress`; it reaches the
/// field as a `&'static str` and is anchored at check time.
#[gtest]
fn a_pattern_from_the_macro_reaches_validation() {
    let spec = || form! { Article { headline => { pattern: r"[A-Z].*" } } };

    let ok = wire(&[("headline", "Capitalized"), ("words", "1")]);
    expect_that!(Submission::accept(spec(), &ok).is_ok(), eq(true));

    let bad = wire(&[("headline", "lowercase"), ("words", "1")]);
    let errors = Submission::accept(spec(), &bad).expect_err("it does not start with a capital");
    expect_that!(failing_paths(&errors), elements_are![eq("headline")]);
}

// ── The compile-time constraint checks ───────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Hall {
    city: String,
    seats: u32,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct PriceRow {
    price: f64,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Blurb(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Listing {
    note: Option<String>,
    hall: Hall,
    tags: Vec<String>,
    rows: Vec<PriceRow>,
    bio: Blurb,
}

/// Every place a constraint may legitimately sit, so the `const _` checks
/// `form!` emits must let each through: an `Option` is judged by what it
/// holds, a nested field and a `[]` row by the leaf's own type, and a newtype
/// is let through because its inside is out of reach at compile time. The
/// rejections are trybuild goldens in `tests/ui/`, since they cannot compile.
#[gtest]
fn a_constraint_is_accepted_wherever_its_field_can_take_it() {
    let spec = || {
        form! {
            Listing {
                note => { max_length: 3 },
                hall.city => { min_length: 1 },
                hall.seats => { min: 1, max: 500 },
                tags[] => { pattern: "[a-z]+" },
                rows[].price => { min: 0, max: 99.5 },
                bio => { max_length: 280 },
            }
        }
    };

    let bad = wire(&[
        ("note", "too long"),
        ("hall.city", "Paris"),
        ("hall.seats", "10"),
        ("bio", "fine"),
    ]);
    let errors = Submission::accept(spec(), &bad).expect_err("`note` is over its limit");
    expect_that!(failing_paths(&errors), elements_are![eq("note")]);
}
