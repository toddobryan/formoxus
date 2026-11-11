//! `ItemValue::raw`: a set's item named by its ENTRY, the string the wire
//! carries at the item's own path (`WIDGETS_PLAN.md` decision F).
//!
//! A multi-pick widget knows a choice only as a string: a checkbox knows
//! "BOS", not `"BOS".to_string()` behind a typed `ItemValue::new`. So it adds an
//! item from that string, the way the wire rebuilds one: a leaf item's entry is
//! its text (`tags.#2` = `rust`), a unit-enum item's is its variant's name
//! (`days.#2` = `Friday`). There is no shape to check, so the parse is the
//! check, and everything after the add is what any other item gets.

use super::{store_keys, with_store_keys};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::nodes::{Edit, ItemValue};
use formoxus::path::NodePath;
use formoxus::prelude::*;
use googletest::prelude::*;
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

fn post(tags: &[&str]) -> Post {
    Post {
        tags: tags.iter().map(ToString::to_string).collect(),
    }
}

#[derive(Facet, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
#[allow(dead_code)]
enum Weekday {
    Monday,
    Wednesday,
    Friday,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Meeting {
    days: BTreeSet<Weekday>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Sizes {
    sizes: BTreeSet<u8>,
}

const CITIES: &[(&str, &str)] = &[("BOS", "Boston"), ("ORD", "Chicago")];

#[derive(Facet, Clone, Debug, PartialEq)]
struct Picked {
    cities: BTreeSet<String>,
}

/// An `AddItem` to the set at `path` naming its item by `entry`.
fn add_raw(path: &str, entry: &str) -> Edit {
    Edit::AddItem {
        path: NodePath::new(path),
        before: None,
        value: Some(ItemValue::raw(entry)),
    }
}

// ── Adding ───────────────────────────────────────────────────────────────

/// The item's key is the next one: `go` is `#0` and the set's draft `#1`, so
/// the new item is `#2`, as a typed add would make it.
#[gtest]
fn a_raw_entry_adds_a_string_item() {
    let mut form = form_for(&post(&["go"]), FormSpec::default());
    expect_that!(form.edit(&add_raw("tags", "rust")), ok(some(eq("tags.#2"))));
    expect_that!(form.validate(), some(eq(&post(&["go", "rust"]))));
}

/// A unit-enum item's entry is its variant's name, which is how a
/// `checkbox_multiple` over `BTreeSet<Weekday>` names a day.
#[gtest]
fn a_raw_entry_adds_a_unit_enum_item_by_its_variant_name() {
    let meeting = Meeting {
        days: [Weekday::Monday].into(),
    };
    let mut form = form_for(
        &meeting,
        form! { Meeting { days => { sort_by: Weekday::cmp } } },
    );
    expect_that!(form.edit(&add_raw("days", "Friday")), ok(some(anything())));
    expect_that!(
        form.validate(),
        some(eq(&Meeting {
            days: [Weekday::Monday, Weekday::Friday].into(),
        }))
    );
}

/// A raw add goes through the set's duplicate check like a typed one: refused
/// for the user, not the caller, so `Ok(None)` and nothing added.
#[gtest]
fn a_raw_duplicate_is_refused_as_a_typed_one_is() {
    let mut form = form_for(&post(&["go", "rust"]), FormSpec::default());
    expect_that!(form.edit(&add_raw("tags", "rust")), ok(none()));
    expect_that!(form.validate(), some(eq(&post(&["go", "rust"]))));
}

/// Through `Form::edit`, a raw item's entry reaches the store, as a typed
/// item's values do, so `validate` and `to_wire` see it.
#[gtest]
fn a_raw_items_entry_reaches_the_store() {
    #[component]
    fn Probe() -> Element {
        let form = use_form(|| form_for(&post(&["go"]), FormSpec::default()));
        use_hook(|| form.edit(&add_raw("tags", "rust")).expect("rust is new"));
        with_store_keys(form)
    }
    // `#0` go, `#1` the draft, `#2` rust.
    expect_that!(
        store_keys(&super::render_to_html(Probe)),
        eq("tags.#0 tags.#1 tags.#2")
    );
}

// ── The parse is the check ───────────────────────────────────────────────

/// An entry that is not a `u8` becomes an item that holds an error, as typed
/// text that does not parse would. Added, so the user sees it, and the form
/// does not validate.
#[gtest]
fn a_raw_entry_that_does_not_parse_is_an_invalid_item() {
    let mut form = form_for(
        &Sizes {
            sizes: BTreeSet::new(),
        },
        FormSpec::default(),
    );
    let at = form
        .edit(&add_raw("sizes", "big"))
        .expect("an edit, not a refusal")
        .expect("an item was added");
    expect_that!(form.validate(), none());
    expect_that!(form.collect_errors().fields.get(&at), some(anything()));
}

/// An entry naming no variant is the item's error, as a stale variant name off
/// the wire is.
#[gtest]
fn a_raw_entry_naming_no_variant_is_an_invalid_item() {
    let mut form = form_for(
        &Meeting {
            days: BTreeSet::new(),
        },
        form! { Meeting { days => { sort_by: Weekday::cmp } } },
    );
    let at = form
        .edit(&add_raw("days", "Sunday"))
        .expect("an edit, not a refusal")
        .expect("an item was added");
    expect_that!(form.validate(), none());
    expect_that!(form.collect_errors().fields.get(&at), some(anything()));
}

/// An entry outside the items' `choices:` is refused on the item by the field
/// itself, exactly as one arriving in a model is
/// (`set_item_errors::an_item_outside_the_choices_is_refused_on_that_item`).
#[gtest]
fn a_raw_entry_outside_the_choices_is_refused_on_the_item() {
    let mut form = form_for(
        &Picked {
            cities: BTreeSet::new(),
        },
        form! { Picked { cities[] => { widget: select, choices: CITIES } } },
    );
    let at = form
        .edit(&add_raw("cities", "XXX"))
        .expect("an edit, not a refusal")
        .expect("an item was added");
    expect_that!(form.validate(), none());
    expect_that!(
        form.collect_errors().fields.get(&at),
        some(elements_are![eq(&ValidationMessage::from(
            "not one of the available choices"
        ))])
    );
}

// ── Removing ─────────────────────────────────────────────────────────────

/// Removing a raw-added item takes its error with it: nothing is left to
/// report once the bad city is gone.
#[gtest]
fn removing_a_raw_item_clears_its_errors() {
    let mut form = form_for(
        &Picked {
            cities: ["BOS".to_string()].into(),
        },
        form! { Picked { cities[] => { widget: select, choices: CITIES } } },
    );
    form.edit(&add_raw("cities", "XXX"))
        .expect("an edit, not a refusal");
    expect_that!(form.validate(), none());
    // Sorted: BOS, XXX.
    form.edit(&Edit::RemoveItem {
        path: NodePath::new("cities"),
        index: 1,
    })
    .expect("XXX is item 1");
    expect_that!(form.collect_errors().fields, is_empty());
    expect_that!(
        form.validate(),
        some(eq(&Picked {
            cities: ["BOS".to_string()].into(),
        }))
    );
}

/// And its entry leaves the store, as any removed item's does
/// (`sets::a_removed_items_value_leaves_the_store`).
#[gtest]
fn removing_a_raw_item_clears_its_store_key() {
    #[component]
    fn Probe() -> Element {
        let form = use_form(|| form_for(&post(&["go"]), FormSpec::default()));
        use_hook(|| {
            form.edit(&add_raw("tags", "rust")).expect("rust is new");
            // Sorted: go, rust.
            form.edit(&Edit::RemoveItem {
                path: NodePath::new("tags"),
                index: 1,
            })
            .expect("rust is item 1")
        });
        with_store_keys(form)
    }
    // `#0` go, `#1` the draft; `#2`, rust's, is gone.
    expect_that!(
        store_keys(&super::render_to_html(Probe)),
        eq("tags.#0 tags.#1")
    );
}
