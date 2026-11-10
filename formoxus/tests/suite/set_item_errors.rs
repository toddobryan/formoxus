//! An error on ONE item of a set, named by its value: `path!(Model.set[= v])`.
//!
//! A set's items are unique by value, so a value names one, where an index
//! could not: a set's order in the model need not be the order it is shown
//! in. The cases are a dev's own checks the field cannot make itself, such as
//! a choice that stopped being legal, a retired tag, or a day with no room.
//! (A value outside a set's static `choices:` is refused by the field; see
//! the last section.)

use super::render_to_html;
use dioxus::prelude::*;
use facet::Facet;
use formoxus::nodes::{Edit, ItemValue};
use formoxus::path::NodePath;
use formoxus::prelude::*;
use googletest::prelude::*;
use std::collections::{BTreeSet, HashSet};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

fn post(tags: &[&str]) -> Post {
    Post {
        tags: tags.iter().map(ToString::to_string).collect(),
    }
}

/// Tags that have been retired since they were added: a list from somewhere
/// the field cannot see, such as a database.
fn no_retired_tags(p: &Post) -> Vec<ValidationError<Post>> {
    const RETIRED: &[&str] = &["perl", "cobol"];
    p.tags
        .iter()
        .filter(|tag| RETIRED.contains(&tag.as_str()))
        .map(|tag| ValidationError::at(path!(Post.tags[= tag]), format!("“{tag}” is retired.")))
        .collect()
}

fn validated(model: &Post) -> FormState<Post> {
    let mut form = form_for(model, FormSpec::default().with_validator(no_retired_tags));
    expect_that!(form.validate(), none());
    form
}

#[gtest]
fn a_validator_puts_its_error_on_the_item_with_that_value() {
    let form = validated(&post(&["go", "perl", "rust"]));
    let errors = form.collect_errors();
    expect_that!(errors.form, is_empty());
    // Sorted: go #0, perl #1, rust #2.
    expect_that!(
        errors.fields.get("tags.#1"),
        some(elements_are![eq(&ValidationMessage::from(
            "“perl” is retired."
        ))])
    );
    expect_that!(errors.fields.len(), eq(1));
}

/// Keys and places both move as items come and go; the value finds the item
/// wherever it now is.
#[gtest]
fn the_value_finds_its_item_after_adds_and_removes() {
    let mut form = form_for(
        &post(&["go", "rust"]),
        FormSpec::default().with_validator(no_retired_tags),
    );
    form.edit(&Edit::RemoveItem {
        path: NodePath::new("tags"),
        index: 0,
    })
    .expect("go is item 0");
    form.edit(&Edit::AddItem {
        path: NodePath::new("tags"),
        before: None,
        value: Some(ItemValue::new("cobol".to_string())),
    })
    .expect("cobol is a new tag");
    expect_that!(form.validate(), none());
    let keyed: Vec<String> = form
        .collect_errors()
        .fields
        .keys()
        .map(ToString::to_string)
        .collect();
    let added = form
        .collect_values()
        .into_iter()
        .find(|(_, value)| value == "cobol")
        .map(|(path, _)| path)
        .expect("cobol is an item");
    expect_that!(keyed, elements_are![eq(added.as_str())]);
}

/// Shown beside the item, as any of its errors is.
#[component]
fn RetiredTag() -> Element {
    let form = use_form(|| {
        form_for(
            &post(&["go", "perl", "rust"]),
            FormSpec::default().with_validator(no_retired_tags),
        )
    });
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

#[gtest]
fn the_error_renders_beside_its_item() {
    let html = render_to_html(RetiredTag);
    let item = html
        .find(r#"<li class="fx-set-item">perl"#)
        .expect("perl renders");
    let message = html.find("“perl” is retired.").expect("the error renders");
    let next = html
        .find(r#"<li class="fx-set-item">rust"#)
        .expect("rust renders");
    expect_that!(item < message && message < next, eq(true));
}

/// A value no item holds names nothing: the message is kept, with the form's.
#[gtest]
fn a_value_no_item_holds_becomes_a_form_error() {
    fn about_java(_: &Post) -> Vec<ValidationError<Post>> {
        vec![ValidationError::at(
            path!(Post.tags[= "java"]),
            "No java here.",
        )]
    }
    let mut form = form_for(
        &post(&["go"]),
        FormSpec::default().with_validator(about_java),
    );
    expect_that!(form.validate(), none());
    let errors = form.collect_errors();
    expect_that!(errors.fields, is_empty());
    expect_that!(
        errors.form_messages(),
        elements_are![contains_substring("No java here.")]
    );
}

/// A server's verdict on one item, and read back by the same path.
#[gtest]
fn a_server_can_reject_one_set_item_by_value() {
    let model = post(&["go", "perl"]);
    let wire = WireForm::from_model(&model, FormSpec::default());
    let submission =
        Submission::accept(FormSpec::default(), wire.values()).expect("the post is valid");
    let rejected = submission.reject_field(&path!(Post.tags[= "perl"]), "Not allowed here.");
    expect_that!(
        rejected.error_messages_at(&path!(Post.tags[= "perl"])),
        elements_are![eq("Not allowed here.")]
    );
    expect_that!(
        rejected.error_messages_at(&path!(Post.tags[= "go"])),
        is_empty()
    );
}

// ── Other kinds of item ──────────────────────────────────────────────────
//
// A value's text has to be the one its item writes, which differs by kind:
// a unit enum writes its variant's name, a newtype its inner value.

#[derive(Facet, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
enum Weekday {
    Monday,
    Wednesday,
    Friday,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Meeting {
    days: BTreeSet<Weekday>,
}

#[gtest]
fn a_unit_enum_item_is_found_by_its_variant() {
    fn not_on_friday(m: &Meeting) -> Vec<ValidationError<Meeting>> {
        m.days
            .iter()
            .filter(|day| **day == Weekday::Friday)
            .map(|day| ValidationError::at(path!(Meeting.days[= day]), "No room on Fridays."))
            .collect()
    }
    let meeting = Meeting {
        days: [Weekday::Monday, Weekday::Friday].into(),
    };
    let mut form = form_for(
        &meeting,
        form! { Meeting { days => { sort_by: Weekday::cmp } } }.with_validator(not_on_friday),
    );
    expect_that!(form.validate(), none());
    expect_that!(
        form.collect_errors().fields.get("days.#1"),
        some(elements_are![eq(&ValidationMessage::from(
            "No room on Fridays."
        ))])
    );
}

#[derive(Facet, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Tag(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shelf {
    labels: HashSet<Tag>,
}

/// A `HashSet` too, which has no order of its own at all.
#[gtest]
fn a_newtype_item_is_found_by_its_inner_value() {
    fn no_empty_shelf(s: &Shelf) -> Vec<ValidationError<Shelf>> {
        let old = Tag("old".to_string());
        if s.labels.contains(&old) {
            vec![ValidationError::at(
                path!(Shelf.labels[= &old]),
                "Relabel it.",
            )]
        } else {
            Vec::new()
        }
    }
    let shelf = Shelf {
        labels: [Tag("new".to_string()), Tag("old".to_string())].into(),
    };
    let mut form = form_for(
        &shelf,
        form! { Shelf { labels => { sort_by: Tag::cmp } } }.with_validator(no_empty_shelf),
    );
    expect_that!(form.validate(), none());
    let errors = form.collect_errors();
    let at = form
        .collect_values()
        .into_iter()
        .find(|(_, value)| value == "old")
        .map(|(path, _)| path)
        .expect("old is an item");
    expect_that!(
        errors.fields.get(&at),
        some(elements_are![eq(&ValidationMessage::from("Relabel it."))])
    );
}

// ── Static choices need none of this ─────────────────────────────────────

const CITIES: &[(&str, &str)] = &[("BOS", "Boston"), ("ORD", "Chicago")];

#[derive(Facet, Clone, Debug, PartialEq)]
struct Picked {
    cities: BTreeSet<String>,
}

/// An item that is no longer one of the set's `choices:` is refused by the
/// field itself, on that item, with no validator.
#[gtest]
fn an_item_outside_the_choices_is_refused_on_that_item() {
    let picked = Picked {
        cities: ["BOS", "XXX"].map(String::from).into(),
    };
    let mut form = form_for(
        &picked,
        form! { Picked { cities[] => { widget: select, choices: CITIES } } },
    );
    expect_that!(form.validate(), none());
    expect_that!(
        form.collect_errors().fields.get("cities.#1"),
        some(elements_are![eq(&ValidationMessage::from(
            "not one of the available choices"
        ))])
    );
}
