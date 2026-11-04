//! `min_items` and `max_items` (#14): the fewest and most items a list or
//! set may have. Checked in `validate` on both sides; at the limits, Add or
//! Remove is disabled; a minimum of one or more marks the legend required;
//! a blank list starts with its minimum of blank items; and a submission
//! listing more than the maximum is refused before any item is built.

use super::{Harness, render_to_html, without_form_id};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::nodes::{Edit, ItemValue, ValuesByPath};
use formoxus::prelude::*;
use formoxus::widgets::AddItemProps;
use googletest::prelude::*;
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Poll {
    answers: Vec<String>,
    tags: BTreeSet<String>,
}

fn poll(answers: &[&str], tags: &[&str]) -> Poll {
    Poll {
        answers: answers.iter().map(ToString::to_string).collect(),
        tags: tags.iter().map(ToString::to_string).collect(),
    }
}

fn spec() -> FormSpec<Poll> {
    form! {
        Poll {
            answers => { min_items: 2, max_items: 3 },
            tags => { max_items: 2 },
        }
    }
}

fn own_errors(form: &FormState<Poll>, path: &str) -> Vec<String> {
    form.collect_errors()
        .fields
        .get(path)
        .map(|found| found.iter().map(|m| m.0.clone()).collect())
        .unwrap_or_default()
}

// ── Validation ───────────────────────────────────────────────────────────

#[gtest]
fn counts_within_the_limits_pass() {
    let model = poll(&["Yes", "No"], &["a", "b"]);
    let mut form = form_for(&model, spec());
    expect_that!(form.validate(), some(eq(&model)));
}

/// A list short of its minimum, read from a model as it is: no blank items
/// are added to it, and it says so.
#[gtest]
fn too_few_items_is_the_lists_own_error() {
    let mut form = form_for(&poll(&["Yes"], &[]), spec());
    expect_that!(form.validate(), none());
    expect_that!(
        own_errors(&form, "answers"),
        elements_are![eq("Add at least 2 items.")]
    );
}

#[gtest]
fn too_many_items_is_the_lists_and_the_sets_own_error() {
    let mut form = form_for(&poll(&["a", "b", "c", "d"], &["x", "y", "z"]), spec());
    expect_that!(form.validate(), none());
    expect_that!(
        own_errors(&form, "answers"),
        elements_are![eq("No more than 3 items.")]
    );
    expect_that!(
        own_errors(&form, "tags"),
        elements_are![eq("No more than 2 items.")]
    );
}

/// The count is the collection's own, whatever its items say: a bad item
/// does not hide that there are too few.
#[gtest]
fn the_count_is_checked_whatever_the_items_say() {
    let mut form = form_for(&poll(&["Yes"], &[]), spec());
    form.distribute_values(
        &[("answers.#0".to_string(), String::new())]
            .into_iter()
            .collect(),
    );
    expect_that!(form.validate(), none());
    expect_that!(
        own_errors(&form, "answers"),
        elements_are![eq("Add at least 2 items.")]
    );
    expect_that!(own_errors(&form, "answers.#0"), not(is_empty()));
}

// ── A blank form ─────────────────────────────────────────────────────────

/// A blank list starts with its minimum of blank items, so the user sees
/// the fields to fill; a set does not, since its blank items would be
/// duplicates of each other.
#[gtest]
fn a_blank_list_starts_with_its_minimum() {
    let form = empty_form(spec());
    let values = form.collect_values();
    expect_that!(values.get("answers"), some(eq("#0,#1")));
    expect_that!(values.get("tags"), some(eq("")));
}

/// Its blank items are real items: filled, they are the model.
#[gtest]
fn the_blank_items_fill_as_any_item_does() {
    let mut form = empty_form(spec());
    form.distribute_values(
        &[
            ("answers.#0".to_string(), "Yes".to_string()),
            ("answers.#1".to_string(), "No".to_string()),
        ]
        .into_iter()
        .collect(),
    );
    expect_that!(form.validate(), some(eq(&poll(&["Yes", "No"], &[]))));
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Question {
    prompt: String,
    choices: Vec<String>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    questions: Vec<Question>,
}

/// An item added to a list is blank, so a list inside it starts with ITS
/// minimum too: a new question comes with its two blank choices.
#[gtest]
fn a_new_items_own_list_starts_with_its_minimum() {
    let mut form = form_for(
        &Quiz {
            questions: Vec::new(),
        },
        form! { Quiz { questions[].choices => { min_items: 2 } } },
    );
    let added = form
        .edit(&Edit::AddItem {
            path: "questions".to_string(),
            before: None,
            value: None,
        })
        .expect("questions is a list")
        .expect("an item was made");
    expect_that!(
        form.collect_values().get(&format!("{added}.choices")),
        some(eq("#0,#1"))
    );
}

// ── What the user sees ───────────────────────────────────────────────────

#[component]
fn AtTheLimits() -> Element {
    let form = use_form(|| {
        form_for(&poll(&["Yes", "No"], &["a", "b"]), {
            form! {
                Poll {
                    answers => { min_items: 2, max_items: 2 },
                    tags => { min_items: 2, max_items: 2 },
                }
            }
        })
    });
    form.render(using_fns! {})
}

/// At both limits at once: Add disabled with its reason, every Remove
/// disabled, and both legends marked required.
#[gtest]
fn at_its_limits_add_and_remove_are_disabled() {
    let html = without_form_id(&render_to_html(AtTheLimits));
    // The list's Add, and the set's.
    expect_that!(
        html.matches(
            r#"class="fx-button fx-button-add-item" disabled=true title="At most 2 items""#
        )
        .count(),
        eq(2)
    );
    // Two answers and two tags, each with a disabled Remove.
    expect_that!(
        html.matches(r#"fx-button-remove-item" aria-label=""#)
            .count(),
        eq(4)
    );
    expect_that!(
        html,
        contains_regex(r#"aria-label="Remove item 1" disabled=true"#)
    );
    expect_that!(
        html,
        contains_regex(r#"aria-label="Remove “a”" disabled=true"#)
    );
    for legend in ["Answers", "Tags"] {
        expect_that!(
            html,
            contains_substring(format!(
                r#"{legend}<span class="fx-required" aria-hidden="true"> *</span></legend>"#
            )),
            "{legend}"
        );
    }
}

#[component]
fn WithRoom() -> Element {
    let form = use_form(|| form_for(&poll(&["Yes", "No"], &["a"]), spec()));
    form.render(using_fns! {})
}

/// With room, nothing is disabled, and a list with no minimum of one is not
/// marked required.
#[gtest]
fn with_room_nothing_is_disabled() {
    let html = without_form_id(&render_to_html(WithRoom));
    expect_that!(html, not(contains_substring(r#"title="At most"#)));
    expect_that!(html, contains_substring("Tags</legend>"));
    // `answers` is at its minimum of 2, so its Removes are disabled; the
    // set's one tag has no minimum.
    expect_that!(html, contains_regex(r#"aria-label="Remove “a”"[^>]*>"#));
    expect_that!(
        html,
        not(contains_regex(r#"aria-label="Remove “a”" disabled=true"#))
    );
}

#[component]
fn FullSetEditing() -> Element {
    let form = use_form(|| form_for(&poll(&["Yes", "No"], &["a", "b"]), spec()));
    use_hook(|| {
        form.edit(&Edit::EditItem {
            path: "tags".to_string(),
            index: Some(0),
        })
        .expect("tags has an item 0")
    });
    form.render(using_fns! {})
}

/// A full set can still replace an item: a replacement leaves the count as
/// it is.
#[gtest]
fn a_full_set_can_still_replace() {
    let html = without_form_id(&render_to_html(FullSetEditing));
    expect_that!(html, contains_substring("Replace"));
    expect_that!(html, not(contains_substring(r#"title="At most"#)));
}

/// An adder of the dev's own is told the maximum, so it can stop there too.
#[gtest]
fn a_custom_adder_is_told_the_maximum() {
    #[component]
    fn Counting(props: AddItemProps) -> Element {
        rsx! { output { "{props.item_count} of {props.max_items:?}" } }
    }
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            form_for(
                &poll(&["Yes", "No"], &[]),
                form! { Poll { answers => { max_items: 3, adder: Counting } } },
            )
        });
        form.render(using_fns! {})
    }
    expect_that!(render_to_html(App), contains_substring("2 of Some(3)"));
}

/// Through `use_form`: a click that would pass the maximum cannot happen,
/// since the button is disabled; the edit itself is not refused, and
/// `validate` is what holds the line.
#[gtest]
fn an_add_past_the_maximum_fails_validation() {
    let mut form = form_for(&poll(&["a", "b", "c"], &[]), spec());
    form.edit(&Edit::AddItem {
        path: "answers".to_string(),
        before: None,
        value: Some(ItemValue::new("d".to_string())),
    })
    .expect("the edit itself is not refused");
    expect_that!(form.validate(), none());
    expect_that!(
        own_errors(&form, "answers"),
        elements_are![eq("No more than 3 items.")]
    );
}

// ── On the server ────────────────────────────────────────────────────────

/// A forged request listing far more item keys than the maximum is refused
/// before any of them are built.
#[gtest]
fn the_server_refuses_too_many_items_unbuilt() {
    let keys: Vec<String> = (0..10_000).map(|i| format!("#{i}")).collect();
    let mut values: ValuesByPath = [
        ("answers".to_string(), keys.join(",")),
        ("tags".to_string(), String::new()),
    ]
    .into_iter()
    .collect();
    values.insert("answers.#0".to_string(), "a".to_string());
    let refused = Submission::accept(spec(), &values).expect_err("10,000 answers is too many");
    expect_that!(
        refused.fields.get("answers"),
        some(elements_are![eq(&ValidationMessage::from(
            "No more than 3 items."
        ))])
    );
    // Unbuilt: the server's form keeps the two blank answers `empty_form`
    // starts with (its minimum), and builds none of the 10,000 listed, so no
    // item past those two has anything to say.
    let item_errors: Vec<&String> = refused
        .fields
        .keys()
        .filter(|path| path.starts_with("answers.#"))
        .collect();
    expect_that!(item_errors.len(), le(2));
    expect_that!(refused.fields.contains_key("answers.#9999"), eq(false));
}

#[component]
fn LiveForm() -> Element {
    let form = use_form(|| empty_form(spec()));
    form.render(using_fns! {})
}

/// The blank form's two answers each have a Remove, both disabled at the
/// minimum; Add has room.
#[gtest]
fn a_blank_form_renders_its_minimum() {
    let app = Harness::mount(LiveForm);
    let html = app.html();
    expect_that!(html.matches(r#"aria-label="Remove item"#).count(), eq(2));
    expect_that!(html, not(contains_substring(r#"title="At most"#)));
}
