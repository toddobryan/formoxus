//! `FormSpec` overrides reaching the built tree, which the shape walk applies
//! as it builds each node.
//!
//! These go through `render_to_html` rather than inspecting nodes, because the
//! question is whether an override *arrives* — and the DOM is the only place that
//! answer is unambiguous. Nothing here constructs a spec through `form!`; the
//! builder is the contract between the macro and the tree, so testing the builder
//! keeps these honest about which half is being exercised.

use super::models::{EventForCreate, Location, Shape};
use super::{render_to_html, without_form_id};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::field_kind::field_for;
use formoxus::nodes::Edit;
use formoxus::nodes::ValuesByPath;
use formoxus::path::NodePath;
use formoxus::prelude::*;
use formoxus::widgets::{CheckboxInput, Select, TextInput, Textarea};
use googletest::prelude::*;
use std::collections::BTreeSet;

/// A `bool` and an `Option<bool>` side by side — the two shapes whose *derived*
/// widgets differ, which is what makes them the useful subjects for testing
/// precedence. A plain `bool` derives a checkbox, an `Option<bool>` a tri-state
/// select.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Flags {
    enabled: bool,
    subscribed: Option<bool>,
}

// ── Labels ───────────────────────────────────────────────────────────────

#[component]
fn RelabelledField() -> Element {
    let form = use_form(|| {
        empty_form(
            FormSpec::<EventForCreate>::default()
                .with_label(&path!(EventForCreate.title), "Event name"),
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_label_override_beats_the_humanized_field_name() {
    // `default_label` would make this "Title"; the spec has to win.
    let html = render_to_html(RelabelledField);
    expect_that!(html, contains_substring("Event name"));
    expect_that!(html, not(contains_substring(">Title<")));
}

#[component]
fn RelabelledNestedField() -> Element {
    let form = use_form(|| {
        empty_form(
            FormSpec::<EventForCreate>::default()
                .with_label(&path!(EventForCreate.location.city), "Town"),
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_nested_path_reaches_exactly_one_field() {
    // The case that fails outright if a container doesn't recurse, and smears if
    // it recurses with the wrong prefix. Asserting the siblings kept their
    // defaults is what separates those two failures from success.
    let html = render_to_html(RelabelledNestedField);
    expect_that!(html, contains_substring("Town"));
    expect_that!(html, contains_substring("Street"));
    expect_that!(html, contains_substring("Zip"));
    expect_that!(html, not(contains_substring(">City<")));
}

#[component]
fn RelabelledStructNode() -> Element {
    let form = use_form(|| {
        empty_form(
            FormSpec::<EventForCreate>::default()
                .with_label(&path!(EventForCreate.location), "Where"),
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_structs_own_label_becomes_its_legend() {
    // A container is addressable in its own right, not just as a prefix — so
    // the walk has to look the container itself up, not only its leaves.
    let html = render_to_html(RelabelledStructNode);
    expect_that!(
        html,
        contains_substring(r#"<legend class="fx-legend fx-field-label">Where</legend>"#)
    );
}

// ── Widgets ─────────────────────────────────────────────────────────────

#[component]
fn UntouchedFlags() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Flags>::default()));
    form.render(using_fns! {})
}

#[gtest]
fn the_derived_widgets_are_what_the_override_has_to_beat() {
    // Baseline, so the two tests below are measuring a change rather than a
    // coincidence: `enabled` is a checkbox and `subscribed` a select.
    let html = render_to_html(UntouchedFlags);
    expect_that!(html.matches(r#"type="checkbox""#).count(), eq(1));
    expect_that!(html.matches("<select").count(), eq(1));
}

#[component]
fn OptionalBoolForcedToCheckbox() -> Element {
    let form = use_form(|| {
        empty_form(
            FormSpec::<Flags>::default().with_widget(&path!(Flags.subscribed?), CheckboxInput),
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn an_override_beats_a_derived_widget() {
    // `Option<bool>` derives `Select`. Forcing a checkbox is the strongest form
    // of the precedence rule: the override has to displace a widget the shape
    // actively chose, not merely fill a gap.
    //
    // (Whether this is a GOOD idea is another matter — a checkbox can't express
    // the third state, so `None` becomes unreachable. The point is that the spec
    // is obeyed, which is what makes the author responsible for the choice.)
    let html = render_to_html(OptionalBoolForcedToCheckbox);
    expect_that!(html.matches(r#"type="checkbox""#).count(), eq(2));
    expect_that!(html.matches("<select").count(), eq(0));
}

#[component]
fn BoolForcedToSelect() -> Element {
    let form = use_form(|| {
        empty_form(FormSpec::<Flags>::default().with_widget(&path!(Flags.enabled), Select))
    });
    form.render(using_fns! {})
}

#[gtest]
fn an_override_works_in_the_other_direction_too() {
    let html = render_to_html(BoolForcedToSelect);
    expect_that!(html.matches("<select").count(), eq(2));
    // A bool's true/false reach a `Select` with no choices of its own.
    expect_that!(html, contains_substring(r#"value="true""#));
    expect_that!(html.matches(r#"type="checkbox""#).count(), eq(0));
}

// ── `with_widget`: a widget type set on a field ─────────────────────────
//
// Naming the widget a field would have chosen anyway must change nothing, so
// the pairs below expect the same markup as the untouched form.

/// Each render mints its own form id, so the comparisons ignore it.
fn rendered(app: fn() -> Element) -> String {
    without_form_id(&render_to_html(app))
}

#[component]
fn TitleThroughTextInput() -> Element {
    let form = use_form(|| {
        empty_form(
            FormSpec::<EventForCreate>::default()
                .with_widget(&path!(EventForCreate.title), TextInput),
        )
    });
    form.render(using_fns! {})
}

#[component]
fn UntouchedEvent() -> Element {
    let form = use_form(|| empty_form(FormSpec::<EventForCreate>::default()));
    form.render(using_fns! {})
}

#[gtest]
fn a_widget_type_renders_what_the_derived_widget_renders() {
    expect_that!(
        rendered(TitleThroughTextInput),
        eq(&rendered(UntouchedEvent))
    );
}

#[component]
fn TitleThroughTextarea() -> Element {
    let form = use_form(|| {
        empty_form(
            FormSpec::<EventForCreate>::default()
                .with_widget(&path!(EventForCreate.title), Textarea),
        )
    });
    form.render(using_fns! {})
}

/// An override, not just the default by another route.
#[gtest]
fn a_widget_type_overrides_the_default() {
    let html = rendered(TitleThroughTextarea);
    expect_that!(html, contains_substring("<textarea"));
    // In place of the title's text input, not beside it.
    let text_inputs = |h: &str| h.matches(r#"type="text""#).count();
    expect_that!(
        text_inputs(&html),
        eq(text_inputs(&rendered(UntouchedEvent)) - 1)
    );
}

#[component]
fn EnabledThroughCheckboxInput() -> Element {
    let form = use_form(|| {
        empty_form(FormSpec::<Flags>::default().with_widget(&path!(Flags.enabled), CheckboxInput))
    });
    form.render(using_fns! {})
}

/// Same markup as the derived checkbox, so in particular no `required` on
/// it: `FieldNode::is_checkbox` reads the widget type's name.
#[gtest]
fn a_checkbox_widget_type_renders_as_the_derived_checkbox() {
    expect_that!(
        rendered(EnabledThroughCheckboxInput),
        eq(&rendered(UntouchedFlags))
    );
}

/// An unticked box is a complete `false`, not a missing value, for a
/// checkbox set through `with_widget` as for the derived one.
#[gtest]
fn an_unticked_checkbox_widget_type_validates() {
    let mut form =
        empty_form(FormSpec::<Flags>::default().with_widget(&path!(Flags.enabled), CheckboxInput));
    expect_that!(
        form.validate(),
        some(eq(&Flags {
            enabled: false,
            subscribed: None,
        }))
    );
}

#[gtest]
#[should_panic(expected = "location is a struct")]
fn a_widget_type_on_a_struct_is_rejected() {
    // A struct has no single widget to be, and silently ignoring the entry
    // would leave the author with no thread to pull. The message names the path
    // because a nested form has more than one candidate.
    let _ = empty_form(
        FormSpec::<EventForCreate>::default()
            .with_widget(&path!(EventForCreate.location), Textarea),
    );
}

#[gtest]
#[should_panic(expected = "venues is a list")]
fn a_widget_type_on_a_list_is_rejected() {
    let _ =
        empty_form(FormSpec::<WithRows>::default().with_widget(&path!(WithRows.venues), Select));
}

// ── Rejections and no-ops ────────────────────────────────────────────────

// A path that names no node is refused when the spec is built. It used to be
// stored and never looked up, so a typo in a string path did nothing at all
// (issue #15). Since the builders take a `ModelPath` (PATHS_PLAN decision H),
// neither `path!` nor `form!` can write one: both check their paths at compile
// time (`tests/ui`). `spec_key_shape` stays as a backstop, so these pin its
// messages through the hidden `ModelPath::__new`, which is what a bug in
// either macro would hand it.

#[gtest]
#[should_panic(
    expected = "nowhere.at.all: names nothing in a form over `EventForCreate`: \
                           `EventForCreate` has no field `nowhere` (its fields: title, location)"
)]
fn an_unknown_field_is_refused() {
    let _ = FormSpec::<EventForCreate>::default()
        .with_label(&ModelPath::__new("nowhere.at.all"), "Ignored");
}

#[gtest]
#[should_panic(expected = "`Location` has no field `ctiy` (its fields: street, city, zip)")]
fn an_unknown_nested_field_is_refused() {
    let _ = FormSpec::<EventForCreate>::default()
        .with_label(&ModelPath::__new("location.ctiy"), "Town");
}

#[gtest]
#[should_panic(expected = "`#0` is one item's key — a spec names every item at once, with `[]`")]
fn one_items_key_is_refused() {
    let _ = FormSpec::<WithRows>::default().with_label(&ModelPath::__new("venues.#0.city"), "Town");
}

/// **An index names one item, and a spec names every item at once,** so a
/// builder refuses it (decision I) and says what to write instead.
#[gtest]
#[should_panic(
    expected = "venues[0].city: names one item, but a spec names every item at once; \
                write `venues[].city`"
)]
fn an_index_is_refused() {
    let _ = FormSpec::<WithRows>::default().with_label(&path!(WithRows.venues[0].city), "Town");
}

/// **So does a value,** which names one item of a set.
#[gtest]
#[should_panic(
    expected = "labels[=urgent]: names one item, but a spec names every item at once; \
                write `labels[]`"
)]
fn a_value_is_refused() {
    let _ = FormSpec::<Lists>::default().with_label(&path!(Lists.labels[= "urgent"]), "Label");
}

#[gtest]
#[should_panic(expected = "`title` is a `String`, not a collection, so `[]` names nothing")]
fn brackets_on_a_scalar_are_refused() {
    let _ = FormSpec::<EventForCreate>::default().with_label(&ModelPath::__new("title[]"), "Title");
}

/// A set's items are named with `[]` as a list's are, so a spec can give
/// them a label or a widget (box 9).
#[gtest]
fn brackets_on_a_set_name_its_items() {
    let _ = FormSpec::<Lists>::default().with_label(&path!(Lists.labels[]), "Label");
}

#[gtest]
fn every_kind_of_node_is_accepted() {
    // A leaf, a struct, a field in it, a list, its items, an item's field: each
    // names a node, so none of these panics.
    let _ = FormSpec::<EventForCreate>::default()
        .with_label(&path!(EventForCreate.title), "Headline")
        .with_label(&path!(EventForCreate.location), "Where")
        .with_label(&path!(EventForCreate.location.city), "Town");
    let _ = FormSpec::<WithRows>::default()
        .with_label(&path!(WithRows.venues), "Venues")
        .with_label(&path!(WithRows.venues[]), "Venue")
        .with_label(&path!(WithRows.venues[].city), "Town");
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct WithRows {
    venues: Vec<Location>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct WithShape {
    shape: Shape,
    maybe: Option<Shape>,
}

#[gtest]
fn a_variants_field_is_accepted_through_an_option_too() {
    let _ = FormSpec::<WithShape>::default()
        .with_label(&path!(WithShape.shape.$Shape::Circle.radius), "R")
        .with_label(&path!(WithShape.maybe?.$Shape::Rectangle.width), "W");
}

#[gtest]
#[should_panic(expected = "`Shape` has no variant `Circel` (its variants: Circle, Rectangle)")]
fn an_unknown_variant_is_refused() {
    let _ =
        FormSpec::<WithShape>::default().with_label(&ModelPath::__new("shape.$Circel.radius"), "R");
}

#[gtest]
#[should_panic(expected = "variant `Circle` has no field `width` (its fields: radius)")]
fn a_field_of_another_variant_is_refused() {
    let _ =
        FormSpec::<WithShape>::default().with_label(&ModelPath::__new("shape.$Circle.width"), "W");
}

#[gtest]
#[should_panic(expected = "`shape.$Circle` is a variant, which has no node of its own")]
fn a_path_ending_at_a_variant_is_refused() {
    let _ = FormSpec::<WithShape>::default().with_label(&ModelPath::__new("shape.$Circle"), "C");
}

#[gtest]
#[should_panic(
    expected = "`shape` is an enum, so a field inside it is reached through its \
                           variant: `shape.$Variant.radius`"
)]
fn a_variants_field_without_the_variant_is_refused() {
    let _ = FormSpec::<WithShape>::default().with_label(&ModelPath::__new("shape.radius"), "R");
}

// ── `adder` — only on a list ────────────────────────────────────────────
//
// `form!` refuses it on a non-list at compile time (`tests/ui`); these are the
// hand-built spec, which can only be refused when it is built.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Lists {
    tags: Vec<String>,
    grid: Vec<Vec<String>>,
    maybe: Option<Vec<String>>,
    venues: Vec<Location>,
    title: String,
    labels: BTreeSet<String>,
}

fn adder() -> formoxus::widgets::ItemAdder {
    formoxus::widgets::ItemAdder::default()
}

/// A list, the inner lists of a list of lists, and a list behind an `Option`,
/// whose key has no `?`. Whether each one renders its adder is the render
/// tests' question; this is only that none is refused.
#[gtest]
fn an_adder_is_accepted_on_every_kind_of_list() {
    let _ = FormSpec::<Lists>::default()
        .with_adder(&path!(Lists.tags), adder())
        .with_adder(&path!(Lists.grid), adder())
        .with_adder(&path!(Lists.grid[]), adder())
        .with_adder(&path!(Lists.maybe?), adder());
}

#[gtest]
#[should_panic(expected = "title: `adder` applies only to list or set, not `String`")]
fn an_adder_on_a_scalar_is_refused() {
    let _ = FormSpec::<Lists>::default().with_adder(&path!(Lists.title), adder());
}

/// `tags[]` is every ITEM of `tags`, each a `String`: the mistake decision A
/// in `ORDERED_LISTS_PLAN.md` warns about, since a label for the items does
/// go there.
#[gtest]
#[should_panic(expected = "tags[]: `adder` applies only to list or set, not `String`")]
fn an_adder_on_a_lists_items_is_refused() {
    let _ = FormSpec::<Lists>::default().with_adder(&path!(Lists.tags[]), adder());
}

#[gtest]
#[should_panic(expected = "venues[]: `adder` applies only to list or set, not `Location`")]
fn an_adder_on_a_struct_item_is_refused() {
    let _ = FormSpec::<Lists>::default().with_adder(&path!(Lists.venues[]), adder());
}

/// The path is checked before the list is, so a typo reads as a typo.
#[gtest]
#[should_panic(expected = "tagz: names nothing in a form over `Lists`")]
fn an_adder_on_an_unknown_path_says_so_first() {
    let _ = FormSpec::<Lists>::default().with_adder(&ModelPath::__new("tagz"), adder());
}

// ── `treat_as_set` and `with_sort_by` ────────────────────────────────────
//
// `form!`'s `set` and `sort_by` are refused at compile time (`tests/ui`);
// these are the hand-built spec, whose checks are the same, made when it is
// built, and named by path.

fn by_bytes() -> formoxus::nodes::SetOrder {
    formoxus::nodes::SetOrder::custom(String::cmp, "bytes")
}

/// A list of strings, the inner lists of a list of lists, and a list behind
/// an `Option`, whose key has no `?`.
#[gtest]
fn treat_as_set_is_accepted_on_every_list_of_leaves() {
    let _ = FormSpec::<Lists>::default()
        .treat_as_set(&path!(Lists.tags))
        .treat_as_set(&path!(Lists.grid[]))
        .treat_as_set(&path!(Lists.maybe?));
}

#[gtest]
#[should_panic(expected = "labels is already a set; don't call `treat_as_set`")]
fn treat_as_set_on_a_set_is_refused() {
    let _ = FormSpec::<Lists>::default().treat_as_set(&path!(Lists.labels));
}

#[gtest]
#[should_panic(expected = "title: `treat_as_set` applies only to a list, not `String`")]
fn treat_as_set_on_a_scalar_is_refused() {
    let _ = FormSpec::<Lists>::default().treat_as_set(&path!(Lists.title));
}

/// A set's items are shown, not edited, so each must be one value.
#[gtest]
#[should_panic(expected = "venues: `treat_as_set` applies only to a list of single values")]
fn treat_as_set_on_a_list_of_structs_is_refused() {
    let _ = FormSpec::<Lists>::default().treat_as_set(&path!(Lists.venues));
}

/// The outer list's items are lists, which are not single values either.
#[gtest]
#[should_panic(expected = "grid: `treat_as_set` applies only to a list of single values")]
fn treat_as_set_on_a_list_of_lists_is_refused() {
    let _ = FormSpec::<Lists>::default().treat_as_set(&path!(Lists.grid));
}

#[gtest]
fn sort_by_is_accepted_on_a_set_and_on_a_list_treated_as_one() {
    let _ = FormSpec::<Lists>::default()
        .with_sort_by(&path!(Lists.labels), by_bytes())
        .treat_as_set(&path!(Lists.tags))
        .with_sort_by(&path!(Lists.tags), by_bytes());
}

/// `form!` emits `treat_as_set` before `with_sort_by`; a hand-built spec
/// must too (decision B in box 5).
#[gtest]
#[should_panic(
    expected = "tags: if you want to treat this list as a set, call `treat_as_set` first"
)]
fn sort_by_on_a_list_not_yet_treated_as_a_set_is_refused() {
    let _ = FormSpec::<Lists>::default()
        .with_sort_by(&path!(Lists.tags), by_bytes())
        .treat_as_set(&path!(Lists.tags));
}

#[gtest]
#[should_panic(expected = "title: `sort_by` applies only to a set, not `String`")]
fn sort_by_on_a_scalar_is_refused() {
    let _ = FormSpec::<Lists>::default().with_sort_by(&path!(Lists.title), by_bytes());
}

// ── `[]` — selecting every item of a list ─────────────────────────────────
//
// All of these go through `form_for` rather than `empty_form`: item count isn't in
// the shape, so a blank form has no items to select and the assertions would hold
// vacuously.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<String>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    venues: Vec<Location>,
}

fn quiz() -> Quiz {
    Quiz {
        answers: vec!["PNG".to_string(), "JPEG".to_string(), "GIF".to_string()],
    }
}

fn trip() -> Trip {
    Trip {
        venues: vec![
            Location {
                street: "1 A St".to_string(),
                city: "Springfield".to_string(),
                zip: "11111".to_string(),
            },
            Location {
                street: "2 B St".to_string(),
                city: "Shelbyville".to_string(),
                zip: "22222".to_string(),
            },
        ],
    }
}

#[component]
fn RowsWithLabels() -> Element {
    let form = use_form(|| {
        form_for(
            &quiz(),
            FormSpec::<Quiz>::default().with_label(&path!(Quiz.answers[]), "Answer"),
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_bracket_selector_reaches_every_item() {
    // Items normally get NO label — `default_label` returns `None` for a `#`-keyed
    // name — so every occurrence here is one the selector put there. Counting is
    // what distinguishes "reached every item" from "reached the first one".
    //
    // Delimited rather than bare: the list's OWN derived legend is "Answers",
    // which contains "Answer", so a substring count would find four.
    let html = render_to_html(RowsWithLabels);
    expect_that!(html.matches(">Answer<").count(), eq(3));
}

#[component]
fn RowFieldsWithLabels() -> Element {
    let form = use_form(|| {
        form_for(
            &trip(),
            FormSpec::<Trip>::default().with_label(&path!(Trip.venues[].city), "Town"),
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_bracket_selector_composes_with_a_field_below_it() {
    // The case `[]` exists for. Two items, so two "Town"s — and the siblings must
    // keep their derived labels, which is what separates a correct substitution
    // from one that smears across the whole item.
    let html = render_to_html(RowFieldsWithLabels);
    expect_that!(html.matches("Town").count(), eq(2));
    expect_that!(html.matches("Street").count(), eq(2));
    expect_that!(html, not(contains_substring(">City<")));
}

#[component]
fn ListWithItsOwnLabel() -> Element {
    let form = use_form(|| {
        form_for(
            &trip(),
            FormSpec::<Trip>::default().with_label(&path!(Trip.venues), "Where we went"),
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_bare_list_path_labels_the_list_not_the_items() {
    // The distinction the bracket buys: without it, `venues` is the `ListNode`
    // itself, so exactly one legend and nothing per item.
    let html = render_to_html(ListWithItsOwnLabel);
    expect_that!(html.matches("Where we went").count(), eq(1));
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Answers {
    correct: Vec<bool>,
}

#[component]
fn RowWidgetsOverridden() -> Element {
    let form = use_form(|| {
        form_for(
            &Answers {
                correct: vec![true, false],
            },
            FormSpec::<Answers>::default().with_widget(&path!(Answers.correct[]), Select),
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_bracket_selector_can_set_each_items_widget() {
    // A `Vec<bool>` derives a checkbox per item; the selector replaces all of
    // them. This is the half Todd's reading of `ListNode` predicted: a list has no
    // widget of its own, but it has N items that each do.
    let html = render_to_html(RowWidgetsOverridden);
    expect_that!(html.matches("<select").count(), eq(2));
    expect_that!(html.matches(r#"type="checkbox""#).count(), eq(0));
}

#[gtest]
#[should_panic(expected = "write `venues[]` to give every ITEM a widget")]
fn a_widget_on_the_list_itself_now_suggests_the_bracket() {
    // The rejection still stands, but it can name the fix now.
    let _ = form_for(
        &trip(),
        FormSpec::<Trip>::default().with_widget(&path!(Trip.venues), Select),
    );
}

// ── Nodes that arrive AFTER the spec was applied ───────────────────────

#[component]
fn RowAddedAfterTheSpec() -> Element {
    // The edit runs on the state before `use_form` takes it, which is the same
    // funnel a button click goes through — `FormState::edit`.
    let form = use_form(|| {
        let mut state = form_for(
            &quiz(),
            FormSpec::<Quiz>::default().with_label(&path!(Quiz.answers[]), "Answer"),
        );
        state
            .edit(&Edit::AddItem {
                path: NodePath::new("answers"),
                before: None,
                value: None,
            })
            .expect("appending to `answers` should succeed");
        state
    });
    form.render(using_fns! {})
}

#[gtest]
fn an_item_added_after_mount_still_gets_the_spec() {
    // `add_item` builds the new item from the shape alone, so without re-applying
    // it would come up with a derived label — none, for a `#`-keyed item — while
    // the three original items carry "Answer". Four items, four labels.
    //
    // This is the case that makes re-application load-bearing, and it only became
    // reachable when `[]` made an item's fields addressable at all.
    let html = render_to_html(RowAddedAfterTheSpec);
    expect_that!(html.matches(">Answer<").count(), eq(4));
}

#[component]
fn RowFieldAddedAfterTheSpec() -> Element {
    let form = use_form(|| {
        let mut state = form_for(
            &trip(),
            FormSpec::<Trip>::default().with_label(&path!(Trip.venues[].city), "Town"),
        );
        state
            .edit(&Edit::AddItem {
                path: NodePath::new("venues"),
                before: None,
                value: None,
            })
            .expect("appending to `venues` should succeed");
        state
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_new_items_nested_field_gets_the_spec_too() {
    // One level deeper: the new item is a whole `StructNode`, and the selector has to
    // reach inside it. Three "Town"s for two original items plus the new one.
    let html = render_to_html(RowFieldAddedAfterTheSpec);
    expect_that!(html.matches("Town").count(), eq(3));
}

// ── Constraints ──────────────────────────────────────────────────────────

fn full_event(title: &str) -> ValuesByPath {
    [
        ("title", title),
        ("location.street", "1 Main"),
        ("location.city", "Springfield"),
        ("location.zip", "12345"),
    ]
    .iter()
    .map(|(k, v)| (NodePath::new(k.to_string()), v.to_string()))
    .collect()
}

fn failing_paths(errors: &FormErrors) -> Vec<String> {
    errors.fields.iter().map(|(p, _)| p.to_string()).collect()
}

/// The whole spec -> field -> check chain, through `Submission` rather than the
/// DOM: a constraint has no markup of its own yet, so validation is the only
/// place its arrival shows.
///
/// Every other constraint test sets constraints straight onto a field, which
/// means this is the only one that would notice the walk dropping a spec's
/// settings on the floor. A validated setting is the field's, so it goes
/// through `with_node`, as `form!` emits it.
#[gtest]
fn a_constraint_from_the_spec_reaches_validation() {
    let spec = || {
        FormSpec::<EventForCreate>::default().with_node(
            &path!(EventForCreate.title),
            field_for(|m: &EventForCreate| &m.title, |f| f.max_length = Some(5)),
        )
    };

    expect_that!(
        Submission::accept(spec(), &full_event("Short")).is_ok(),
        eq(true),
        "a title inside the stated bound is accepted"
    );

    let errors = Submission::accept(spec(), &full_event("Much too long"))
        .expect_err("the title is over the stated maximum");
    expect_that!(failing_paths(&errors), elements_are![eq("title")]);
}

/// The walk looks a node up by its QUALIFIED path, so a constraint on a nested
/// leaf has to survive `qualify(prefix, name)`. The flat case above would pass
/// even if the prefix were being dropped.
#[gtest]
fn a_constraint_reaches_a_nested_leaf() {
    let spec = FormSpec::<EventForCreate>::default().with_node(
        &path!(EventForCreate.location.zip),
        field_for(
            |m: &EventForCreate| &m.location.zip,
            |f| {
                f.pattern = Some(r"\d{5}");
            },
        ),
    );

    let mut values = full_event("Fine");
    values.insert(NodePath::new("location.zip"), "NW1".to_string());

    let errors = Submission::accept(spec, &values).expect_err("NW1 is not five digits");
    expect_that!(failing_paths(&errors), elements_are![eq("location.zip")]);
}

/// A validated key in `with_attrs` would render and never be checked, so the
/// browser and the server would disagree; it is refused instead. The message
/// names the key as `form!` spells it (`max_length`), not as HTML does
/// (`maxlength`), since that is what the dev wrote and what the setting is
/// called.
#[gtest]
#[should_panic(expected = "title: `max_length` is validated, so it is a setting of the field")]
fn a_validated_key_in_with_attrs_is_refused_by_its_form_name() {
    let _ = FormSpec::<EventForCreate>::default().with_attrs(
        &path!(EventForCreate.title),
        formoxus::field_attrs::FieldAttrs::from([(
            formoxus::attrs::AttrKey::Std(formoxus::attrs::Attr::MaxLength),
            formoxus::attrs::AttrValue::Int(5),
        )]),
    );
}
