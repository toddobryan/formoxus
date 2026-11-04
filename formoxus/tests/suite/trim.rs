//! Trimming (#16): a text field's raw text loses its leading and trailing
//! whitespace where it is READ, so a field of spaces is empty and `" Ada "`
//! is `"Ada"`. Never in the input: the store keeps what was typed. A
//! password is never trimmed, `no_trim` turns it off for any other text
//! field, and a number is always trimmed.

use super::Harness;
use dioxus::prelude::*;
use facet::Facet;
use formoxus::{nodes::ValuesByPath, prelude::*};
use googletest::prelude::*;
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Bio(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Account {
    name: String,
    nickname: Option<String>,
    password: String,
    padded: String,
    age: u8,
    bio: Bio,
}

fn spec() -> FormSpec<Account> {
    form! {
        Account {
            password => { widget: password },
            padded => { no_trim },
        }
    }
}

/// Every field, as raw text: what a browser sends and the server reads.
fn raw(pairs: &[(&str, &str)]) -> ValuesByPath {
    pairs
        .iter()
        .map(|(path, value)| ((*path).to_string(), (*value).to_string()))
        .collect()
}

fn read(pairs: &[(&str, &str)]) -> FormState<Account> {
    let mut form = empty_form(spec());
    form.distribute_values(&raw(pairs));
    form
}

fn typed() -> Vec<(&'static str, &'static str)> {
    vec![
        ("name", "  Ada  "),
        ("nickname", " \t "),
        ("password", " s3cret "),
        ("padded", "  as typed  "),
        ("age", " 36 "),
        ("bio", " Mathematician. "),
    ]
}

#[gtest]
fn each_field_is_read_as_its_rule_says() {
    let mut form = read(&typed());
    expect_that!(
        form.validate(),
        some(eq(&Account {
            // Trimmed, a text field's default.
            name: "Ada".to_string(),
            // Only whitespace, so absent.
            nickname: None,
            // A password keeps its spaces: they are part of it.
            password: " s3cret ".to_string(),
            // `no_trim`.
            padded: "  as typed  ".to_string(),
            // A number, which would not parse with its spaces.
            age: 36,
            // A string newtype trims as a string does.
            bio: Bio("Mathematician.".to_string()),
        }))
    );
}

/// A required field of spaces is unfilled, not present: what apcsp found,
/// where an answer of spaces was accepted.
#[gtest]
fn a_required_field_of_spaces_is_unfilled() {
    let mut pairs = typed();
    pairs[0] = ("name", "     ");
    let mut form = read(&pairs);
    expect_that!(form.validate(), none());
    expect_that!(
        form.collect_errors().fields.get("name"),
        some(elements_are![eq(&ValidationMessage::from(
            "This field is required."
        ))])
    );
}

/// A password of only spaces is a password, not an empty field: its spaces
/// are never trimmed, so it is not mistaken for one left blank.
#[gtest]
fn a_password_is_never_trimmed() {
    let mut pairs = typed();
    pairs[2] = ("password", "   ");
    let mut form = read(&pairs);
    expect_that!(
        form.validate().map(|account| account.password),
        some(eq("   "))
    );
}

/// A constraint judges the trimmed value: two letters inside spaces are two.
#[gtest]
fn constraints_see_the_trimmed_value() {
    let mut form = empty_form(form! { Account { name => { max_length: 3 } } });
    let mut pairs = typed();
    pairs[0] = ("name", "   Ada   ");
    form.distribute_values(&raw(&pairs));
    expect_that!(form.validate(), some(anything()));
}

/// The server reads raw text the same way, so a request that skipped the
/// browser cannot sneak a field of spaces past `required`.
#[gtest]
fn the_server_trims_the_same_way() {
    let mut pairs = typed();
    pairs[0] = ("name", "   ");
    let refused = Submission::accept(spec(), &raw(&pairs))
        .expect_err("a name of spaces is refused on the server too");
    expect_that!(refused.fields.get("name"), some(anything()));
}

// ── The input keeps what was typed ───────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Named {
    name: String,
}

#[component]
fn NamedForm() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Named>::default()));
    rsx! {
        {form.render(using_fns! {})}
        output { id: "validated", "{form.validate():?}" }
    }
}

/// Trimming is where the text is read, never in the store, so the input
/// still shows the space a user may be about to type past.
#[gtest]
fn the_input_keeps_the_spaces_typed() {
    let mut app = Harness::mount(NamedForm);
    app.fire("input", app.only_listener("input"), "Ada ");
    let html = app.html();
    expect_that!(html, contains_substring(r#"value="Ada ""#));
    expect_that!(
        html,
        contains_substring("Some(Named { name: &#34;Ada&#34; })")
    );
}

// ── A set's draft ────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tagged {
    tags: BTreeSet<String>,
}

#[component]
fn TagsForm() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Tagged>::default()));
    rsx! {
        {form.render(using_fns! {})}
        output { id: "validated", "{form.validate():?}" }
    }
}

/// Added trimmed, so a tag typed with a space is the same tag, and a draft
/// of spaces adds nothing.
#[gtest]
fn a_sets_draft_is_trimmed_when_added() {
    let mut app = Harness::mount(TagsForm);
    app.fire("input", app.only_listener("input"), "  rust ");
    app.click(app.only_listener("click"));
    let html = app.html();
    expect_that!(html, contains_substring(r#"<li class="fx-set-item">rust<"#));
    expect_that!(html, contains_substring("tags: {&#34;rust&#34;}"));
}
