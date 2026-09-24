//! `Submission<T>` — the server side of a form round trip.
//!
//! These stand in for a `#[post]` handler: raw `(path, value)` pairs arrive,
//! the form is rebuilt from the same `FormSpec` the client used, and either a
//! model or a `FormErrors` comes back. Nothing here needs a Dioxus runtime,
//! which is the property that makes the whole approach work.

use std::collections::HashMap;

use facet::Facet;
use formoxus::*;
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Credentials {
    username: String,
    password: String,
    confirm_password: String,
}

fn passwords_must_match(c: &Credentials) -> Vec<FormError> {
    if c.password == c.confirm_password {
        Vec::new()
    } else {
        vec![FormError("Passwords don't match.".to_string())]
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

    expect_that!(
        form_messages(&errors),
        elements_are![eq("Passwords don't match.")]
    );
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
    let errors =
        Submission::accept(credentials_spec(), &values).expect_err("the passwords disagree");

    expect_that!(errors.fields, is_empty());
    expect_that!(errors.form, not(is_empty()));
}

// ── reject ───────────────────────────────────────────────────────────────

#[gtest]
fn a_server_verdict_lands_on_the_named_field() {
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let wire = submission.reject_field(path!(Credentials.username), "That username is taken.");
    let errors = wire.errors();

    expect_that!(paths(errors), elements_are![eq("username")]);
    expect_that!(
        messages_at(errors, "username"),
        elements_are![eq("That username is taken.")]
    );
    expect_that!(errors.form, is_empty());
}

#[gtest]
fn a_form_level_verdict_lands_in_form_with_no_field_named() {
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let wire = submission.reject("Those credentials don't match.");
    let errors = wire.errors();

    expect_that!(errors.fields, is_empty());
    expect_that!(
        form_messages(errors),
        elements_are![eq("Those credentials don't match.")]
    );
}

#[gtest]
fn rejecting_reports_only_the_rejected_field() {
    // The submission validated cleanly, so nothing else has anything to say —
    // the server's verdict travels alone rather than dragging along the
    // now-clean state of every sibling.
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let wire = submission.reject_field(
        path!(Credentials.password),
        "That password was found in a breach.",
    );

    expect_that!(paths(wire.errors()), elements_are![eq("password")]);
}

/// The values ride back with the verdict, so the client can absorb both in one
/// call rather than holding its own copy and merging.
#[gtest]
fn a_rejection_carries_the_values_back_too() {
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let wire = submission.reject_field(path!(Credentials.username), "Taken.");

    expect_that!(wire.values().get("username"), some(eq("ada")));
    expect_that!(wire.is_clean(), eq(false));
}

/// A clean acceptance still produces a `WireForm`, so the return type of a
/// handler does not change shape between the accept and reject arms.
#[gtest]
fn an_accepted_submission_becomes_a_clean_wire_form() {
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    let wire = submission.into_wire();

    expect_that!(wire.is_clean(), eq(true));
    expect_that!(wire.values().get("password"), some(eq("hunter2")));
}

/// Values regenerated from a normalized model — the server changed something
/// and wants the client to show the corrected form.
#[gtest]
fn from_model_regenerates_every_leaf() {
    let mut model = Submission::accept(credentials_spec(), &filled())
        .expect("valid")
        .into_model();
    model.username = model.username.to_uppercase();

    let wire = WireForm::from_model(&model, credentials_spec());

    expect_that!(wire.values().get("username"), some(eq("ADA")));
    expect_that!(wire.values().get("password"), some(eq("hunter2")));
    expect_that!(wire.is_clean(), eq(true));
}

// A path the model does not have can no longer be WRITTEN: `reject_field` takes
// `Path<T>`, and `path!(Credentials.nope)` fails to compile. That guarantee is
// pinned by `tests/ui/path_unknown_field.rs` rather than here.
//
// The runtime panic it replaces is still reachable, but only for a path that
// exists on `T` while the form's tree does not hold it — an unchosen variant's
// field. `errors.rs` covers that through `FormState::push_field_error`, which
// keeps its `&str` for exactly the dynamic cases `path!` cannot spell.

#[gtest]
fn into_model_hands_the_value_onward() {
    let submission = Submission::accept(credentials_spec(), &filled()).expect("valid");
    expect_that!(submission.into_model().username, eq("ada"));
}
