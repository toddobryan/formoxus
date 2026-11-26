//! Newtypes: `struct Markdown(String)` and its kind.
//!
//! **No newtype is special** (`CUSTOM_FIELDS_PLAN.md` decisions N and W,
//! 2026-11-26). Formoxus does not guess that a one-field tuple struct is its
//! inner value, and ignores `#[facet(transparent)]` too: explicit
//! is less surprising. A newtype gets a field the way any type of the dev's
//! own does, and `formoxus::fields::Newtype<T, F>` makes that one line. It is
//! a `Field<Value = T>` that hands every question to an inner field `F` and
//! wraps and unwraps through facet, so the model needs nothing extra.
//!
//! Registered here (`register`) for the whole suite with
//! `TypeRegistry::newtype::<T, V>()` (decision Z), which is `field_for` with
//! a default `Newtype<T, <V as DefaultField>::Field>`. The per-path route,
//! `newtype: String` in `form!` (decision Y), is tested with the macro.
//!
//! These pin both halves: the string-facing side behaves exactly like the
//! inner field, and the value still rebuilds as the newtype.

use super::{across_the_wire, install_test_registry, panic_message, render_to_html};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::{Attr, AttrValue, FieldType};
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::field_attrs::FieldAttrs;
use formoxus::fields::{Newtype, NumberField, StringField};
use formoxus::path::NodePath;
use formoxus::prelude::*;
use formoxus::type_registry::TypeRegistry;
use formoxus::widgets::{AnyWidget, Choice, Textarea};
use futures::executor::block_on;
use googletest::prelude::*;

/// Every newtype this module registers, for `super::install_test_registry`.
pub(crate) fn register(types: TypeRegistry) -> TypeRegistry {
    types
        .newtype::<Note, String>()
        .newtype::<Count, u32>()
        .field_for(Newtype::<Motto, _>::new(StringField {
            max_length: Some(5),
            ..StringField::default()
        }))
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Note(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Count(u32);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Doc {
    title: String,
    body: Note,
}

/// A one-field NAMED struct, registered by nothing: still a struct.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Config {
    name: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Settings {
    config: Config,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tally {
    label: String,
    count: Count,
}

/// A one-field tuple struct around something that ISN'T a scalar, registered
/// by nothing. No single input could carry it, so it stays a struct, with
/// nothing to panic about.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Pair(Config);

#[derive(Facet, Clone, Debug, PartialEq)]
struct HasPair {
    pair: Pair,
}

fn paths_of(
    form: &FormState<impl Clone + std::fmt::Debug + PartialEq + Facet<'static>>,
) -> Vec<String> {
    form.collect_values()
        .into_iter()
        .map(|(p, _)| p.to_string())
        .collect()
}

fn value_at(
    form: &FormState<impl Clone + std::fmt::Debug + PartialEq + Facet<'static>>,
    path: &str,
) -> Option<String> {
    form.collect_values()
        .into_iter()
        .find(|(p, _)| p == path)
        .map(|(_, v)| v)
}

fn error_paths(
    form: &FormState<impl Clone + std::fmt::Debug + PartialEq + Facet<'static>>,
) -> Vec<String> {
    form.collect_errors()
        .fields
        .iter()
        .map(|(p, _)| p.to_string())
        .collect()
}

// ── A registered newtype is one leaf ─────────────────────────────────────

#[gtest]
fn a_registered_newtype_is_one_leaf_at_the_fields_own_path() {
    install_test_registry();
    let form = empty_form::<Doc>(FormSpec::default());
    expect_that!(
        paths_of(&form),
        elements_are![eq("title"), eq("body")],
        "a newtype must NOT produce `body.0`"
    );
}

#[gtest]
fn a_one_field_named_struct_is_still_a_struct() {
    // Leaf paths are the wire format, so a struct that happens to have one
    // field today must not be renamed to `config` behind the dev's back.
    let form = empty_form::<Settings>(FormSpec::default());
    expect_that!(paths_of(&form), elements_are![eq("config.name")]);
}

#[gtest]
fn a_newtype_around_a_non_scalar_is_still_a_struct() {
    let form = empty_form::<HasPair>(FormSpec::default());
    expect_that!(paths_of(&form), elements_are![eq("pair.0.name")]);
}

// ── An unregistered newtype is not guessed at (decisions N and W) ────────

/// No registered field, no `field:`.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Headline(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Story {
    headline: Headline,
}

/// The type is named, with the ways to give it a field: the app's registry
/// for every path, `field:` for this one, and the adapter that makes either
/// one line.
#[gtest]
fn a_plain_newtype_without_a_registered_field_panics_naming_the_type() {
    install_test_registry();
    let message = panic_message(|| {
        let _ = empty_form::<Story>(FormSpec::default());
    });
    expect_that!(
        message,
        all![
            contains_substring("Headline"),
            contains_substring("TypeRegistry::field_for"),
            contains_substring("field:"),
            contains_substring("Newtype")
        ]
    );
}

/// `#[facet(transparent)]` is facet's statement that the type IS its inner
/// value, and formoxus does not act on it (W). The message says so, since a
/// dev who wrote it will expect otherwise.
#[derive(Facet, Clone, Debug, PartialEq)]
#[facet(transparent)]
struct Byline(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Column {
    byline: Byline,
}

#[gtest]
fn a_transparent_newtype_without_a_registered_field_panics_like_any_other() {
    install_test_registry();
    let message = panic_message(|| {
        let _ = empty_form::<Column>(FormSpec::default());
    });
    expect_that!(
        message,
        all![
            contains_substring("Byline"),
            contains_substring("TypeRegistry::field_for"),
            contains_substring("transparent")
        ]
    );
}

// ── Round trips ──────────────────────────────────────────────────────────

#[gtest]
fn a_registered_newtype_round_trips_through_edit_mode() {
    // Populating reads THROUGH the newtype, and writing rebuilds it.
    install_test_registry();
    let doc = Doc {
        title: "Unit 1".to_string(),
        body: Note("hello **there**".to_string()),
    };
    let mut form = form_for(&doc, FormSpec::default());
    expect_that!(form.validate(), some(eq(&doc)));
    expect_that!(across_the_wire(&doc), ok(eq(&doc)));
}

#[gtest]
fn a_populated_newtype_shows_its_inner_value_in_the_input() {
    install_test_registry();
    let doc = Doc {
        title: "Unit 1".to_string(),
        body: Note("hello".to_string()),
    };
    let form = form_for(&doc, FormSpec::default());
    expect_that!(
        value_at(&form, "body"),
        some(eq("hello")),
        "the raw value is the INNER string, not a Debug of the newtype"
    );
}

#[gtest]
fn a_newtype_builds_from_submitted_values() {
    install_test_registry();
    let mut form = empty_form::<Doc>(FormSpec::default());
    form.distribute_form_values(&[
        (NodePath::new("title"), "Unit 1".to_string()),
        (NodePath::new("body"), "typed by hand".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Doc {
            title: "Unit 1".to_string(),
            body: Note("typed by hand".to_string()),
        }))
    );
}

/// `trims` is the inner field's: a `StringField` trims, where `Field`'s
/// default would not.
#[gtest]
fn a_string_newtype_trims_as_a_string_does() {
    install_test_registry();
    let mut form = empty_form::<Doc>(FormSpec::default());
    form.distribute_form_values(&[
        (NodePath::new("title"), "Unit 1".to_string()),
        (NodePath::new("body"), "  padded  ".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Doc {
            title: "Unit 1".to_string(),
            body: Note("padded".to_string()),
        }))
    );
}

// ── The inner field's rules still apply ──────────────────────────────────

#[gtest]
fn a_numeric_newtype_parses_through_the_inner_field() {
    install_test_registry();
    let mut form = empty_form::<Tally>(FormSpec::default());
    form.distribute_form_values(&[
        (NodePath::new("label"), "Votes".to_string()),
        (NodePath::new("count"), "42".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Tally {
            label: "Votes".to_string(),
            count: Count(42)
        }))
    );
}

#[gtest]
fn a_bad_inner_value_is_rejected_rather_than_defaulted() {
    // The failure mode worth pinning: parsing goes through the INNER field,
    // so `Count("abc")` fails exactly as a bare `u32` would instead of
    // quietly becoming `Count(0)`.
    install_test_registry();
    let mut form = empty_form::<Tally>(FormSpec::default());
    form.distribute_form_values(&[
        (NodePath::new("label"), "Votes".to_string()),
        (NodePath::new("count"), "not-a-number".to_string()),
    ]);
    expect_that!(form.validate(), none());
    expect_that!(error_paths(&form), elements_are![eq("count")]);
}

#[gtest]
fn an_empty_newtype_field_is_required_like_any_other() {
    install_test_registry();
    let mut form = empty_form::<Doc>(FormSpec::default());
    form.distribute_form_values(&[(NodePath::new("title"), "Unit 1".to_string())]);
    expect_that!(form.validate(), none());
    expect_that!(error_paths(&form), elements_are![eq("body")]);
}

/// Registered with a configured inner field, `StringField { max_length: 5 }`:
/// the setting checks the value and reaches the input as `maxlength`.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Motto(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Crest {
    motto: Motto,
}

#[gtest]
fn a_registered_inner_fields_settings_check_the_value() {
    install_test_registry();
    let mut form = empty_form::<Crest>(FormSpec::default());
    form.distribute_form_values(&[(NodePath::new("motto"), "far too long".to_string())]);
    expect_that!(form.validate(), none());
    expect_that!(error_paths(&form), elements_are![eq("motto")]);

    let mut form = empty_form::<Crest>(FormSpec::default());
    form.distribute_form_values(&[(NodePath::new("motto"), "brief".to_string())]);
    expect_that!(
        form.validate(),
        some(eq(&Crest {
            motto: Motto("brief".to_string())
        }))
    );
}

#[component]
fn EmptyCrest() -> Element {
    use_form(|| empty_form::<Crest>(FormSpec::default())).render(using_fns! {})
}

#[gtest]
fn a_registered_inner_fields_settings_reach_the_input() {
    install_test_registry();
    let html = render_to_html(EmptyCrest);
    expect_that!(
        html,
        all![
            contains_substring(r#"name="motto""#),
            // An `Int` attribute renders unquoted, like `required=true`.
            contains_substring("maxlength=5")
        ]
    );
}

// ── Optional ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct MaybeDoc {
    title: String,
    body: Option<Note>,
}

#[gtest]
fn an_absent_optional_newtype_round_trips_as_none() {
    install_test_registry();
    let mut form = empty_form::<MaybeDoc>(FormSpec::default());
    form.distribute_form_values(&[(NodePath::new("title"), "Unit 1".to_string())]);
    expect_that!(
        form.validate(),
        some(eq(&MaybeDoc {
            title: "Unit 1".to_string(),
            body: None
        }))
    );
}

#[gtest]
fn a_present_optional_newtype_round_trips_as_some() {
    install_test_registry();
    let doc = MaybeDoc {
        title: "Unit 1".to_string(),
        body: Some(Note("here".to_string())),
    };
    let mut form = form_for(&doc, FormSpec::default());
    expect_that!(form.validate(), some(eq(&doc)));
    expect_that!(across_the_wire(&doc), ok(eq(&doc)));
}

// ── `Newtype` hands every question to its inner field ────────────────────
//
// Every `Field` method with a default body is the trap: one `Newtype` leaves
// out silently answers with `Field`'s default instead of the inner field's.
// `Loud` overrides each of them with something no default would say, so a
// missing delegation fails here, one method per test.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shout(String);

/// Reads text upper-cased and writes it lower-cased; refuses "BAD", and on
/// the server "TAKEN"; never trims; offers two choices; renders a textarea.
#[derive(Clone, Debug, Default, PartialEq)]
struct Loud;

impl Field for Loud {
    type Value = String;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn parse(&self, raw: &str) -> std::result::Result<String, ValidationMessage> {
        if raw == "nope" {
            Err("Loud refuses nope.".into())
        } else {
            Ok(raw.to_uppercase())
        }
    }

    fn format(&self, value: &String) -> String {
        value.to_lowercase()
    }

    fn check(&self, value: &String) -> Vec<ValidationMessage> {
        if value == "BAD" {
            vec!["Loud refuses BAD.".into()]
        } else {
            Vec::new()
        }
    }

    fn widget_attrs(&self) -> FieldAttrs {
        FieldAttrs::default().with_default(Attr::MaxLength, AttrValue::Int(9))
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        Textarea.into()
    }

    // A ready future, not an `async fn`: nothing here awaits, and the test
    // is that `Newtype` hands back the inner field's future, whatever it is.
    fn check_on_server(
        &self,
        value: &String,
        _ctx: &(),
    ) -> impl Future<Output = Vec<ValidationMessage>> + Send {
        std::future::ready(if value == "TAKEN" {
            vec!["Loud says TAKEN is taken.".into()]
        } else {
            Vec::new()
        })
    }

    fn choices(&self) -> Option<Vec<Choice>> {
        Some(vec![Choice::new("a", "A"), Choice::new("b", "B")])
    }

    fn trims(&self, _password: bool) -> bool {
        true
    }
}

fn shout() -> Newtype<Shout, Loud> {
    Newtype::new(Loud)
}

#[gtest]
fn newtype_takes_its_kind_from_the_inner_field() {
    expect_that!(<Newtype<Shout, Loud> as Field>::KIND, eq(FieldType::Text));
    expect_that!(
        <Newtype<Count, NumberField<u32>> as Field>::KIND,
        eq(FieldType::Int)
    );
}

#[gtest]
fn newtype_parses_through_the_inner_field_and_wraps() {
    expect_that!(shout().parse("hey"), ok(eq(&Shout("HEY".to_string()))));
    expect_that!(
        shout().parse("nope"),
        err(eq(&ValidationMessage::from("Loud refuses nope.")))
    );
}

#[gtest]
fn newtype_formats_through_the_inner_field() {
    expect_that!(shout().format(&Shout("HEY".to_string())), eq("hey"));
}

#[gtest]
fn newtype_checks_through_the_inner_field() {
    expect_that!(
        Field::check(&shout(), &Shout("BAD".to_string())),
        elements_are![eq(&ValidationMessage::from("Loud refuses BAD."))]
    );
    expect_that!(
        Field::check(&shout(), &Shout("FINE".to_string())),
        is_empty()
    );
}

#[gtest]
fn newtype_checks_on_the_server_through_the_inner_field() {
    expect_that!(
        block_on(shout().check_on_server(&Shout("TAKEN".to_string()), &())),
        elements_are![eq(&ValidationMessage::from("Loud says TAKEN is taken."))]
    );
}

#[gtest]
fn newtype_takes_its_attrs_from_the_inner_field() {
    expect_that!(shout().widget_attrs(), eq(&Loud.widget_attrs()));
}

#[gtest]
fn newtype_takes_its_default_widget_from_the_inner_field() {
    expect_that!(
        shout().default_widget(false),
        eq(&AnyWidget::from(Textarea))
    );
}

#[gtest]
fn newtype_takes_its_choices_from_the_inner_field() {
    expect_that!(shout().choices(), eq(&Loud.choices()));
}

#[gtest]
fn newtype_trims_as_the_inner_field_does() {
    expect_that!(shout().trims(false), eq(true));
}

// ── Pairing a newtype with a field ───────────────────────────────────────

/// The inner field must read the newtype's own field type, which only the
/// run time can see (a field's shape is out of reach of const code), so
/// `new` checks it and names both types.
#[gtest]
fn a_newtype_over_a_field_of_the_wrong_type_panics_naming_both() {
    let message = panic_message(|| {
        let _ = Newtype::<Note, NumberField<u32>>::new(NumberField::default());
    });
    expect_that!(
        message,
        all![contains_substring("Note"), contains_substring("u32")]
    );
}

/// `TypeRegistry::newtype` is `field_for` underneath, so a newtype already
/// registered panics as a second `field_for` would.
#[gtest]
fn registering_a_newtype_twice_panics_naming_the_type() {
    let message = panic_message(|| {
        let _ = TypeRegistry::new()
            .newtype::<Note, String>()
            .newtype::<Note, String>();
    });
    expect_that!(
        message,
        all![contains_substring("Note"), contains_substring("already")]
    );
}

/// And its kind is the inner field's, so a registered widget that cannot
/// render it is refused as for any registered field.
#[gtest]
fn a_registered_newtypes_kind_is_checked_against_its_widget() {
    let message = panic_message(|| {
        let _ = TypeRegistry::new()
            .newtype::<Count, u32>()
            .widget_for::<Count>(Textarea);
    });
    expect_that!(
        message,
        all![contains_substring("Count"), contains_substring("textarea")]
    );
}
