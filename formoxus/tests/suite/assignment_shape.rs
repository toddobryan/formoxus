//! apcsp's assignment form, end to end (`ORDERED_LISTS_PLAN.md` box 7): a
//! list of enum groups, each holding a list of question ids, the shape every
//! list feature has to survive at once. A list inside an enum inside a list,
//! moved at both levels, added to by a custom adder with a filled item, its
//! items shown by a display-only widget, and an error on one nested item.
//!
//! Each feature has its own tests elsewhere (`vecs.rs`, `enums.rs`); these
//! are for the nesting, where a path built wrong at one level shows up only
//! two levels down.

use super::{Harness, across_the_wire, render_to_html};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::nodes::{Edit, ItemValue};
use formoxus::prelude::*;
use formoxus::widgets::{AddItemProps, FieldErrors, Widget, WidgetProps};
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct AssignmentForm {
    name: String,
    groups: Vec<QuestionGroup>,
}

/// apcsp's three kinds of group. `Random` and `Streak` call their pool
/// `set`, which is also a `form!` flag: a field by that name must still nest.
#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum QuestionGroup {
    All {
        questions: Vec<String>,
        shuffle_questions: bool,
    },
    Random {
        how_many: u8,
        set: Vec<String>,
    },
    Streak {
        how_many: u8,
        set: Vec<String>,
    },
}

fn ids(ids: &[&str]) -> Vec<String> {
    ids.iter().map(ToString::to_string).collect()
}

fn assignment() -> AssignmentForm {
    AssignmentForm {
        name: "Unit 1 review".to_string(),
        groups: vec![
            QuestionGroup::All {
                questions: ids(&["q1", "q2", "q3"]),
                shuffle_questions: false,
            },
            QuestionGroup::Random {
                how_many: 2,
                set: ids(&["q4", "q5"]),
            },
            QuestionGroup::Streak {
                how_many: 3,
                set: ids(&["q6"]),
            },
        ],
    }
}

/// A question shown, not typed: its id in a hidden input, the id as text,
/// and the item's errors, which a custom widget renders itself.
#[derive(Clone, Debug, Default, PartialEq)]
struct QuestionItem;

impl Widget for QuestionItem {
    const NAME: &'static str = "question_item";
    const EXTENDS: formoxus::attrs::AttrSet = formoxus::attrs::AttrSet::EMPTY;
    const PARTS: &'static [formoxus::attrs::Tag] = &[];

    fn render(&self, props: WidgetProps) -> Element {
        let current = props.field_handle().current();
        rsx! {
            input { r#type: "hidden", id: "{props.id}", name: "{props.path}", value: "{current}" }
            span { class: "question", "{current}" }
            FieldErrors {
                field_id: props.id.clone(),
                paths: vec![props.path.clone()],
                errors: props.errors,
                classes: props.classes.clone()
            }
        }
    }
}

/// apcsp's search-to-add, reduced to one button that adds a FILLED item,
/// `q9`, to whichever list it is the adder of.
#[component]
fn QuestionAdder(props: AddItemProps) -> Element {
    let AddItemProps {
        path, id, on_edit, ..
    } = props;
    rsx! {
        button {
            r#type: "button",
            id: "{id}-add",
            // Dynamic, so `Harness::labelled` can tell the three adders apart.
            aria_label: "Add q9 to {path}",
            onclick: move |_| {
                on_edit.call(Edit::AddItem {
                    path: path.clone(),
                    before: None,
                    value: Some(ItemValue::new("q9".to_string())),
                });
            },
            "Add q9"
        }
    }
}

/// The form apcsp would declare: every nested list's items shown by
/// `QuestionItem`, and the custom adder on each, reached through `[]` and a
/// variant segment. That this compiles is the first test.
fn spec() -> FormSpec<AssignmentForm> {
    form! {
        AssignmentForm {
            groups[].$QuestionGroup::All.questions => { adder: QuestionAdder },
            groups[].$QuestionGroup::All.questions[] => { widget: QuestionItem },
            groups[].$QuestionGroup::Random.set => { adder: QuestionAdder },
            groups[].$QuestionGroup::Random.set[] => { widget: QuestionItem },
            groups[].$QuestionGroup::Streak.set => { adder: QuestionAdder },
            groups[].$QuestionGroup::Streak.set[] => { widget: QuestionItem },
        }
    }
}

// ── The data ─────────────────────────────────────────────────────────────

#[gtest]
fn the_assignment_round_trips() {
    let mut form = form_for(&assignment(), spec());
    expect_that!(form.validate(), some(eq(&assignment())));
    expect_that!(across_the_wire(&assignment()), ok(eq(&assignment())));
}

/// A moved group takes its questions with it: they live under the group's
/// key, which a move does not change.
#[gtest]
fn moving_a_group_carries_its_questions() {
    let mut form = form_for(&assignment(), spec());
    form.edit(&Edit::MoveItem {
        path: "groups".to_string(),
        from: 0,
        to: 2,
    })
    .expect("groups is a list with three items");

    let mut expected = assignment();
    expected.groups.rotate_left(1);
    expect_that!(form.validate(), some(eq(&expected)));
}

/// A move inside one group's questions reorders that group and no other.
#[gtest]
fn moving_a_question_reorders_only_its_group() {
    let mut form = form_for(&assignment(), spec());
    form.edit(&Edit::MoveItem {
        path: "groups.#0.$All.questions".to_string(),
        from: 2,
        to: 0,
    })
    .expect("the All group's questions are a list of three");

    let mut expected = assignment();
    expected.groups[0] = QuestionGroup::All {
        questions: ids(&["q3", "q1", "q2"]),
        shuffle_questions: false,
    };
    expect_that!(form.validate(), some(eq(&expected)));
}

/// A filled item added to a pool inside a variant, after the groups have
/// moved: the edit is addressed by key, so the move does not misdirect it.
#[gtest]
fn a_filled_item_reaches_a_pool_after_its_group_moves() {
    let mut form = form_for(&assignment(), spec());
    form.edit(&Edit::MoveItem {
        path: "groups".to_string(),
        from: 1,
        to: 0,
    })
    .expect("groups is a list with three items");
    let added = form.edit(&Edit::AddItem {
        path: "groups.#1.$Random.set".to_string(),
        before: None,
        value: Some(ItemValue::new("q9".to_string())),
    });
    expect_that!(added, ok(some(eq("groups.#1.$Random.set.#2"))));

    let mut expected = assignment();
    expected.groups.swap(0, 1);
    expected.groups[0] = QuestionGroup::Random {
        how_many: 2,
        set: ids(&["q4", "q5", "q9"]),
    };
    expect_that!(form.validate(), some(eq(&expected)));
}

// ── The markup ───────────────────────────────────────────────────────────

#[component]
fn AssignmentPage() -> Element {
    let form = use_form(|| form_for(&assignment(), spec()));
    form.render(using_fns! {})
}

/// Every nested item is a `QuestionItem`, named by its full path, and every
/// nested list has the custom adder in place of Add.
#[gtest]
fn nested_items_render_through_the_devs_widget_and_adder() {
    let html = render_to_html(AssignmentPage);
    expect_that!(
        html,
        all![
            contains_substring(r#"name="groups.#0.$All.questions.#2" value="q3""#),
            contains_substring(r#"name="groups.#1.$Random.set.#1" value="q5""#),
            contains_substring(r#"name="groups.#2.$Streak.set.#0" value="q6""#),
            contains_substring(r#"<span class="question">q6</span>"#),
            contains_regex(r#"id="[^"]+-groups\.#0\.\$All\.questions-add""#),
            contains_regex(r#"id="[^"]+-groups\.#1\.\$Random\.set-add""#),
            contains_regex(r#"id="[^"]+-groups\.#2\.\$Streak\.set-add""#),
        ]
    );
    // The outer list keeps the built-in Add: only the nested lists were given
    // the adder.
    expect_that!(html.matches("fx-button-add-item").count(), eq(1));
}

/// The ↑/↓ of the outer list and of each nested one, each numbered within
/// its own list: three groups, then the All group's three questions.
#[gtest]
fn every_level_has_its_own_move_buttons() {
    let html = render_to_html(AssignmentPage);
    expect_that!(
        html.matches(r#"aria-label="Move item 3 down""#).count(),
        eq(2)
    );
}

#[component]
fn AssignmentWithANestedError() -> Element {
    let form = use_form(|| form_for(&assignment(), spec()));
    form.absorb(WireForm::from_errors(FormErrors {
        form: Vec::new(),
        fields: [(
            "groups.#1.$Random.set.#0".to_string(),
            vec![ValidationMessage("Already in group 1.".to_string())],
        )]
        .into_iter()
        .collect(),
    }))
    .expect("the Random group's first question is a path of this form");
    form.render(using_fns! {})
}

/// An error at one nested item, as a server's duplicate check would send it,
/// renders beside that item and no other.
#[gtest]
fn an_error_on_a_nested_item_renders_beside_it() {
    let html = render_to_html(AssignmentWithANestedError);
    let at = html
        .find(r#"name="groups.#1.$Random.set.#0""#)
        .expect("the item renders");
    let message = html.find("Already in group 1.").expect("the error renders");
    let next = html
        .find(r#"name="groups.#1.$Random.set.#1""#)
        .expect("the next item renders");
    expect_that!(at < message && message < next, eq(true));
    expect_that!(html.matches("Already in group 1.").count(), eq(1));
}

// ── In use ───────────────────────────────────────────────────────────────

#[component]
fn LiveAssignment() -> Element {
    let form = use_form(|| form_for(&assignment(), spec()));
    form.render(using_fns! {})
}

/// The adder's click lands in the group it belongs to, and the new item
/// shows through the widget with its value already in the store.
#[gtest]
fn the_nested_adder_adds_a_filled_item_where_it_belongs() {
    let mut app = Harness::mount(LiveAssignment);
    app.click(app.labelled("click", "Add q9 to groups.#2.$Streak.set"));
    expect_that!(
        app.html(),
        all![
            contains_substring(r#"name="groups.#2.$Streak.set.#1" value="q9""#),
            contains_substring(r#"<span class="question">q9</span>"#),
        ]
    );
}

// ── A validator naming one nested item ──────────────────────────────────

/// apcsp's rule: a question may be in one group only. The error goes on the
/// LATER copy, named by its indices, which the form resolves to that item's
/// key wherever moves and removals have left it.
fn one_group_per_question(a: &AssignmentForm) -> Vec<ValidationError<AssignmentForm>> {
    let mut first_group = std::collections::HashMap::<&str, usize>::new();
    let mut errors = Vec::new();
    for (g, group) in a.groups.iter().enumerate() {
        let questions = match group {
            QuestionGroup::All { questions, .. } => questions,
            QuestionGroup::Random { set, .. } | QuestionGroup::Streak { set, .. } => set,
        };
        for (q, question) in questions.iter().enumerate() {
            match first_group.get(question.as_str()) {
                None => {
                    first_group.insert(question, g);
                }
                Some(&earlier) => {
                    let at = match group {
                        QuestionGroup::All { .. } => {
                            path!(AssignmentForm.groups[g].$QuestionGroup::All.questions[q])
                        }
                        QuestionGroup::Random { .. } => {
                            path!(AssignmentForm.groups[g].$QuestionGroup::Random.set[q])
                        }
                        QuestionGroup::Streak { .. } => {
                            path!(AssignmentForm.groups[g].$QuestionGroup::Streak.set[q])
                        }
                    };
                    errors.push(ValidationError::at(
                        at,
                        format!("Already in group {}.", earlier + 1),
                    ));
                }
            }
        }
    }
    errors
}

/// The assignment, but with `q2` in the Random group too, as its second item.
fn with_a_duplicate() -> AssignmentForm {
    let mut duplicated = assignment();
    duplicated.groups[1] = QuestionGroup::Random {
        how_many: 2,
        set: ids(&["q4", "q2", "q5"]),
    };
    duplicated
}

/// The message is on the duplicate's own item, and nowhere else.
#[gtest]
fn a_validator_puts_its_error_on_one_nested_item() {
    let mut form = form_for(
        &with_a_duplicate(),
        spec().with_validator(one_group_per_question),
    );
    expect_that!(form.validate(), none());
    let errors = form.collect_errors();
    expect_that!(errors.form, is_empty());
    expect_that!(
        errors.fields.get("groups.#1.$Random.set.#1"),
        some(elements_are![eq(&ValidationMessage::from(
            "Already in group 1."
        ))])
    );
    expect_that!(errors.fields.len(), eq(1));
}

/// After a move, an index and a key no longer agree: the Random group is
/// now FIRST, at index 0, but still `groups.#1`. The error follows the key.
#[gtest]
fn after_a_move_the_index_resolves_to_the_moved_items_key() {
    let mut form = form_for(
        &with_a_duplicate(),
        spec().with_validator(one_group_per_question),
    );
    form.edit(&Edit::MoveItem {
        path: "groups".to_string(),
        from: 1,
        to: 0,
    })
    .expect("groups is a list with three items");
    expect_that!(form.validate(), none());
    // Now the Random group comes first, so it is the All group's `q2` that
    // repeats one already seen.
    expect_that!(
        form.collect_errors()
            .fields
            .get("groups.#0.$All.questions.#1"),
        some(elements_are![eq(&ValidationMessage::from(
            "Already in group 1."
        ))])
    );
}

/// Rendered beside that item, as any field error is.
#[component]
fn ValidatedDuplicate() -> Element {
    let form = use_form(|| {
        form_for(
            &with_a_duplicate(),
            spec().with_validator(one_group_per_question),
        )
    });
    // Validated on its first render, as a submit would; `use_hook` so the
    // re-render it causes does not validate again.
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

#[gtest]
fn the_validators_error_renders_beside_the_duplicate() {
    let html = render_to_html(ValidatedDuplicate);
    let duplicate = html
        .find(r#"name="groups.#1.$Random.set.#1" value="q2""#)
        .expect("the duplicate renders");
    let message = html.find("Already in group 1.").expect("the error renders");
    let next = html
        .find(r#"name="groups.#1.$Random.set.#2""#)
        .expect("the next item renders");
    expect_that!(duplicate < message && message < next, eq(true));
}

/// A server's verdict by index, through `Submission::reject_field`, lands
/// the same way.
#[gtest]
fn a_server_can_reject_one_item_by_index() {
    let wire = WireForm::from_model(&assignment(), spec());
    let submission = Submission::accept(spec(), wire.values()).expect("the assignment is valid");
    let rejected = submission.reject_field(
        &path!(AssignmentForm.groups[2].$QuestionGroup::Streak.set[0]),
        "Not in this course.",
    );
    expect_that!(
        rejected.errors().fields.get("groups.#2.$Streak.set.#0"),
        some(elements_are![eq(&ValidationMessage::from(
            "Not in this course."
        ))])
    );
    // And read back by the same indexed path, as a client would.
    expect_that!(
        rejected.error_messages_at(&path!(
            AssignmentForm.groups[2].$QuestionGroup::Streak.set[0]
        )),
        elements_are![eq("Not in this course.")]
    );
    expect_that!(
        rejected.error_messages_at(&path!(
            AssignmentForm.groups[2].$QuestionGroup::Streak.set[1]
        )),
        is_empty()
    );
}

/// An index past the end names no item: the message is not lost, but shown
/// with the form's own errors, spelled as the validator wrote it.
#[gtest]
fn an_index_past_the_end_becomes_a_form_error() {
    fn past_the_end(_: &AssignmentForm) -> Vec<ValidationError<AssignmentForm>> {
        vec![ValidationError::at(
            path!(AssignmentForm.groups[5].$QuestionGroup::All.questions[0]),
            "No such group.",
        )]
    }
    let mut form = form_for(&assignment(), spec().with_validator(past_the_end));
    expect_that!(form.validate(), none());
    let errors = form.collect_errors();
    expect_that!(errors.fields, is_empty());
    expect_that!(
        errors.form_messages(),
        elements_are![contains_substring("groups[5].$All.questions[0]")]
    );
}
