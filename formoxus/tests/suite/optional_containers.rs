//! `Option` composes with every member kind — it is not a fifth kind of its own.
//!
//! `member_for_shape` peels ONE `Option` layer and recurses, so the recursion IS
//! the dispatch and no match is duplicated. The result is wrapped in an
//! `OptionMember`, a decorator that owns the `begin_some`/`set_default` frame and
//! intercepts `validate` — absent means don't validate the inner, which is what
//! unwound `FormField::required`'s double duty.
//!
//! These were the RED target for that work: before it, scalars and enums behind
//! an `Option` worked but **structs and lists did not**, in either direction.
//! `Some(v)` panicked (`begin_field` landed on the `Option` slot, so the inner
//! `begin_field`/`init_list` hit `Option`'s own enum shape) and `None` failed
//! *silently* — `validate()` returned `None`, because the inner fields were
//! required and `Empty`, making an absent optional container unrepresentable.
//! They all pass now and stand as the regression net.
//!
//! Optional *enums* stay covered by `enums` (`edit_mode_round_trips_an_optional_
//! enum`, `create_mode_absent_builds_a_none`, …) — the guard for having retired
//! `VariantSet::optional`, which `OptionMember` subsumes. `VariantChoice::Absent`
//! deliberately survives: a chosen unit variant has no leaves, so presence there
//! cannot be derived the way it is for structs and lists.

use super::Harness;
use super::models::{Location, Mode};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::*;
use googletest::prelude::*;
use std::{collections::HashMap, fmt::Debug};

/// `Option<Struct>` — the case that used to panic one way and lie the other.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Contact {
    name: String,
    address: Option<Location>,
}

/// `Option<Vec<Scalar>>`. Moved here from `vecs`, where it was the lone ignored
/// list test — it was never a list bug, it was this one.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Tagged {
    tags: Option<Vec<String>>,
}

/// `Option<Vec<Struct>>` — the write path has to compose `begin_some` →
/// `init_list` → `begin_list_item` → `begin_field`, one frame per layer.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Roster {
    members: Option<Vec<Location>>,
}

/// `Option<Vec<Option<Scalar>>>` — two `Option`s at different depths, which is
/// exactly what "peel one layer per recursion" buys and what the old single
/// up-front unwrap could never reach.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Matrix {
    cells: Option<Vec<Option<String>>>,
}

fn springfield() -> Location {
    Location {
        street: "123 Main St".to_string(),
        city: "Springfield".to_string(),
        zip: "12345".to_string(),
    }
}

fn contact(address: Option<Location>) -> Contact {
    Contact {
        name: "Ada".to_string(),
        address,
    }
}

fn applied(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(p, v)| (p.to_string(), v.to_string()))
        .collect()
}

fn paths<T: Clone + Debug + PartialEq + Facet<'static>>(form: &FormState<T>) -> Vec<String> {
    form.leaves().into_iter().map(|(p, _)| p).collect()
}

// ── Shape of the form ──

#[gtest]
fn an_absent_optional_struct_still_offers_its_leaves() {
    // Passes today, and the decorator must not break it: an absent optional
    // struct still renders its inner inputs, empty. If it hid them there
    // would be no way to *fill one in* — presence is derived from what the
    // user types, so the inputs have to be there to type into.
    //
    // Also pins that `Option` contributes no path segment of its own:
    // `address.street`, never `address.some.street`.
    let form = form_for(&contact(None), FormSpec::default());
    expect_that!(
        paths(&form),
        elements_are![
            eq("name"),
            eq("address.street"),
            eq("address.city"),
            eq("address.zip")
        ]
    );
    expect_that!(
        form.leaves(),
        eq(&vec![
            ("name".to_string(), "Ada".to_string()),
            ("address.street".to_string(), String::new()),
            ("address.city".to_string(), String::new()),
            ("address.zip".to_string(), String::new()),
        ])
    );
}

// ── Option<Struct> ──

#[gtest]
fn a_present_optional_struct_round_trips() {
    let value = contact(Some(springfield()));
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}

#[gtest]
fn an_absent_optional_struct_round_trips() {
    // The dangerous one. No panic today: the inner fields are required and
    // `Empty`, so `validate()` reports errors and hands back `None` — an
    // absent optional struct simply cannot be expressed. The wrapper has to
    // intercept `validate`, not just the write path.
    let value = contact(None);
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}

#[gtest]
fn create_mode_leaves_an_untouched_optional_struct_absent() {
    // Same rule arriving through the DOM path rather than through populating —
    // the two boundaries have to agree, as they now do for `""`.
    let mut form = empty_form::<Contact>(FormSpec::default());
    form.apply(&applied(&[("name", "Ada")]));
    expect_that!(form.validate(), some(eq(&contact(None))));
}

#[gtest]
fn filling_in_an_absent_optional_struct_makes_it_present() {
    // `present` is DERIVED, never asked: the user typing into the inner
    // inputs is what makes the container `Some`. No third construction
    // question, and no "is there an address?" checkbox.
    let mut form = form_for(&contact(None), FormSpec::default());
    form.apply(&applied(&[
        ("address.street", "123 Main St"),
        ("address.city", "Springfield"),
        ("address.zip", "12345"),
    ]));
    expect_that!(form.validate(), some(eq(&contact(Some(springfield())))));
}

#[gtest]
fn blanking_a_present_optional_struct_makes_it_absent() {
    // The inverse, and the same rule one level up from `""` IS absence:
    // every leaf underneath empty ⟺ the container is absent.
    let mut form = form_for(&contact(Some(springfield())), FormSpec::default());
    form.apply(&applied(&[
        ("address.street", ""),
        ("address.city", ""),
        ("address.zip", ""),
    ]));
    expect_that!(form.validate(), some(eq(&contact(None))));
}

#[gtest]
fn a_partly_filled_optional_struct_is_an_error() {
    // Green today, but only vacuously — everything under an `Option<Struct>`
    // is required right now, so *any* partial fill errors. Its real job is
    // as a live guard while the decorator lands: absent has to mean EVERY
    // leaf empty, so a wrapper that treats "some leaf empty" as absent (and
    // quietly drops the street the user typed) fails here.
    //
    // Falls straight out of the two rules above: some leaf is non-empty, so
    // the container is present, so its inner *required* fields are required
    // again. "Optional" is about the whole address, not about each line of
    // it — a street with no city is a half-answered address, not an absent
    // one.
    let mut form = form_for(&contact(None), FormSpec::default());
    form.apply(&applied(&[("address.street", "123 Main St")]));
    expect_that!(form.validate(), none());
    expect_that!(form.has_errors(), eq(true));
}

// ── Option<Vec<T>> ──

#[gtest]
fn a_present_optional_list_round_trips() {
    let value = Tagged {
        tags: Some(vec!["x".to_string()]),
    };
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}

#[gtest]
fn an_absent_optional_list_round_trips() {
    let value = Tagged { tags: None };
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}

#[gtest]
fn an_empty_optional_list_collapses_to_absent() {
    // DESIGN QUESTION, not a settled rule. `Some(vec![])` has no leaves at
    // all, so "absent ⟺ every leaf underneath is empty" is vacuously true
    // and it comes back as `None` — the exact analogue of `Some("")`
    // collapsing to `None`, and unrepresentable in the DOM for the same
    // reason (a zero-row list submits nothing).
    //
    // It may not survive create-mode lengths (VEC_PLAN step 4), where a
    // caller could legitimately ask for a present, zero-row list. If that
    // wins, flip this test to expect `Some(vec![])` and derive presence from
    // the *length choice* rather than from emptiness.
    let mut form = form_for(
        &Tagged {
            tags: Some(Vec::new()),
        },
        FormSpec::default(),
    );
    expect_that!(form.validate(), some(eq(&Tagged { tags: None })));
}

#[gtest]
fn an_optional_list_of_structs_round_trips() {
    // Three frames deep on the write path — `begin_some` → `init_list` →
    // `begin_list_item` → `begin_field` — which is where a decorator that
    // forwards `write_into` instead of `write_value_into` comes apart.
    let value = Roster {
        members: Some(vec![springfield()]),
    };
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}

// ── Composition ──

#[gtest]
fn option_peels_one_layer_at_a_time() {
    // `Option<Vec<Option<String>>>` →
    // `OptionalMember(ListSet(rows of OptionalMember(FormField)))`, each
    // layer contributing exactly one frame. Row 1 is empty, so it's `None`
    // by the same rule that makes the whole list present: some leaf under
    // `cells` is non-empty.
    let value = Matrix {
        cells: Some(vec![Some("a".to_string()), None]),
    };
    let form = form_for(&value, FormSpec::default());
    expect_that!(paths(&form), elements_are![eq("cells.#0"), eq("cells.#1")]);

    let mut form = form;
    expect_that!(form.validate(), some(eq(&value)));
}

// ── Presence that the leaves cannot see ──
//
// Every other member kind answers `is_present` from something a leaf carries, so
// a wrapper that scans leaves itself gets the same answer by accident. A chosen
// FIELDLESS variant breaks that: it contributes no leaves at all, and only
// `VariantSet` knows it was chosen. These are the tests that fail if
// `OptionMember::is_present` inspects leaves instead of asking its inner member.

/// `Option<Enum>` where the chosen variant carries no fields.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Job {
    name: String,
    mode: Option<Mode>,
}

/// `Option<Vec<Enum>>` of fieldless variants — presence has to survive being
/// asked through two containers.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Schedule {
    modes: Option<Vec<Mode>>,
}

#[gtest]
fn a_chosen_unit_variant_survives_behind_an_option() {
    // The regression. Scanning leaves reports "absent" for a present unit
    // variant, so `write_value_into` takes the `set_default()` branch and the
    // user's choice silently becomes `None` — no panic, no error, just gone.
    let value = Job {
        name: "nightly".to_string(),
        mode: Some(Mode::Fast),
    };
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}

#[gtest]
fn an_absent_optional_unit_variant_stays_absent() {
    // The other direction, and NOT redundant: it's what stops the bug above from
    // being "fixed" by having `is_present` answer `true` unconditionally. Both
    // tests together pin presence to the choice rather than to either constant.
    let value = Job {
        name: "nightly".to_string(),
        mode: None,
    };
    let mut form = form_for(&value, FormSpec::default());
    expect_that!(form.validate(), some(eq(&value)));
}

#[gtest]
fn a_chosen_unit_variant_survives_two_containers_deep() {
    // `OptionMember` -> `ListSet` -> `VariantSet`, none of which has a leaf to
    // its name. Presence has to be asked for, one member at a time, the whole
    // way down.
    let value = Schedule {
        modes: Some(vec![Mode::Fast, Mode::Slow]),
    };
    let form = form_for(&value, FormSpec::default());
    expect_that!(
        form.leaves(),
        eq(&Vec::new()),
        "fieldless variants have no leaves"
    );

    let mut form = form;
    expect_that!(form.validate(), some(eq(&value)));
}

#[component]
fn EmptyContactForm() -> Element {
    let form = use_form(|| empty_form::<Contact>(FormSpec::default()));
    form.render_fragment()
}

/// The attribute list of the `<input>` named `name`, cut at the first `>` so a
/// neighbouring field's `class="required"` marker can't leak into the match.
fn input_tag(html: &str, name: &str) -> String {
    html.split("<input")
        .map(|frag| frag.split('>').next().unwrap_or_default())
        .find(|tag| tag.contains(&format!(r#"name="{name}""#)))
        .unwrap_or_else(|| panic!("no input named {name} in:\n{html}"))
        .to_string()
}

#[gtest]
fn an_optional_structs_leaves_are_not_marked_required() {
    // `required` is per-input in HTML5, so it cannot express the all-or-nothing
    // rule an optional struct follows. Marking `address.*` would let the browser
    // block a deliberately blank address — which is a legal value here, per
    // `an_absent_optional_struct_round_trips`. So the leaves render unmarked and
    // `validate()` stays the authority; the partly-filled case is caught by
    // `a_partly_filled_optional_struct_is_an_error`, not by the browser.
    //
    // This is what `RenderCtx::optional()` buys, and the only place a member
    // changes the context instead of passing it along.
    let html = super::render_to_html(EmptyContactForm);

    expect_that!(input_tag(&html, "name"), contains_substring("required"));
    for leaf in ["address.street", "address.city", "address.zip"] {
        expect_that!(input_tag(&html, leaf), not(contains_substring("required")));
    }
}

#[gtest]
fn an_optional_structs_leaves_are_still_rendered() {
    // The pairing for `an_absent_optional_struct_still_offers_its_leaves`: not
    // marking them required must not shade into hiding them. Presence is derived
    // from what the user types, so there has to be somewhere to type.
    let html = super::render_to_html(EmptyContactForm);
    for leaf in ["address.street", "address.city", "address.zip"] {
        expect_that!(html, contains_substring(format!(r#"name="{leaf}""#)));
    }
}

// ── `optional` describes ONE member, not a subtree ───────────────────────
//
// Three flags now travel through the build walk and they behave differently on
// purpose. `FormMode` is fixed at the root and threaded down unchanged.
// `RenderCtx::required` is set once by `OptionMember` and INHERITED by every
// descendant — deliberately, because it is only a presentation hint and an
// optional struct's leaves must not be individually marked required. The build
// walk's `optional` is neither: it decides which *widget* a `bool` gets, so it
// has to be consumed by the member it describes and reset for that member's
// children.
//
// These two tests pin both directions, so a fix can't be "stop setting it".

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venue {
    open: bool,
}

/// One model carrying all three cases at once, so a single render distinguishes
/// them: a plain `bool`, a plain `bool` whose *ancestor* is optional, and a
/// `bool` that is itself optional.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Event {
    published: bool,
    venue: Option<Venue>,
    subscribed: Option<bool>,
}

#[component]
fn BooleanKindsForm() -> Element {
    let form = use_form(|| empty_form::<Event>(FormSpec::default()));
    form.render_fragment()
}

#[gtest]
fn a_plain_bool_inside_an_optional_struct_still_renders_a_checkbox() {
    // `Event.venue` is optional; `Venue.open` is a plain `bool` that can never
    // be absent, so it must render exactly as `published` does. Leaking
    // `optional` down into the struct's fields would offer "No Value" for a
    // state the type cannot hold.
    //
    // Counted rather than matched by `name`, so this stays honest about what it
    // checks: it is the number of checkboxes that is wrong when `optional`
    // leaks, and that holds whatever the surrounding markup does.
    let html = super::render_to_html(BooleanKindsForm);
    expect_that!(
        html.matches(r#"type="checkbox""#).count(),
        eq(2),
        "`published` and `venue.open` are both plain bools, so both are checkboxes:\n{html}"
    );
}

#[gtest]
fn an_optional_bool_renders_a_tri_state_widget_instead() {
    // The other direction. A checkbox has two states and `Option<bool>` has
    // three, so `subscribed` is the one bool here that genuinely needs the
    // select — which is why the flag can't simply be dropped.
    let html = super::render_to_html(BooleanKindsForm);
    expect_that!(
        html.matches("<select").count(),
        eq(1),
        "only `subscribed` is an Option<bool>:\n{html}"
    );
}

// ── The tri-state select ─────────────────────────────────────────────────

/// A single `Option<bool>` and nothing else, so `only_listener("change")` is
/// unambiguous — `BooleanKindsForm`'s checkboxes register `change` too.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Prefs {
    subscribed: Option<bool>,
}

#[component]
fn PrefsForm() -> Element {
    let form = use_form(|| empty_form::<Prefs>(FormSpec::default()));
    form.render_fragment()
}

#[gtest]
fn an_optional_bool_offers_all_three_of_its_states() {
    // A checkbox can express two of the three, which is the whole reason this
    // widget exists. "Absent" is a real answer here, so it is a selectable
    // option rather than the unselectable placeholder a required select gets.
    let html = super::render_to_html(PrefsForm);
    expect_that!(
        html,
        contains_substring(format!(
            r#"<option value="" selected=true>{ABSENT_DISPLAY}</option>"#
        ))
    );
    expect_that!(
        html,
        contains_substring(r#"<option value="true">True</option>"#)
    );
    expect_that!(
        html,
        contains_substring(r#"<option value="false">False</option>"#)
    );
    // It IS a leaf, unlike a variant picker, so it has to be findable by name.
    expect_that!(html, contains_substring(r#"name="subscribed""#));
}

#[gtest]
fn choosing_a_value_and_choosing_none_both_round_trip_through_the_dom() {
    let mut app = Harness::mount(PrefsForm);
    let select = app.only_listener("change");

    app.fire("change", select, "true");
    expect_that!(
        app.html(),
        contains_substring(r#"<option value="true" selected=true>True</option>"#)
    );

    // Back to absent. The option's value is `""`, so this takes the same write
    // path as any other choice — there is no "clear" branch to get wrong.
    app.fire("change", select, "");
    expect_that!(
        app.html(),
        contains_substring(format!(
            r#"<option value="" selected=true>{ABSENT_DISPLAY}</option>"#
        ))
    );
    expect_that!(
        app.html(),
        not(contains_substring(r#"value="true" selected=true"#))
    );
}

#[gtest]
fn a_choices_value_has_to_be_what_parse_scalar_expects() {
    // Pins the other end of the contract `SelectChoice::value` has to meet: what
    // `parse_scalar` actually accepts for an `Option<bool>`. This does NOT check
    // that `bool_choices()` matches — it feeds raw strings directly, so
    // `SelectChoice::new("True", ..)` would still pass here and fail in the two
    // render tests above. The pair is what covers it: this says what the raw
    // strings must be, those say the widget emits them.
    for (raw, expected) in [("true", Some(true)), ("false", Some(false)), ("", None)] {
        let mut form = empty_form::<Prefs>(FormSpec::default());
        form.apply_form_values(&[("subscribed".to_string(), raw.to_string())]);
        expect_that!(
            form.validate(),
            some(eq(&Prefs {
                subscribed: expected
            })),
            "raw {raw:?} should validate as {expected:?}"
        );
    }
}

// ── An unticked box is an answer, not an absence ─────────────────────────
//
// `""` means "unfilled" everywhere else, but a checkbox posts nothing when it is
// unticked and `false` is a complete answer. The obvious fix — read `""` as
// `"false"` in `apply_leaves` — was TRIED and reverted: it makes the field
// permanently present, and since `FieldSet::is_present` is `any` over its
// members, an `Option<Struct>` whose only field is a checkbox then builds
// `Some(..)` for a section the user never opened. The second test below is that
// bug, and it was red against exactly that version.
//
// So `Empty` keeps one meaning everywhere and the two CONSUMERS know about
// checkboxes instead: `validate` doesn't call one required, and
// `write_value_into` synthesises `false` at the last moment. Both go through
// `FormField::is_unticked_checkbox`, so the rule is stated once.
//
// The two directions pull against each other, which is why both are pinned: the
// first says a required `bool` must still SUBMIT, the second says that must not
// cost an absent ancestor its absence.

/// A required `bool` beside a field the user actually fills, so this reproduces
/// the reported symptom rather than a `bool` in isolation: a correctly completed
/// form that refuses to submit because one box was left alone.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
    subscribed: bool,
}

#[gtest]
fn an_untouched_checkbox_submits_as_false() {
    let mut form = empty_form::<Signup>(FormSpec::default());

    // The real submit path — `leaves()` out, edits in, `apply()` back — because
    // that is where the `""` came from. Feeding `("subscribed", "false")`
    // straight in would test the parse and skip the bug entirely.
    let mut values: HashMap<String, String> = form.leaves().into_iter().collect();
    expect_that!(
        values.get("subscribed"),
        some(eq("")),
        "an untouched checkbox stays empty the whole way through — nothing on \
         the leaf path reinterprets it, and `false` appears only at write time"
    );
    values.insert("email".to_string(), "ada@example.com".to_string());

    form.apply(&values);
    expect_that!(
        form.validate(),
        some(eq(&Signup {
            email: "ada@example.com".to_string(),
            subscribed: false,
        }))
    );
    expect_that!(form.has_errors(), eq(false));
}

#[gtest]
fn an_untouched_bool_inside_an_optional_struct_leaves_it_absent() {
    // The other direction, and the trap. `venue.open` stays `Empty` through
    // `apply`, so `venue` reports itself absent and builds `None` — a user who
    // never opened the venue section doesn't silently create one. Synthesise
    // `false` any earlier than `write_value_into` and this goes red: `open`
    // becomes present, `FieldSet::is_present` is `any` over its members, and out
    // comes `Some(Venue { open: false })`.
    //
    // `published` proves the same run still submits its top-level `bool`, so a
    // "fix" that just stopped synthesising can't pass this on its own.
    //
    // `subscribed` is the third case: an `Option<bool>` renders a tri-state
    // select whose blank really is absence, so `Empty` must reach `None`
    // untouched. `is_unticked_checkbox` excludes `optional: true` for this.
    let mut form = empty_form::<Event>(FormSpec::default());
    let values: HashMap<String, String> = form.leaves().into_iter().collect();

    form.apply(&values);
    expect_that!(
        form.validate(),
        some(eq(&Event {
            published: false,
            venue: None,
            subscribed: None,
        }))
    );
}
