//! `Submission<T>` — the server side of a form round trip.
//!
//! These stand in for a `#[post]` handler: raw `(path, value)` pairs arrive,
//! the form is rebuilt from the same `FormSpec` the client used, and either a
//! model or a `FormErrors` comes back. Nothing here needs a Dioxus runtime,
//! which is the property that makes the whole approach work.

use super::across_the_wire;
use super::models::{Shape, Title};
use facet::Facet;
use formoxus::{nodes::ValuesByPath, prelude::*};
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

// ── Enums: the choice has to cross the wire (#13) ─────────────────────────
//
// The server rebuilds every request from `empty_form`, where each enum starts
// `Unchosen`, so the choice has to arrive as a value of its own: the variant's
// name at the enum's own path (`title` = `Dr`). Nothing else can carry it. A
// unit variant has no leaves at all, and a data-carrying one's `$Circle`
// segment is never read back.
//
// Every test here goes `from_model` -> `accept` -> `into_model`, the trip a
// real submission takes minus the browser, so a model that comes back unchanged
// is the whole assertion.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Greeting {
    title: Title,
    name: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Sketch {
    name: String,
    shape: Shape,
}

/// Tuple variants, which build exactly as struct variants do, with fields
/// named `0`, `1`, ….
#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Pen {
    Width(f64),
    Rgb(u8, u8, u8),
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Stroke {
    pen: Pen,
}

/// An enum reachable only through another enum's variant.
#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Fill {
    Solid { opacity: f64 },
    Hatched,
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Layer {
    Painted { fill: Fill },
    Empty,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Canvas {
    layer: Layer,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Gallery {
    shapes: Vec<Shape>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Letter {
    title: Option<Title>,
    name: String,
}

#[gtest]
fn a_unit_variant_survives_the_wire() {
    let greeting = Greeting {
        title: Title::Dr,
        name: "Ada".to_string(),
    };
    expect_that!(across_the_wire(&greeting), ok(eq(&greeting)));
}

#[gtest]
fn a_struct_variant_survives_the_wire() {
    let sketch = Sketch {
        name: "disc".to_string(),
        shape: Shape::Circle { radius: 2.0 },
    };
    expect_that!(across_the_wire(&sketch), ok(eq(&sketch)));
}

#[gtest]
fn a_tuple_variant_survives_the_wire() {
    let one = Stroke {
        pen: Pen::Width(1.5),
    };
    let three = Stroke {
        pen: Pen::Rgb(255, 128, 0),
    };
    expect_that!(across_the_wire(&one), ok(eq(&one)));
    expect_that!(across_the_wire(&three), ok(eq(&three)));
}

/// The spelling, pinned: the name at the enum's own path, for both kinds. A
/// browser-free client (a script, a test like these) has to send exactly this.
#[gtest]
fn the_choice_travels_as_the_variant_name_at_the_enums_path() {
    let unit = WireForm::from_model(
        &Greeting {
            title: Title::Ms,
            name: "Ada".to_string(),
        },
        FormSpec::default(),
    );
    expect_that!(unit.values().get("title"), some(eq("Ms")));

    let data = WireForm::from_model(
        &Sketch {
            name: "disc".to_string(),
            shape: Shape::Circle { radius: 2.0 },
        },
        FormSpec::default(),
    );
    expect_that!(data.values().get("shape"), some(eq("Circle")));
    expect_that!(data.values().get("shape.$Circle.radius"), some(eq("2")));
}

#[gtest]
fn a_missing_choice_is_still_an_error() {
    let errors = Submission::accept(FormSpec::<Greeting>::default(), &wire(&[("name", "Ada")]))
        .expect_err("a required enum with no choice");
    expect_that!(
        errors.messages_at(Some(path!(Greeting.title))),
        elements_are![eq("You must choose a variant for this field.")]
    );
}

/// `""` IS absence, here as everywhere else.
#[gtest]
fn an_empty_choice_is_a_missing_one() {
    let errors = Submission::accept(
        FormSpec::<Greeting>::default(),
        &wire(&[("title", ""), ("name", "Ada")]),
    )
    .expect_err("an empty choice is no choice");
    expect_that!(
        errors.messages_at(Some(path!(Greeting.title))),
        elements_are![eq("You must choose a variant for this field.")]
    );
}

/// Not only a forged request: a tab still running the old wasm after a deploy
/// that renamed a variant sends this honestly. Either way it arrives from the
/// client, so it is a field error, never a panic.
#[gtest]
fn an_unknown_variant_name_is_an_error_at_the_enums_path() {
    let errors = Submission::accept(
        FormSpec::<Greeting>::default(),
        &wire(&[("title", "Sir"), ("name", "Ada")]),
    )
    .expect_err("Sir is not a Title");
    let messages = errors.messages_at(Some(path!(Greeting.title)));
    expect_that!(
        messages,
        elements_are![all![
            contains_substring("Sir"),
            contains_substring("not a variant")
        ]],
        "the message names the bad choice, not the generic \"must choose\""
    );
}

/// The case the generic message would hide completely. An unchosen optional
/// enum is a legitimate `None`, so if the bad name were dropped the request
/// would be ACCEPTED, storing something the client never sent.
#[gtest]
fn an_unknown_variant_name_on_an_optional_enum_is_not_none() {
    let result = Submission::accept(
        FormSpec::<Letter>::default(),
        &wire(&[("title", "Sir"), ("name", "Ada")]),
    )
    .map(Submission::into_model);
    expect_that!(result, err(anything()), "Sir must not become None");
}

#[gtest]
fn an_unchosen_optional_enum_crosses_as_none() {
    let letter = Letter {
        title: None,
        name: "Ada".to_string(),
    };
    expect_that!(across_the_wire(&letter), ok(eq(&letter)));
}

#[gtest]
fn a_chosen_optional_enum_survives_the_wire() {
    let letter = Letter {
        title: Some(Title::Rev),
        name: "Ada".to_string(),
    };
    expect_that!(across_the_wire(&letter), ok(eq(&letter)));
}

/// The inner choice sits at `layer.$Painted.fill`, under the outer variant's
/// segment, so the outer one has to be chosen before the inner one can be read.
#[gtest]
fn an_enum_inside_a_chosen_variant_survives_the_wire() {
    let solid = Canvas {
        layer: Layer::Painted {
            fill: Fill::Solid { opacity: 0.5 },
        },
    };
    let hatched = Canvas {
        layer: Layer::Painted {
            fill: Fill::Hatched,
        },
    };
    expect_that!(
        WireForm::from_model(&hatched, FormSpec::default())
            .values()
            .get("layer.$Painted.fill"),
        some(eq("Hatched"))
    );
    expect_that!(across_the_wire(&solid), ok(eq(&solid)));
    expect_that!(across_the_wire(&hatched), ok(eq(&hatched)));
}

/// Each row carries its own choice. NOTE: this also needs `accept` to rebuild
/// the rows, which it does not do today for ANY list; see
/// `a_list_survives_the_wire` below.
#[gtest]
fn each_rows_choice_survives_the_wire() {
    let gallery = Gallery {
        shapes: vec![
            Shape::Circle { radius: 1.0 },
            Shape::Rectangle {
                width: 2.0,
                height: 3.0,
            },
        ],
    };
    let wire = WireForm::from_model(&gallery, FormSpec::default());
    expect_that!(wire.values().get("shapes.#0"), some(eq("Circle")));
    expect_that!(wire.values().get("shapes.#1"), some(eq("Rectangle")));
    expect_that!(across_the_wire(&gallery), ok(eq(&gallery)));
}

// ── Lists: the rows have to cross the wire too ───────────────────────────
//
// Found while writing the enum tests above, and worse than #13 because it is
// SILENT: `empty_form` builds a list with no rows, and `ListSet::
// distribute_values` only fills rows that exist, so every row on the wire is
// dropped and `accept` returns `Ok` with an empty `Vec`. The design notes
// predicted "create mode yields zero rows" before `Submission` existed; nothing
// connected it to the server rebuilding from `empty_form`.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tagged {
    name: String,
    tags: Vec<String>,
}

#[gtest]
fn a_list_survives_the_wire() {
    let tagged = Tagged {
        name: "post".to_string(),
        tags: vec!["rust".to_string(), "forms".to_string()],
    };
    expect_that!(across_the_wire(&tagged), ok(eq(&tagged)));
}

/// Rows arrive in the order the list's entry names them, which is not key
/// order: a row inserted at the front has the newest key.
#[gtest]
fn rows_are_rebuilt_in_the_order_the_entry_names_them() {
    let accepted = Submission::accept(
        FormSpec::<Tagged>::default(),
        &wire(&[
            ("name", "post"),
            ("tags", "#2,#0"),
            ("tags.#0", "second"),
            ("tags.#2", "first"),
        ]),
    )
    .map(Submission::into_model);
    expect_that!(
        accepted,
        ok(eq(&Tagged {
            name: "post".to_string(),
            tags: vec!["first".to_string(), "second".to_string()],
        }))
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Grid {
    rows: Vec<Vec<String>>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Salutations {
    titles: Vec<Option<Title>>,
}

/// A row with NO leaves at all, the case that inferring rows from leaf paths
/// could never recover: an empty inner list contributes only its own entry,
/// `rows.#0` = `""`.
#[gtest]
fn a_row_with_no_leaves_survives_the_wire() {
    let grid = Grid {
        rows: vec![Vec::new(), vec!["a".to_string()]],
    };
    expect_that!(
        WireForm::from_model(&grid, FormSpec::default())
            .values()
            .get("rows.#0"),
        some(eq(""))
    );
    expect_that!(across_the_wire(&grid), ok(eq(&grid)));
}

/// A `None` row is an unchosen optional enum: its only entry is `""`, and the
/// row has to come back as `None`, neither dropped nor an error.
#[gtest]
fn a_none_row_survives_the_wire() {
    let salutations = Salutations {
        titles: vec![None, Some(Title::Dr)],
    };
    expect_that!(across_the_wire(&salutations), ok(eq(&salutations)));
}

/// A row the client inserted at the FRONT has the newest key, so only the
/// list's own entry can tell the server it comes first.
#[gtest]
fn a_row_inserted_at_the_front_arrives_at_the_front() {
    let mut form = form_for(
        &Tagged {
            name: "post".to_string(),
            tags: vec!["rust".to_string(), "forms".to_string()],
        },
        FormSpec::default(),
    );
    form.edit(&formoxus::nodes::Edit::AddRow {
        path: "tags".to_string(),
        before: Some(0),
    })
    .expect("tags is a list");
    form.distribute_values(&wire(&[("tags.#2", "dioxus")]));

    let accepted = Submission::accept(FormSpec::<Tagged>::default(), &form.collect_values())
        .map(Submission::into_model);
    expect_that!(
        accepted,
        ok(eq(&Tagged {
            name: "post".to_string(),
            tags: vec![
                "dioxus".to_string(),
                "rust".to_string(),
                "forms".to_string()
            ],
        }))
    );
}

/// CHARACTERIZATION, not an endorsement. A payload that leaves out a list's
/// own entry gets an EMPTY list, silently, with its rows' leaves ignored:
/// absent means "leave the rows alone", and a server starts with none. A
/// required enum would at least error. `Form::to_wire` and
/// `WireForm::from_model` always send the entry, so only a hand-built client
/// gets here. Do NOT build anything that submits without structure (a native
/// `<form>` post straight to `Submission::accept`, say) on this behaviour; if
/// that is ever wanted, a missing entry has to become an error first.
#[gtest]
fn a_missing_row_list_is_an_empty_list() {
    let accepted = Submission::accept(
        FormSpec::<Tagged>::default(),
        &wire(&[("name", "post"), ("tags.#0", "rust")]),
    )
    .map(Submission::into_model);
    expect_that!(
        accepted,
        ok(eq(&Tagged {
            name: "post".to_string(),
            tags: Vec::new(),
        }))
    );
}

/// No client writes this, so it is forged or broken, and either way it
/// arrives from outside: a field error at the list's path, never a panic.
#[gtest]
fn a_malformed_row_list_is_an_error_at_the_lists_path() {
    let errors = Submission::accept(
        FormSpec::<Tagged>::default(),
        &wire(&[("name", "post"), ("tags", "#0,#0"), ("tags.#0", "rust")]),
    )
    .expect_err("a repeated key is not a list of rows");
    expect_that!(
        errors.messages_at(Some(path!(Tagged.tags))),
        elements_are![contains_substring("not a list of row keys")]
    );
}
