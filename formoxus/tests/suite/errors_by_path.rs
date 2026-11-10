//! Where a field's errors live, and when they leave.
//!
//! Errors are held by path in `FormState::field_errors`, apart from the nodes.
//! That map outlives whatever a node used to clear for itself, so these pin
//! each moment an error has to go, and the paths it is kept under.
//!
//! A set's draft is the special case. A refusal on it is in `field_errors`, for
//! its own widget to show, and nowhere else: it is not part of what would be
//! submitted, so `collect_errors`, `has_errors` and the wire never see it.

use super::models::Shape;
use super::render_to_html;
use dioxus::prelude::*;
use facet::Facet;
use formoxus::nodes::{Edit, ValuesByPath};
use formoxus::path::NodePath;
use formoxus::prelude::*;
use googletest::prelude::*;
use std::collections::BTreeSet;

fn values(pairs: &[(&str, &str)]) -> ValuesByPath {
    pairs
        .iter()
        .map(|(k, v)| (NodePath::new(k.to_string()), v.to_string()))
        .collect()
}

fn messages_at<T>(form: &FormState<T>, path: &str) -> Vec<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    form.collect_errors()
        .fields
        .get(path)
        .map(|found| found.iter().map(|m| m.0.clone()).collect())
        .unwrap_or_default()
}

// ── An enum's own error ──────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Drawing {
    name: String,
    shape: Shape,
}

fn choose(variant: Option<&str>) -> Edit {
    Edit::ChooseVariant {
        path: NodePath::new("shape"),
        variant: variant.map(ToString::to_string),
    }
}

/// The enum's path still exists after a choice, so pruning by path cannot do
/// this: the edit has to remove the error itself.
#[gtest]
fn choosing_a_variant_clears_must_choose() {
    let mut form = empty_form::<Drawing>(FormSpec::default());
    expect_that!(form.validate(), none());
    expect_that!(
        messages_at(&form, "shape"),
        elements_are![eq("You must choose a variant for this field.")]
    );

    expect_that!(form.edit(&choose(Some("Circle"))), ok(none()));

    expect_that!(form.collect_errors().paths(), elements_are![eq("name")]);
}

/// Back to `--none--` is a choice too. Whether unchosen is wrong is for the
/// next `validate` to say.
#[gtest]
fn choosing_none_clears_must_choose_until_the_next_validate() {
    let mut form = empty_form::<Drawing>(FormSpec::default());
    expect_that!(form.validate(), none());

    expect_that!(form.edit(&choose(None)), ok(none()));
    expect_that!(messages_at(&form, "shape"), is_empty());

    expect_that!(form.validate(), none());
    expect_that!(messages_at(&form, "shape"), len(eq(1)));
}

/// A live `<select>` can send a stale name. The user sees why beside it, and a
/// dev who called `edit` gets the `Err` as well.
#[gtest]
fn a_stale_variant_name_reports_at_the_enums_path_and_in_the_err() {
    let mut form = empty_form::<Drawing>(FormSpec::default());

    let refused = form.edit(&choose(Some("Triangle")));

    let refusal = refused.expect_err("Triangle is not a variant");
    expect_that!(refusal.0, contains_substring("Triangle"));
    expect_that!(
        messages_at(&form, "shape"),
        elements_are![contains_substring("Triangle")]
    );
}

#[gtest]
fn a_good_choice_clears_a_stale_names_error() {
    let mut form = empty_form::<Drawing>(FormSpec::default());
    form.edit(&choose(Some("Triangle")))
        .expect_err("Triangle is not a variant");

    expect_that!(form.edit(&choose(Some("Circle"))), ok(none()));

    expect_that!(messages_at(&form, "shape"), is_empty());
}

/// The second field is still checked after the first fails.
#[gtest]
fn two_bad_fields_in_one_variant_both_report() {
    let mut form = empty_form::<Drawing>(FormSpec::default());
    form.distribute_values(&values(&[("name", "Plan")]));
    form.edit(&choose(Some("Rectangle")))
        .expect("Rectangle is a variant");

    expect_that!(form.validate(), none());

    expect_that!(
        form.collect_errors().paths(),
        elements_are![eq("shape.$Rectangle.width"), eq("shape.$Rectangle.height")]
    );
}

/// An enum below the top level, so its own path is not its prefix.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Sheet {
    drawing: Drawing,
}

#[gtest]
fn a_nested_enums_error_is_at_its_own_path() {
    let mut form = empty_form::<Sheet>(FormSpec::default());

    expect_that!(form.validate(), none());

    expect_that!(
        form.collect_errors().paths(),
        elements_are![eq("drawing.name"), eq("drawing.shape")]
    );
}

// ── Paths an edit takes away ─────────────────────────────────────────────

/// Gone from the map, not only out of sight: the fields' paths exist again if
/// the variant is chosen again, and the old error would come back with them.
#[gtest]
fn a_changed_variants_field_errors_go() {
    let mut form = empty_form::<Drawing>(FormSpec::default());
    form.distribute_values(&values(&[("name", "Plan")]));
    form.edit(&choose(Some("Circle")))
        .expect("Circle is a variant");
    expect_that!(form.validate(), none());
    expect_that!(
        form.collect_errors().paths(),
        elements_are![eq("shape.$Circle.radius")]
    );

    form.edit(&choose(Some("Rectangle")))
        .expect("Rectangle is a variant");
    expect_that!(form.collect_errors().paths(), is_empty());

    form.edit(&choose(Some("Circle")))
        .expect("Circle is a variant");
    expect_that!(form.collect_errors().paths(), is_empty());
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

#[gtest]
fn a_removed_items_errors_go() {
    let post = Post {
        tags: ["go".to_string(), "rust".to_string()].into(),
    };
    let mut form = form_for(&post, FormSpec::default());
    form.push_field_error(&path!(Post.tags[= "go"]), "Retired.")
        .expect("the item exists");
    form.push_field_error(&path!(Post.tags[= "rust"]), "Also retired.")
        .expect("the item exists");

    form.edit(&Edit::RemoveItem {
        path: NodePath::new("tags"),
        index: 0,
    })
    .expect("the item exists");

    expect_that!(form.collect_errors().paths(), elements_are![eq("tags.#1")]);
    expect_that!(form.field_errors.get("tags.#0"), none());
}

// ── A set's draft ────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Ages {
    ages: BTreeSet<u32>,
}

/// Where the draft sits in [`refused_draft`]: `#0` is the one item.
const DRAFT: &str = "ages.#1";

fn add_the_draft() -> Edit {
    Edit::AddItem {
        path: NodePath::new("ages"),
        before: None,
        value: None,
    }
}

fn draft_errors(form: &FormState<Ages>) -> usize {
    form.field_errors.get(DRAFT).map_or(0, Vec::len)
}

/// A set of one item, whose draft holds text that is no number, and Add has
/// just refused it.
fn refused_draft() -> FormState<Ages> {
    let ages = Ages { ages: [30].into() };
    let mut form = form_for(&ages, FormSpec::default());
    form.distribute_values(&values(&[(DRAFT, "old")]));
    form.edit(&add_the_draft()).expect("Add refuses with an Ok");
    form
}

#[gtest]
fn a_refused_draft_keeps_its_error_for_its_widget() {
    let form = refused_draft();

    expect_that!(draft_errors(&form), eq(1));
}

#[gtest]
fn a_refused_draft_is_not_an_error_of_the_form() {
    let form = refused_draft();

    expect_that!(form.collect_errors().paths(), is_empty());
    expect_that!(form.has_errors(), eq(false));
}

#[component]
fn RefusedDraftOnTheWire() -> Element {
    let form = use_form(refused_draft);
    let paths = form.to_wire().errors().paths().join(" ");
    rsx! {
        pre { id: "wire-errors", "[{paths}]" }
    }
}

#[gtest]
fn a_refused_draft_stays_off_the_wire() {
    let html = render_to_html(RefusedDraftOnTheWire);

    expect_that!(html, contains_substring(">[]</pre>"));
}

/// An error aimed at a draft from outside has no submitted value to be about,
/// so it comes back as a path the form does not have. From a server, through
/// `distribute_errors`: `push_field_error` takes a `ModelPath`, which names a
/// set's item only by a value it holds, so it cannot name a draft at all.
#[gtest]
fn a_distributed_error_does_not_land_on_a_draft() {
    let ages = Ages { ages: [30].into() };
    let mut form = form_for(&ages, FormSpec::default());

    let unclaimed = form.distribute_errors(
        [(
            NodePath::new(DRAFT),
            vec![ValidationMessage::from("Too old.")],
        )]
        .into_iter()
        .collect(),
    );
    expect_that!(
        unclaimed.keys().map(NodePath::as_str).collect::<Vec<_>>(),
        elements_are![eq(&DRAFT)]
    );
    expect_that!(draft_errors(&form), eq(0));
}

/// `validate` never reaches an empty draft, so Add has to forget the refusal
/// before it looks.
#[gtest]
fn emptying_a_refused_draft_and_adding_clears_the_refusal() {
    let mut form = refused_draft();
    form.distribute_values(&values(&[(DRAFT, "")]));

    expect_that!(form.edit(&add_the_draft()), ok(none()));

    expect_that!(draft_errors(&form), eq(0));
}

/// Edit rebuilds the draft from the item, so what was refused is gone.
#[gtest]
fn editing_an_item_clears_a_refused_drafts_error() {
    let mut form = refused_draft();

    form.edit(&Edit::EditItem {
        path: NodePath::new("ages"),
        index: Some(0),
    })
    .expect("the item exists");

    expect_that!(draft_errors(&form), eq(0));
}

/// A submit empties the map, and the draft is not part of a submit. Add judges
/// it again.
#[gtest]
fn a_submit_clears_a_drafts_refusal_and_the_next_add_brings_it_back() {
    let mut form = refused_draft();

    expect_that!(form.validate(), some(eq(&Ages { ages: [30].into() })));
    expect_that!(draft_errors(&form), eq(0));

    expect_that!(form.edit(&add_the_draft()), ok(none()));
    expect_that!(draft_errors(&form), eq(1));
}

// ── An absent option ─────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Callback {
    name: String,
    phone: Option<String>,
}

/// `validate` writes nothing for an absent option's field, so nothing would
/// overwrite this entry: only clearing the whole map removes it.
#[gtest]
fn a_pushed_error_on_an_absent_option_is_gone_after_validate() {
    let mut form = empty_form::<Callback>(FormSpec::default());
    form.distribute_values(&values(&[("name", "Ada")]));
    form.push_field_error(&path!(Callback.phone?), "That number is taken.")
        .expect("an absent option's field still has a path");
    expect_that!(form.collect_errors().paths(), elements_are![eq("phone")]);

    expect_that!(
        form.validate(),
        some(eq(&Callback {
            name: "Ada".to_string(),
            phone: None,
        }))
    );

    expect_that!(form.collect_errors().paths(), is_empty());
}

// ── Items ────────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Poll {
    answers: Vec<String>,
}

/// A count is the list's own error, and it does not hide what is wrong with
/// the items.
#[gtest]
fn a_count_error_still_reports_its_items_errors() {
    let spec = form! { Poll { answers => { min_items: 2 } } };
    let poll = Poll {
        answers: vec!["Yes".to_string()],
    };
    let mut form = form_for(&poll, spec);
    form.distribute_values(&values(&[("answers.#0", "")]));

    expect_that!(form.validate(), none());

    expect_that!(
        form.collect_errors().paths(),
        elements_are![eq("answers"), eq("answers.#0")]
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Room {
    name: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venue {
    answers: Vec<String>,
    tags: BTreeSet<String>,
    rooms: Vec<Room>,
}

/// An item is named once: `tags.#0`, never `#0` or `tags.#0.#0`. In the order
/// the page shows them, and with a set's blank draft in none of it.
#[gtest]
fn an_items_error_is_at_its_exact_path() {
    let venue = Venue {
        answers: vec!["Yes".to_string()],
        tags: ["rust".to_string()].into(),
        rooms: vec![Room {
            name: "Hall".to_string(),
        }],
    };
    let mut form = form_for(&venue, FormSpec::default());
    form.distribute_values(&values(&[
        ("answers.#0", ""),
        ("tags.#0", ""),
        ("rooms.#0.name", ""),
    ]));

    expect_that!(form.validate(), none());

    expect_that!(
        form.collect_errors().paths(),
        elements_are![eq("answers.#0"), eq("tags.#0"), eq("rooms.#0.name")]
    );
}

#[component]
fn LiveErrors() -> Element {
    let form = use_form(|| empty_form::<Drawing>(FormSpec::default()));
    use_hook(|| form.validate());
    let paths = form.to_wire().errors().paths().join(" ");
    let has_errors = form.has_errors();
    rsx! { pre { id: "wire-errors", "[{paths}] {has_errors}" } }
}

/// `field_errors` is empty between operations on a live form, so reading it
/// here would lose every error without failing anything else.
#[gtest]
fn a_live_forms_errors_reach_the_wire_and_has_errors() {
    expect_that!(
        render_to_html(LiveErrors),
        contains_substring(">[name shape] true</pre>")
    );
}
