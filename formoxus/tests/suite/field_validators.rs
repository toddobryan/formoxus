//! A field's own `validator:` (C4): a check over the field's typed value,
//! whose messages are that field's. On any kind of field: a leaf, an
//! `Option`'s or a newtype's, a struct, an enum, a list, a set, and each item
//! of a list or set. Django's `validators=` and `clean_<field>`.
//!
//! The rule every case shares: it runs only once everything under the field
//! is valid, since until then there is no whole value to judge.

use super::{Harness, render_to_html, without_form_id};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;
use googletest::prelude::*;
use std::collections::BTreeSet;

fn messages(texts: &[&str]) -> Vec<ValidationMessage> {
    texts.iter().map(|t| ValidationMessage::from(*t)).collect()
}

fn field_errors<T>(form: &FormState<T>, path: &str) -> Vec<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    form.collect_errors()
        .fields
        .get(path)
        .map(|found| found.iter().map(|m| m.0.clone()).collect())
        .unwrap_or_default()
}

// ── A leaf ───────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Bio(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    age: u8,
    nickname: Option<String>,
    bio: Bio,
}

/// A number's check takes the number itself, as clippy would have it.
fn must_be_even(age: u8) -> Vec<ValidationMessage> {
    if age.is_multiple_of(2) {
        Vec::new()
    } else {
        messages(&["Must be even."])
    }
}

/// Several checks in one validator, each with its own message. A `String`'s
/// check takes a `&str`.
fn no_spaces_and_short(nickname: &str) -> Vec<ValidationMessage> {
    let mut found = Vec::new();
    if nickname.contains(' ') {
        found.push("No spaces.".into());
    }
    if nickname.len() > 8 {
        found.push("At most 8 letters.".into());
    }
    found
}

fn signup_spec() -> FormSpec<Signup> {
    form! {
        Signup {
            age => { validator: must_be_even, max: 120 },
            nickname? => { validator: no_spaces_and_short },
            // A newtype's check takes the newtype, rebuilt from its leaf.
            bio => {
                validator: |bio: &Bio| if bio.0.is_empty() || bio.0.ends_with('.') {
                    Vec::new()
                } else {
                    vec!["End with a full stop.".into()]
                }
            },
        }
    }
}

fn signup(age: u8, nickname: Option<&str>, bio: &str) -> Signup {
    Signup {
        age,
        nickname: nickname.map(ToString::to_string),
        bio: Bio(bio.to_string()),
    }
}

#[gtest]
fn a_valid_model_passes_every_validator() {
    let model = signup(30, Some("ada"), "Hi.");
    let mut form = form_for(&model, signup_spec());
    expect_that!(form.validate(), some(eq(&model)));
}

#[gtest]
fn a_leafs_messages_are_its_own() {
    let mut form = form_for(&signup(31, None, "Hi."), signup_spec());
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "age"),
        elements_are![eq("Must be even.")]
    );
    expect_that!(form.collect_errors().fields.len(), eq(1));
}

/// One validator, several messages.
#[gtest]
fn a_validator_may_say_several_things() {
    let mut form = form_for(&signup(30, Some("ada lovelace"), "Hi."), signup_spec());
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "nickname"),
        elements_are![eq("No spaces."), eq("At most 8 letters.")]
    );
}

/// An absent `Option` is not checked: there is nothing to judge.
#[gtest]
fn an_absent_option_is_not_checked() {
    let mut form = form_for(&signup(30, None, "Hi."), signup_spec());
    expect_that!(form.validate(), some(anything()));
}

#[gtest]
fn a_newtypes_validator_gets_the_newtype() {
    let mut form = form_for(&signup(30, None, "No stop"), signup_spec());
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "bio"),
        elements_are![eq("End with a full stop.")]
    );
}

/// The field's own checks come first, and a failure there is the only
/// message: a constraint (`max`), and a value that does not parse.
#[gtest]
fn the_fields_own_checks_come_first() {
    let mut over = form_for(&signup(121, None, "Hi."), signup_spec());
    expect_that!(over.validate(), none());
    expect_that!(
        field_errors(&over, "age"),
        elements_are![contains_substring("120")]
    );

    let mut form = form_for(&signup(30, None, "Hi."), signup_spec());
    form.distribute_values(
        &[("age".to_string(), "thirty".to_string())]
            .into_iter()
            .collect(),
    );
    expect_that!(form.validate(), none());
    let age = field_errors(&form, "age");
    expect_that!(age.len(), eq(1));
    expect_that!(age, not(contains(eq("Must be even."))));
}

#[component]
fn OddAge() -> Element {
    let form = use_form(|| form_for(&signup(31, None, "Hi."), signup_spec()));
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

/// Shown beside the field, and describing its control, as any error is.
#[gtest]
fn a_leafs_message_renders_beside_it() {
    let html = without_form_id(&render_to_html(OddAge));
    expect_that!(
        html,
        all![
            contains_regex(r#"<input [^>]*id="FORMID-age"[^>]*aria-invalid="true""#),
            contains_substring(r#"<ul class="fx-field-errors" id="FORMID-age-errors">"#),
            contains_substring("Must be even."),
        ]
    );
}

/// A closure may capture what it checks against, as long as it can cross
/// to a server (`Send + Sync`).
#[gtest]
fn a_closure_may_capture_its_data() {
    let taken: Vec<String> = vec!["ada".to_string()];
    let spec = form! {
        Signup {
            nickname? => {
                validator: move |n: &str| if taken.iter().any(|t| t == n) { vec!["Taken.".into()] } else { Vec::new() }
            },
        }
    };
    let mut form = form_for(&signup(30, Some("ada"), "Hi."), spec);
    expect_that!(form.validate(), none());
    expect_that!(field_errors(&form, "nickname"), elements_are![eq("Taken.")]);
}

// ── The check's parameter, by the field's type ───────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Kinds {
    name: String,
    count: u32,
    ratio: f64,
    agreed: bool,
    names: Vec<String>,
    tags: BTreeSet<String>,
    bio: Bio,
}

/// Each written as a `fn` with the parameter clippy would want: `&str`,
/// the value itself, `&[T]`, and a reference for anything else. That this
/// compiles is the test; that each runs is the assertion.
#[gtest]
fn each_field_type_takes_its_idiomatic_parameter() {
    fn name(_: &str) -> Vec<ValidationMessage> {
        messages(&["name"])
    }
    fn count(_: u32) -> Vec<ValidationMessage> {
        messages(&["count"])
    }
    fn ratio(_: f64) -> Vec<ValidationMessage> {
        messages(&["ratio"])
    }
    fn agreed(_: bool) -> Vec<ValidationMessage> {
        messages(&["agreed"])
    }
    fn names(_: &[String]) -> Vec<ValidationMessage> {
        messages(&["names"])
    }
    fn tags(_: &BTreeSet<String>) -> Vec<ValidationMessage> {
        messages(&["tags"])
    }
    fn bio(_: &Bio) -> Vec<ValidationMessage> {
        messages(&["bio"])
    }
    let model = Kinds {
        name: "Ada".to_string(),
        count: 3,
        ratio: 0.5,
        agreed: true,
        names: vec!["x".to_string()],
        tags: ["y".to_string()].into(),
        bio: Bio("Hi.".to_string()),
    };
    let mut form = form_for(
        &model,
        form! {
            Kinds {
                name => { validator: name },
                count => { validator: count },
                ratio => { validator: ratio },
                agreed => { validator: agreed },
                names => { validator: names },
                tags => { validator: tags },
                bio => { validator: bio },
            }
        },
    );
    expect_that!(form.validate(), none());
    for path in ["name", "count", "ratio", "agreed", "names", "tags", "bio"] {
        expect_that!(field_errors(&form, path), elements_are![eq(path)], "{path}");
    }
}

// ── A struct ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Answer {
    text: String,
    correct: bool,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Question {
    prompt: String,
    answers: Vec<Answer>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    title: String,
    question: Question,
}

fn has_a_right_answer(q: &Question) -> Vec<ValidationMessage> {
    if q.answers.iter().any(|a| a.correct) {
        Vec::new()
    } else {
        messages(&["Mark at least one answer correct."])
    }
}

fn question(prompt: &str, correct: &[bool]) -> Question {
    Question {
        prompt: prompt.to_string(),
        answers: correct
            .iter()
            .enumerate()
            .map(|(i, &correct)| Answer {
                text: format!("Answer {i}"),
                correct,
            })
            .collect(),
    }
}

fn quiz_spec() -> FormSpec<Quiz> {
    form! { Quiz { question => { validator: has_a_right_answer } } }
}

#[gtest]
fn a_structs_validator_sees_the_whole_struct() {
    let quiz = Quiz {
        title: "Unit 1".to_string(),
        question: question("Pick one", &[false, false]),
    };
    let mut form = form_for(&quiz, quiz_spec());
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "question"),
        elements_are![eq("Mark at least one answer correct.")]
    );

    let mut fixed = form_for(
        &Quiz {
            question: question("Pick one", &[false, true]),
            ..quiz
        },
        quiz_spec(),
    );
    expect_that!(fixed.validate(), some(anything()));
}

/// A struct with a bad field is not judged as a whole: the field's error is
/// the only one. A prompt over its `max_length` still builds a `Question`
/// (it has no right answer either), so only the rule keeps the struct's
/// check from running.
#[gtest]
fn a_struct_with_a_bad_field_is_not_judged() {
    let quiz = Quiz {
        title: "Unit 1".to_string(),
        question: question("Far too long a prompt", &[false]),
    };
    let mut form = form_for(
        &quiz,
        form! {
            Quiz {
                question => { validator: has_a_right_answer },
                question.prompt => { max_length: 10 },
            }
        },
    );
    expect_that!(form.validate(), none());
    expect_that!(field_errors(&form, "question"), is_empty());
    expect_that!(field_errors(&form, "question.prompt"), not(is_empty()));
}

#[component]
fn NoRightAnswer() -> Element {
    let form = use_form(|| {
        form_for(
            &Quiz {
                title: "Unit 1".to_string(),
                question: question("Pick one", &[false]),
            },
            quiz_spec(),
        )
    });
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

/// A struct's own errors render under its legend, before its fields, and
/// mark and describe its `fieldset`, as a list's do.
#[gtest]
fn a_structs_message_renders_under_its_legend() {
    let html = without_form_id(&render_to_html(NoRightAnswer));
    expect_that!(
        html,
        contains_substring(
            r#"<fieldset class="fx-group fx-struct fx-invalid" aria-describedby="FORMID-question-errors">"#
        )
    );
    let legend = html.find("Question</legend>").expect("the legend renders");
    let message = html
        .find("Mark at least one answer correct.")
        .expect("the message renders");
    let prompt = html
        .find(r#"id="FORMID-question.prompt""#)
        .expect("the prompt renders");
    expect_that!(legend < message && message < prompt, eq(true));
}

/// A form validator's `path!` to a struct lands on it too, where it used to
/// fall through to the form's own errors.
#[gtest]
fn a_form_validators_error_at_a_struct_lands_on_it() {
    fn about_the_question(_: &Quiz) -> Vec<ValidationError<Quiz>> {
        vec![ValidationError::at(path!(Quiz.question), "Too easy.")]
    }
    let quiz = Quiz {
        title: "Unit 1".to_string(),
        question: question("Pick one", &[true]),
    };
    let mut form = form_for(
        &quiz,
        FormSpec::default().with_validator(about_the_question),
    );
    expect_that!(form.validate(), none());
    expect_that!(form.collect_errors().form, is_empty());
    expect_that!(
        field_errors(&form, "question"),
        elements_are![eq("Too easy.")]
    );
}

// ── Items of a list ──────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Exam {
    questions: Vec<Question>,
}

/// `questions[]` checks every item, each on its own.
#[gtest]
fn each_struct_item_is_checked_on_its_own() {
    let exam = Exam {
        questions: vec![question("One", &[true]), question("Two", &[false])],
    };
    let mut form = form_for(
        &exam,
        form! { Exam { questions[] => { validator: has_a_right_answer } } },
    );
    expect_that!(form.validate(), none());
    expect_that!(field_errors(&form, "questions.#0"), is_empty());
    expect_that!(
        field_errors(&form, "questions.#1"),
        elements_are![eq("Mark at least one answer correct.")]
    );
}

// ── An enum ──────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Group {
    All { questions: Vec<String> },
    Random { how_many: u8, pool: Vec<String> },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    group: Group,
}

fn draws_no_more_than_its_pool(group: &Group) -> Vec<ValidationMessage> {
    match group {
        Group::Random { how_many, pool } if usize::from(*how_many) > pool.len() => {
            vec![format!("Can't draw {how_many} from {}.", pool.len()).into()]
        }
        _ => Vec::new(),
    }
}

fn assignment_spec() -> FormSpec<Assignment> {
    form! { Assignment { group => { validator: draws_no_more_than_its_pool } } }
}

fn random(how_many: u8, pool: &[&str]) -> Assignment {
    Assignment {
        group: Group::Random {
            how_many,
            pool: pool.iter().map(ToString::to_string).collect(),
        },
    }
}

/// The whole enum, whichever variant is chosen; the message is the
/// variant picker's, where an enum's own errors show.
#[gtest]
fn an_enums_validator_sees_the_chosen_variant() {
    let mut form = form_for(&random(3, &["q1", "q2"]), assignment_spec());
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "group"),
        elements_are![eq("Can't draw 3 from 2.")]
    );

    let mut fine = form_for(&random(2, &["q1", "q2"]), assignment_spec());
    expect_that!(fine.validate(), some(anything()));
}

#[component]
fn OverdrawnPool() -> Element {
    let form = use_form(|| form_for(&random(3, &["q1"]), assignment_spec()));
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

#[gtest]
fn an_enums_message_renders_with_its_select() {
    let html = without_form_id(&render_to_html(OverdrawnPool));
    expect_that!(
        html,
        all![
            contains_regex(r#"<select [^>]*id="FORMID-group"[^>]*aria-invalid="true""#),
            contains_substring("Can&#39;t draw 3 from 1."),
        ]
    );
}

// ── A list and a set ─────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    links: Vec<String>,
    tags: BTreeSet<String>,
}

fn post(links: &[&str], tags: &[&str]) -> Post {
    Post {
        links: links.iter().map(ToString::to_string).collect(),
        tags: tags.iter().map(ToString::to_string).collect(),
    }
}

fn post_spec() -> FormSpec<Post> {
    form! {
        Post {
            links => {
                // Un-annotated: a `Vec`'s check takes a slice, and the closure
                // is told so by the field.
                validator: |links| if <[String]>::is_empty(links) { vec!["Give at least one link.".into()] } else { Vec::new() }
            },
            tags => {
                validator: |tags| if BTreeSet::len(tags) > 3 { vec!["At most three tags.".into()] } else { Vec::new() }
            },
        }
    }
}

#[gtest]
fn a_lists_and_a_sets_validators_see_the_whole_collection() {
    let mut form = form_for(&post(&[], &["a", "b", "c", "d"]), post_spec());
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "links"),
        elements_are![eq("Give at least one link.")]
    );
    expect_that!(
        field_errors(&form, "tags"),
        elements_are![eq("At most three tags.")]
    );

    let mut fine = form_for(&post(&["x"], &["a"]), post_spec());
    expect_that!(fine.validate(), some(anything()));
}

#[component]
fn NoLinks() -> Element {
    let form = use_form(|| form_for(&post(&[], &[]), post_spec()));
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

/// Under the list's legend, where its own errors already go.
#[gtest]
fn a_lists_message_renders_under_its_legend() {
    let html = without_form_id(&render_to_html(NoLinks));
    expect_that!(
        html,
        contains_substring(
            r#"<fieldset class="fx-group fx-list fx-invalid" id="FORMID-links" aria-describedby="FORMID-links-errors">"#
        )
    );
    expect_that!(html, contains_substring("Give at least one link."));
}

// ── A set's item, through its draft ──────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tagged {
    tags: BTreeSet<String>,
}

#[component]
fn LowercaseTags() -> Element {
    let form = use_form(|| {
        empty_form(form! {
            Tagged {
                tags[] => {
                    validator: |tag| if str::chars(tag).any(char::is_uppercase) { vec!["Lowercase only.".into()] } else { Vec::new() }
                },
            }
        })
    });
    form.render(using_fns! {})
}

/// A set's draft is its item's field, so Add is refused by the item's check,
/// with the message on the draft, and nothing is added.
#[gtest]
fn a_set_items_validator_refuses_an_add() {
    let mut app = Harness::mount(LowercaseTags);
    app.fire("input", app.only_listener("input"), "Rust");
    app.click(app.only_listener("click"));
    let html = app.html();
    expect_that!(html, contains_substring("Lowercase only."));
    expect_that!(html, not(contains_substring(r#"<li class="fx-set-item">"#)));

    app.fire("input", app.only_listener("input"), "rust");
    app.click(app.only_listener("click"));
    expect_that!(
        app.html(),
        contains_substring(r#"<li class="fx-set-item">rust"#)
    );
}

// ── On the server ────────────────────────────────────────────────────────

/// `Submission::accept` validates as the client does, so a request that
/// skipped the browser still meets every validator.
#[gtest]
fn a_server_runs_the_same_validators() {
    let wire = WireForm::from_model(&signup(31, None, "Hi."), FormSpec::default());
    let refused = Submission::accept(signup_spec(), wire.values())
        .expect_err("an odd age is refused on the server too");
    expect_that!(
        refused.fields.get("age"),
        some(elements_are![eq(&ValidationMessage::from("Must be even."))])
    );
}
