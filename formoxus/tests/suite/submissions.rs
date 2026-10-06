//! `Submission<T>` — the server side of a form round trip.
//!
//! These stand in for a `#[post]` handler: raw `(path, value)` pairs arrive,
//! the form is rebuilt from the same `FormSpec` the client used, and either a
//! model or a `FormErrors` comes back. Nothing here needs a Dioxus runtime,
//! which is the property that makes the whole approach work.

use facet::Facet;
use formoxus::{members::ValuesByPath, prelude::*};
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Credentials {
    username: String,
    password: String,
    confirm_password: String,
}

fn passwords_must_match(c: &Credentials) -> Vec<ValidationError<Credentials>> {
    if c.password == c.confirm_password {
        Vec::new()
    } else {
        vec![ValidationError::form("Passwords don't match.")]
    }
}

fn credentials_spec() -> FormSpec<Credentials> {
    FormSpec::default().with_validator(passwords_must_match)
}

fn wire(pairs: &[(&str, &str)]) -> ValuesByPath {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn filled() -> ValuesByPath {
    wire(&[
        ("username", "ada"),
        ("password", "hunter2"),
        ("confirm_password", "hunter2"),
    ])
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

    expect_that!(errors.paths(), elements_are![eq("confirm_password")]);
    expect_that!(
        errors.messages_at(Some(path!(Credentials.confirm_password))),
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
        errors.form_messages(),
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

    expect_that!(errors.paths(), elements_are![eq("username")]);
    expect_that!(
        errors.messages_at(Some(path!(Credentials.username))),
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
        errors.form_messages(),
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

    expect_that!(wire.errors().paths(), elements_are![eq("password")]);
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

// ── `required_true` on a bool (issue #6) ────────────────────────────────
//
// The browser's `required` on a checkbox is trivially bypassed, so this is
// where "I agree to the terms" is actually enforced. An unticked checkbox is
// LEFT OUT of the form data entirely, which is why the missing-pair case is
// the one that matters most.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Terms {
    agreed: bool,
}

fn must_agree() -> FormSpec<Terms> {
    form! { Terms { agreed => { required_true } } }
}

#[gtest]
fn an_unticked_required_true_box_is_rejected_on_the_server() {
    let errors = Submission::accept(must_agree(), &wire(&[]))
        .expect_err("an unticked box sends nothing, and the box must be ticked");
    expect_that!(
        errors.messages_at(Some(path!(Terms.agreed))),
        elements_are![eq("this value must be true")]
    );
}

/// A request that sends `false` outright, which no browser does for a
/// checkbox but anything else can.
#[gtest]
fn an_explicit_false_is_rejected_the_same_way() {
    let errors = Submission::accept(must_agree(), &wire(&[("agreed", "false")]))
        .expect_err("false is not agreeing");
    expect_that!(
        errors.messages_at(Some(path!(Terms.agreed))),
        elements_are![eq("this value must be true")]
    );
}

#[gtest]
fn a_ticked_required_true_box_is_accepted() {
    let submission =
        Submission::accept(must_agree(), &wire(&[("agreed", "true")])).expect("the box is ticked");
    expect_that!(submission.model(), eq(&Terms { agreed: true }));
}

/// Without the rule, the same empty request is a complete answer: "no".
#[gtest]
fn an_unticked_plain_box_is_accepted_as_false() {
    let submission = Submission::accept(form! { Terms {} }, &wire(&[]))
        .expect("an unticked plain checkbox is an answer");
    expect_that!(submission.model(), eq(&Terms { agreed: false }));
}
