//! `Submission<T>` — the server side of a form round trip.
//!
//! These stand in for a `#[post]` handler: raw `(path, value)` pairs arrive,
//! the form is rebuilt from the same `FormSpec` the client used, and either a
//! model or a `FormErrors` comes back. Nothing here needs a Dioxus runtime,
//! which is the property that makes the whole approach work.

use std::collections::HashMap;

use crate::reflect::*;
use facet::Facet;
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Credentials {
    username: String,
    password: String,
    confirm_password: String,
}

fn passwords_must_match(c: &Credentials) -> Vec<FormError> {
    if c.password != c.confirm_password {
        vec![FormError("Passwords don't match.".to_string())]
    } else {
        Vec::new()
    }
}

fn credentials_spec() -> FormSpec<Credentials> {
    FormSpec::default().with_validator(passwords_must_match)
}

fn wire(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn filled() -> HashMap<String, String> {
    wire(&[
        ("username", "ada"),
        ("password", "hunter2"),
        ("confirm_password", "hunter2"),
    ])
}

fn paths(errors: &FormErrors) -> Vec<String> {
    errors.fields.iter().map(|(p, _)| p.clone()).collect()
}

fn form_messages(errors: &FormErrors) -> Vec<String> {
    errors.form.iter().map(|e| e.0.clone()).collect()
}

fn messages_at(errors: &FormErrors, path: &str) -> Vec<String> {
    errors
        .fields
        .iter()
        .filter(|(p, _)| p == path)
        .flat_map(|(_, errs)| errs.iter().map(|e| e.0.clone()))
        .collect()
}

// ── accept ───────────────────────────────────────────────────────────────

#[gtest]
fn a_complete_submission_yields_the_model() {
    let submission = Submission::accept(credentials_spec(), &filled())
        .expect("every field is filled and the passwords match");

    expect_that!(
        submission.model(),
        eq(&Credentials {
            username: "ada".to_string(),
            password: "hunter2".to_string(),
            confirm_password: "hunter2".to_string(),
        })
    );
}

#[gtest]
fn a_missing_field_is_rejected_before_the_handler_sees_anything() {
    // The client's `required` attribute is a hint, not the authority — a
    // request that never went through a browser is caught right here.
    let values = wire(&[("username", "ada"), ("password", "hunter2")]);
    let errors = Submission::accept(credentials_spec(), &values)
        .expect_err("confirm_password was never sent");

    expect_that!(paths(&errors), elements_are![eq("confirm_password")]);
    expect_that!(
        messages_at(&errors, "confirm_password"),
        elements_are![eq("This field is required.")]
    );
}

#[gtest]
fn an_unknown_path_is_ignored_rather_than_trusted() {
    // A forged request naming a field the form doesn't have must not be able to
    // inject anything: each leaf looks ITSELF up in the map, so a stray key has
    // nothing to attach to.
    let mut values = filled();
    values.insert("is_admin".to_string(), "true".to_string());

    let submission = Submission::accept(credentials_spec(), &values)
        .expect("a stray key is not a validation failure");
    expect_that!(submission.model().username, eq("ada"));
}

#[gtest]
fn the_specs_validator_runs_on_the_server_too() {
    // The whole reason the wire carries raw values instead of a typed model:
    // this check cannot be skipped by a client that simply doesn't run it.
    let values = wire(&[
        ("username", "ada"),
        ("password", "hunter2"),
        ("confirm_password", "hunter3"),
    ]);
    let errors =
        Submission::accept(credentials_spec(), &values).expect_err("the passwords disagree");

    expect_that!(form_messages(&errors), elements_are![eq("Passwords don't match.")]);
}

#[gtest]
fn a_cross_field_failure_arrives_with_no_field_errors_at_all() {
    // The trap this API exists to make unrepresentable. Every field is
    // individually fine, so `fields` is EMPTY — a caller reading that as
    // "accepted" would let a mismatched password through. The `Result` is the
    // only correct signal.
    let values = wire(&[
        ("username", "ada"),
        ("password", "hunter2"),
        ("confirm_password", "hunter3"),
    ]);
    let errors = Submission::accept(credentials_spec(), &values).expect_err("the passwords disagree");

    expect_that!(errors.fields, is_empty());
    expect_that!(errors.form, not(is_empty()));
}

// ── reject ───────────────────────────────────────────────────────────────

#[gtest]
fn a_server_verdict_lands_on_the_named_field() {
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let errors = submission.reject_field("username", "That username is taken.");

    expect_that!(paths(&errors), elements_are![eq("username")]);
    expect_that!(
        messages_at(&errors, "username"),
        elements_are![eq("That username is taken.")]
    );
    expect_that!(errors.form, is_empty());
}

#[gtest]
fn a_form_level_verdict_lands_in_form_with_no_field_named() {
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let errors = submission.reject("Those credentials don't match.");

    expect_that!(errors.fields, is_empty());
    expect_that!(
        form_messages(&errors),
        elements_are![eq("Those credentials don't match.")]
    );
}

#[gtest]
fn rejecting_reports_only_the_rejected_field() {
    // The submission validated cleanly, so nothing else has anything to say —
    // the server's verdict travels alone rather than dragging along the
    // now-clean state of every sibling.
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let errors = submission.reject_field("password", "That password was found in a breach.");

    expect_that!(paths(&errors), elements_are![eq("password")]);
}

#[gtest]
#[should_panic(expected = "cannot reject `nope`")]
fn rejecting_a_path_the_form_does_not_have_is_a_caller_bug() {
    // Loud rather than silent: dropping the message would leave the user with a
    // rejected submission and no visible reason for it.
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let _ = submission.reject_field("nope", "...");
}

#[gtest]
fn into_model_hands_the_value_onward() {
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    expect_that!(submission.into_model().username, eq("ada"));
}
