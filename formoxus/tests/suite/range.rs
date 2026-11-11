//! `widget: range`, `<input type="range">` (`WIDGETS_PLAN.md` part 1).
//!
//! **A slider cannot show "no value"** (decision A). Given `value=""` the
//! browser draws the midpoint of `min` and `max`, so a range field that would
//! start empty starts at exactly the value the browser would draw, and the
//! store, the model and the screen agree from the first render. HTML's rule:
//! the default value is `min + (max − min) / 2`, and a value that is not on a
//! step moves to the NEAREST one (`min` plus a multiple of `step`), the larger
//! of two equally near. A range's `step` is 1 unless given, which is why a
//! float range must be given one.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::path::NodePath;
use formoxus::prelude::*;

use super::FORM_ID;
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Mixer {
    volume: i32,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Dial {
    level: f64,
}

fn render(app: fn() -> Element) -> String {
    super::render_to_html(app)
}

/// The range's own `<input …>` tag in `html`, so a test can ask what is on it
/// without depending on the order its attributes are written in. Panics if
/// there is none, which is itself the failure.
fn range_tag(html: &str) -> String {
    html.split("<input ")
        .skip(1)
        .map(|rest| rest.split_once('>').map_or(rest, |(tag, _)| tag))
        .find(|tag| tag.contains(r#"type="range""#))
        .unwrap_or_else(|| panic!("no range input in {html}"))
        .to_string()
}

/// The model an untouched `empty_form` would give: decision A's starting value,
/// read through `validate`, so it is the MODEL that is checked and not only
/// the markup.
fn starting_model<T>(spec: FormSpec<T>) -> Option<T>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static> + 'static,
{
    empty_form(spec).validate()
}

// ── The markup ───────────────────────────────────────────────────────────

#[gtest]
fn a_range_renders_type_range_with_its_bounds_as_written() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! { Mixer { volume => { widget: range, min: 0, max: 10, step: 2 } } })
        });
        form.render(using_fns! {})
    }
    let tag = range_tag(&render(App));
    expect_that!(tag, contains_substring(r#"name="volume""#));
    expect_that!(tag, contains_substring("min=0"));
    expect_that!(tag, contains_substring("max=10"));
    expect_that!(tag, contains_substring("step=2"));
}

/// **No `required` attribute, though the field is required.** HTML does not
/// allow `required` on a range, and a range always has a value, so there is
/// nothing for it to enforce (the plan's "Settled by HTML"). `volume` is an
/// `i32`, not an `Option`, which on a `number` input WOULD render `required`.
#[gtest]
fn a_required_range_has_no_required_attribute() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! { Mixer { volume => { widget: range, min: 0, max: 10 } } })
        });
        form.render(using_fns! {})
    }
    expect_that!(range_tag(&render(App)), not(contains_substring("required")));
}

/// The `<output>` (decision B) names the slider with `for`, by its page id,
/// and shows the value the slider holds.
#[gtest]
fn the_output_shows_the_sliders_value() {
    #[component]
    fn App() -> Element {
        let mixer = Mixer { volume: 7 };
        let form = use_form(move || {
            form_for(
                &mixer,
                form! { Mixer { volume => { widget: range, min: 0, max: 10 } } },
            )
        });
        form.render(using_fns! {})
    }
    expect_that!(
        render(App),
        contains_regex(format!(
            r#"<output [^>]*for="{FORM_ID}-volume"[^>]*>7</output>"#
        ))
    );
}

#[gtest]
fn omit_value_leaves_the_output_out() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! { Mixer { volume => { widget: range, min: 0, max: 10, omit_value } } })
        });
        form.render(using_fns! {})
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"type="range""#));
    expect_that!(html, not(contains_substring("<output")));
}

// ── Where an empty range starts (decision A) ─────────────────────────────

/// The input, the `<output>` and the model all say 5: the first render already
/// agrees with what the browser draws.
#[gtest]
fn an_empty_range_starts_at_its_midpoint() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! { Mixer { volume => { widget: range, min: 0, max: 10 } } })
        });
        form.render(using_fns! {})
    }
    let html = render(App);
    expect_that!(range_tag(&html), contains_substring(r#"value="5""#));
    expect_that!(html, contains_regex(r"<output [^>]*>5</output>"));
    expect_that!(
        starting_model(form! { Mixer { volume => { widget: range, min: 0, max: 10 } } }),
        some(eq(&Mixer { volume: 5 }))
    );
}

/// 5 is not a multiple of 3, and 6 is nearer to it than 3 is. Rounding DOWN
/// would give 3, which the browser does not draw.
#[gtest]
fn a_midpoint_off_the_step_moves_to_the_nearest_step() {
    expect_that!(
        starting_model(form! { Mixer { volume => { widget: range, min: 0, max: 10, step: 3 } } }),
        some(eq(&Mixer { volume: 6 }))
    );
    expect_that!(
        starting_model(form! { Mixer { volume => { widget: range, min: 0, max: 10, step: 4 } } }),
        some(eq(&Mixer { volume: 4 }))
    );
}

/// The midpoint of 1 and 4 is 2.5, exactly between 2 and 3: HTML takes the
/// larger. Steps count from `min`, not from 0.
#[gtest]
fn a_midpoint_between_two_steps_takes_the_larger() {
    expect_that!(
        starting_model(form! { Mixer { volume => { widget: range, min: 1, max: 4 } } }),
        some(eq(&Mixer { volume: 3 }))
    );
}

/// A float range, which must be given a `step:` (a range steps by 1 unless
/// told otherwise, so without one a float range offers only whole numbers;
/// `form_range_float_without_step.rs` pins the refusal). The same rule as an
/// integer's: 0.5 is on a step of 0.1, and with a step of 0.4 it moves to the
/// nearer of 0.4 and 0.8.
#[gtest]
fn a_float_range_starts_on_its_step() {
    expect_that!(
        starting_model(
            form! { Dial { level => { widget: range, min: 0.0, max: 1.0, step: 0.1 } } }
        ),
        some(eq(&Dial { level: 0.5 }))
    );
    expect_that!(
        starting_model(
            form! { Dial { level => { widget: range, min: 0.0, max: 1.0, step: 0.4 } } }
        ),
        some(eq(&Dial { level: 0.4 }))
    );
}

/// A model that gives a value keeps it: the starting value is only for a field
/// that would otherwise be empty.
#[gtest]
fn a_given_value_is_not_replaced_by_the_midpoint() {
    #[component]
    fn App() -> Element {
        let mixer = Mixer { volume: 2 };
        let form = use_form(move || {
            form_for(
                &mixer,
                form! { Mixer { volume => { widget: range, min: 0, max: 10 } } },
            )
        });
        form.render(using_fns! {})
    }
    expect_that!(range_tag(&render(App)), contains_substring(r#"value="2""#));
}

/// `reset` goes back to where the form started, which for an empty range is
/// the midpoint, not `""`.
#[gtest]
fn reset_returns_an_empty_range_to_its_midpoint() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! { Mixer { volume => { widget: range, min: 0, max: 10 } } })
        });
        form.write(&path!(Mixer.volume), "9")
            .expect("volume is a field of this form");
        form.reset();
        form.render(using_fns! {})
    }
    expect_that!(range_tag(&render(App)), contains_substring(r#"value="5""#));
}

// ── Across the wire ──────────────────────────────────────────────────────

fn mixer_spec() -> FormSpec<Mixer> {
    form! { Mixer { volume => { widget: range, min: 0, max: 10 } } }
}

/// A moved slider's value reaches the server. The server rebuilds from an
/// empty form, which starts at 5, so this also shows the wire value wins over
/// the starting one.
#[gtest]
fn a_moved_value_crosses_the_wire() {
    let wire = WireForm::from_model(&Mixer { volume: 7 }, mixer_spec());
    expect_that!(
        Submission::accept(mixer_spec(), wire.values()).map(Submission::into_model),
        ok(eq(&Mixer { volume: 7 }))
    );
}

/// An untouched slider sends the midpoint the user saw, not `""`: the reason
/// for decision A, end to end.
#[gtest]
fn an_untouched_range_sends_its_midpoint() {
    let values = empty_form(mixer_spec()).collect_values();
    expect_that!(
        Submission::accept(mixer_spec(), &values).map(Submission::into_model),
        ok(eq(&Mixer { volume: 5 }))
    );
}

/// **A request that leaves the range out entirely gets the midpoint**, not a
/// "required" error. The server rebuilds from an empty form, which starts at
/// the midpoint, and a field the request does not mention is left as it stands
/// (`distribute_values`). Unlike every other required field, which an omitted
/// value leaves empty. Pinned on purpose: a slider always has a value, so a
/// browser never sends a range without one, and only a hand-built request can
/// leave it out.
#[gtest]
fn a_request_without_the_range_gets_its_midpoint() {
    let nothing = std::iter::empty::<(NodePath, String)>().collect();
    expect_that!(
        Submission::accept(mixer_spec(), &nothing).map(Submission::into_model),
        ok(eq(&Mixer { volume: 5 }))
    );
}

/// `min` and `max` are checked on the server too, as on a `number` input: a
/// forged 11 is an error on the field, not a value.
#[gtest]
fn a_forged_value_past_max_is_refused() {
    let values = [(NodePath::new("volume"), "11".to_string())]
        .into_iter()
        .collect();
    let Err(errors) = Submission::accept(mixer_spec(), &values) else {
        panic!("11 is past max");
    };
    expect_that!(errors.fields.get("volume"), some(anything()));
}
