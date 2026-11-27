//! Lists of the app's types, known at compile time (`CUSTOM_FIELDS_PLAN.md`
//! decision AB, built in box 5.9).
//!
//! A crate declares its types once with `formoxus::types!`. `form!` finds the
//! list through the `FormoxusTypes` in scope, and each form carries its list's
//! run-time registry in its spec, so nothing needs installing for a form to
//! read its own types. What an app installs is only its overrides: the
//! `replace` lines of its list, which reach a library's forms too (AB.6, AF).
//!
//! The order a leaf's field is looked up in (AB.6): the path's own settings,
//! `field:` or `widget:`; then the installed overrides; then the form's own
//! list; then the built-ins, which no list can replace (AF).
//!
//! Three groups of tests:
//!
//! 1. **One list, then two,** with this module's own types: a form reads
//!    through the list it was built with, including when it builds again later
//!    (a server filling it from the wire, an item added, a variant chosen).
//! 2. **A library** (`formoxus-fixture-lib`, decision AE), whose forms bring
//!    their own list, and which an app's list can include.
//! 3. **Overrides,** through `super::install_overrides`. `install` is once per
//!    process, so every test in this group calls it first and reads only the
//!    fixture's `Stamp` and `Seal`, which nothing else in the suite builds.

use super::{across_the_wire_with, install_overrides, panic_message};
use facet::Facet;
use formoxus::attrs::FieldType;
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::prelude::*;
use formoxus::widgets::{AnyWidget, TextInput};
use formoxus_fixture_lib::{Badge, Card, Seal, SealedCard, Stamp};
use googletest::prelude::*;
use std::collections::HashSet;

// ── This module's types ──────────────────────────────────────────────────

/// A newtype this module lists. Never in the suite's overrides, so a test of
/// it reads the same whether or not they are installed.
#[derive(Facet, Clone, Debug, PartialEq, Eq, Hash)]
struct Shade(String);

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Mood {
    Calm,
    Bold(Shade),
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Palette {
    name: String,
    shade: Shade,
    shades: Vec<Shade>,
    mood: Mood,
}

fn palette() -> Palette {
    Palette {
        name: "Sea".to_string(),
        shade: Shade("teal".to_string()),
        shades: vec![Shade("navy".to_string())],
        mood: Mood::Bold(Shade("coral".to_string())),
    }
}

/// A field for `Shade` that writes it upper case and reads it back lower
/// case, so which field a form used shows in both directions.
#[derive(Clone, Debug, Default, PartialEq)]
struct ShoutShade;

impl Field for ShoutShade {
    type Value = Shade;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn format(&self, value: &Shade) -> String {
        value.0.to_uppercase()
    }

    fn parse(&self, raw: &str) -> Result<Shade, ValidationMessage> {
        Ok(Shade(raw.to_lowercase()))
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

/// A struct this module lists, read as one number: what a listed type whose
/// shape is a struct, not a newtype, gets from `form!`'s checks.
#[derive(Facet, Clone, Debug, PartialEq, Eq, Hash)]
struct Tone {
    level: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ToneField;

impl Field for ToneField {
    type Value = Tone;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn format(&self, value: &Tone) -> String {
        value.level.to_string()
    }

    fn parse(&self, raw: &str) -> Result<Tone, ValidationMessage> {
        raw.parse()
            .map(|level| Tone { level })
            .map_err(|_| "not a level".into())
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Mixer {
    tone: Tone,
    tones: Vec<Tone>,
}

formoxus::types! {
    struct PlainTypes;
    Shade => newtype String,
    Tone => field ToneField,
}

/// This module's forms read `PlainTypes`, through `form!` as through a spec
/// built with `with_types`.
use PlainTypes as FormoxusTypes;

/// A second list, in a module of its own, where `Shade` shouts.
mod loud {
    use super::{Shade, ShoutShade};

    formoxus::types! {
        pub(super) struct LoudTypes;
        Shade => field ShoutShade,
    }
}
use loud::LoudTypes;

fn plain<T: Clone + std::fmt::Debug + PartialEq + Facet<'static>>() -> FormSpec<T> {
    FormSpec::default().with_types(PlainTypes::registry())
}

fn loud<T: Clone + std::fmt::Debug + PartialEq + Facet<'static>>() -> FormSpec<T> {
    FormSpec::default().with_types(LoudTypes::registry())
}

fn value_at<T>(form: &FormState<T>, path: &str) -> Option<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    form.collect_values()
        .into_iter()
        .find_map(|(p, v)| (p == path).then_some(v))
}

fn paths_of<T>(form: &FormState<T>) -> Vec<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    form.collect_values()
        .into_iter()
        .map(|(p, _)| p.to_string())
        .collect()
}

/// What a server sees: an empty form, filled from the wire's values.
fn rebuilt<T>(values: &[(&str, &str)], spec: FormSpec<T>) -> FormState<T>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    let mut form = empty_form(spec);
    form.distribute_form_values(
        &values
            .iter()
            .map(|(p, v)| {
                (
                    formoxus::path::NodePath::new(p.to_string()),
                    (*v).to_string(),
                )
            })
            .collect::<Vec<_>>(),
    );
    form
}

// ── One list ─────────────────────────────────────────────────────────────

#[gtest]
fn a_listed_newtype_is_one_leaf_wherever_it_sits() {
    let form = empty_form::<Palette>(plain());
    expect_that!(
        paths_of(&form),
        elements_are![eq("name"), eq("shade"), eq("shades"), eq("mood")],
        "a newtype must not produce `shade.0`"
    );
}

#[gtest]
fn a_form_reads_and_writes_through_its_list() {
    let form = form_for(&palette(), plain());
    expect_that!(value_at(&form, "shade"), some(eq("teal")));
    expect_that!(value_at(&form, "shades.#0"), some(eq("navy")));
    expect_that!(value_at(&form, "mood.$Bold.0"), some(eq("coral")));
    expect_that!(
        across_the_wire_with(PlainTypes::registry(), &palette()),
        ok(eq(&palette()))
    );
}

/// A listed struct is one value, judged by its field's `KIND` (AB.10.3): a
/// text widget and a text attribute compile on it, and so does `set` on a
/// list of them, where an unlisted struct gets compile errors
/// (`form_widget_on_a_struct`, `form_constraint_on_a_struct`,
/// `form_set_on_a_list_of_structs`). Building it proves the run-time `set`
/// check asks the spec's list too.
#[gtest]
fn a_listed_struct_takes_a_widget_an_attribute_and_set() {
    let spec = form! {
        Mixer {
            tone => { widget: textarea, placeholder: "0 to 255" },
            tones => { set, sort_by: |a: &Tone, b: &Tone| a.level.cmp(&b.level) },
        }
    };
    let mixer = Mixer {
        tone: Tone { level: 7 },
        tones: vec![Tone { level: 3 }],
    };
    let form = form_for(&mixer, spec);
    expect_that!(value_at(&form, "tone"), some(eq("7")));
    expect_that!(
        form.collect_values()
            .into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>(),
        contains(eq("3"))
    );
}

/// A real set of a struct, for 5.6.3.2's run-time check on a set's items.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Chord {
    tones: HashSet<Tone>,
}

fn chord() -> Chord {
    Chord {
        tones: HashSet::from([Tone { level: 3 }]),
    }
}

/// A struct has no default order, so every `Chord` spec that builds needs one.
fn by_level() -> formoxus::nodes::SetOrder {
    formoxus::nodes::SetOrder::custom(|a: &Tone, b: &Tone| a.level.cmp(&b.level), "level")
}

/// A struct item no list holds is a group of fields, so a set of one needs
/// only that each field can be built. A field that cannot is refused when the
/// form is built, even with the set empty, because the set always builds its
/// draft, whose walk meets the field. The message names the field's own path,
/// which a check on the set as a whole could not (5.6.3.2, found not needed).
#[derive(Facet, Clone, Debug, PartialEq, Eq, Hash)]
struct Spot {
    x: usize,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Board {
    spots: HashSet<Spot>,
}

#[gtest]
fn a_set_item_with_a_field_formoxus_cannot_build_is_refused_naming_the_field() {
    let message = panic_message(|| {
        let _ = empty_form::<Board>(FormSpec::default().with_sort_by(
            &path!(Board.spots),
            formoxus::nodes::SetOrder::custom(|a: &Spot, b: &Spot| a.x.cmp(&b.x), "x"),
        ));
    });
    expect_that!(
        message,
        all![
            starts_with("spots.#0.x: "),
            contains_substring("`usize`"),
            contains_substring("formoxus::types!")
        ]
    );
}

/// The same set builds when the spec's list gives `Tone` its field: each
/// item is one leaf, written through `ToneField`.
#[gtest]
fn a_set_of_a_listed_struct_builds_each_item_as_one_leaf() {
    let form = form_for(
        &chord(),
        plain().with_sort_by(&path!(Chord.tones), by_level()),
    );
    expect_that!(value_at(&form, "tones.#0"), some(eq("3")));
}

/// And it builds when the items' own path names a field, with no list
/// holding `Tone`: the constructor arm builds each item as a leaf whatever
/// its type, so the check must not refuse it.
#[gtest]
fn a_set_whose_items_name_a_field_builds_with_no_list() {
    let spec = FormSpec::<Chord>::default()
        .with_sort_by(&path!(Chord.tones), by_level())
        .with_node(
            &path!(Chord.tones[]),
            formoxus::field_kind::named_field(|_: &Chord| -> &Tone { unreachable!() }, ToneField),
        );
    let form = form_for(&chord(), spec);
    expect_that!(value_at(&form, "tones.#0"), some(eq("3")));
}

/// 5.6.3.3: `form!` refuses a widget that cannot render a listed type's
/// field at compile time (`form_widget_cannot_render_a_listed_type`); a spec
/// built by hand gets the same refusal when the form is built, naming the
/// path, the widget and what the field reads. `Tone` reads as text, and a
/// checkbox renders only a `bool`.
#[gtest]
fn a_hand_built_widget_that_cannot_render_a_listed_types_field_is_refused() {
    let message = panic_message(|| {
        let _ = empty_form::<Mixer>(
            plain().with_widget(&path!(Mixer.tone), formoxus::widgets::CheckboxInput),
        );
    });
    expect_that!(
        message,
        all![
            starts_with("tone: "),
            contains_substring("`checkbox`"),
            contains_substring("`Tone`"),
            contains_substring("reads Text")
        ]
    );
}

/// A widget that takes choices renders any field that has them, and only
/// then: a `select` on a `Tone` with no choices is refused like the checkbox.
#[gtest]
fn a_choice_widget_on_a_listed_type_with_no_choices_is_refused() {
    let message = panic_message(|| {
        let _ =
            empty_form::<Mixer>(plain().with_widget(&path!(Mixer.tone), formoxus::widgets::Select));
    });
    expect_that!(
        message,
        all![starts_with("tone: "), contains_substring("`select`")]
    );
}

/// The attribute half: `step` applies to a number, and `Tone` reads as text,
/// so a spec built by hand that gives it one is refused, naming the path, the
/// attribute and what the field reads. A quoted key is not in the table, so
/// nothing is asked about it.
#[gtest]
fn a_hand_built_attribute_that_does_not_apply_to_a_listed_types_field_is_refused() {
    use formoxus::attrs::{Attr, AttrKey, AttrValue, Step};
    let message = panic_message(|| {
        let _ = empty_form::<Mixer>(plain().with_attrs(
            &path!(Mixer.tone),
            formoxus::field_attrs::FieldAttrs::from([
                (
                    AttrKey::NonStd("data-x"),
                    AttrValue::String("1".to_string()),
                ),
                (AttrKey::Std(Attr::Step), AttrValue::Step(Step::Any)),
            ]),
        ));
    });
    expect_that!(
        message,
        all![
            starts_with("tone: "),
            contains_substring("`step`"),
            contains_substring("`Tone`"),
            contains_substring("reads Text")
        ]
    );
}

#[gtest]
fn a_hand_built_attribute_that_applies_to_a_listed_types_field_builds() {
    use formoxus::attrs::{Attr, AttrKey, AttrValue};
    let _ = empty_form::<Mixer>(plain().with_attrs(
        &path!(Mixer.tone),
        formoxus::field_attrs::FieldAttrs::from([
            (
                AttrKey::NonStd("data-x"),
                AttrValue::String("1".to_string()),
            ),
            (
                AttrKey::Std(Attr::Placeholder),
                AttrValue::String("0 to 255".to_string()),
            ),
        ]),
    ));
}

/// A built-in type is checked the same way (decision V, amended
/// 2026-11-27): a checkbox on a `String` is refused when the form is built,
/// as `form!` refuses it at compile time.
#[gtest]
fn a_hand_built_widget_on_a_built_in_type_is_checked_too() {
    let message = panic_message(|| {
        let _ = empty_form::<Palette>(
            plain().with_widget(&path!(Palette.name), formoxus::widgets::CheckboxInput),
        );
    });
    expect_that!(
        message,
        all![
            starts_with("name: "),
            contains_substring("`checkbox`"),
            contains_substring("`String`")
        ]
    );
}

/// AF at run time: a hand-built registry cannot give a built-in type a field
/// either. `types!` refuses the same entry at compile time
/// (`types_built_in_entry`); this is `field_for`'s own refusal, which says
/// what to do instead.
#[gtest]
fn field_for_refuses_a_built_in_type() {
    let message = panic_message(|| {
        let _ = formoxus::type_registry::TypeRegistry::new()
            .field_for(formoxus::fields::StringField::default());
    });
    expect_that!(
        message,
        all![
            contains_substring("`String` is built in"),
            contains_substring("struct Shouty(String);")
        ]
    );
}

/// `form!` takes the list from the `FormoxusTypes` in scope, here
/// `PlainTypes`, with no `with_types` written.
#[gtest]
fn form_reads_the_formoxus_types_in_scope() {
    let form = form_for(&palette(), form! { Palette { name => { max_length: 10 } } });
    expect_that!(value_at(&form, "shade"), some(eq("teal")));
}

/// A module with no list of its own gets the prelude's `FormoxusTypes`, the
/// built-ins (decision AC): a form over built-in types needs nothing.
mod no_list_of_its_own {
    use facet::Facet;
    use formoxus::prelude::*;

    #[derive(Facet, Clone, Debug, PartialEq)]
    pub(super) struct Plain {
        pub(super) name: String,
        pub(super) count: u8,
    }

    pub(super) fn spec() -> FormSpec<Plain> {
        form! { Plain { name => { max_length: 3 } } }
    }
}

#[gtest]
fn a_module_with_no_list_gets_the_built_ins() {
    let plain = no_list_of_its_own::Plain {
        name: "Ada".to_string(),
        count: 3,
    };
    let mut form = form_for(&plain, no_list_of_its_own::spec());
    expect_that!(form.validate(), some(eq(&plain)));
}

/// A spec built by hand with no list sees the installed overrides and the
/// built-ins (decision AD), and `Shade` is in neither.
#[gtest]
fn a_spec_with_no_list_refuses_a_listed_type() {
    let message = panic_message(|| {
        let _ = empty_form::<Palette>(FormSpec::default());
    });
    expect_that!(message, contains_substring("Shade"));
}

// ── Two lists in one process ─────────────────────────────────────────────

/// Nothing global decides how `Shade` reads: each form has its own list.
#[gtest]
fn two_lists_give_one_type_different_fields_in_one_process() {
    let plain_form = form_for(&palette(), plain());
    let loud_form = form_for(&palette(), loud());
    expect_that!(value_at(&plain_form, "shade"), some(eq("teal")));
    expect_that!(value_at(&loud_form, "shade"), some(eq("TEAL")));
    expect_that!(value_at(&loud_form, "shades.#0"), some(eq("NAVY")));
}

/// The server fills an empty form from the wire, which builds the list's
/// items after the first walk. They must come through the form's list: with
/// any other, `NAVY` would stay upper case or `shades.#0` would not be one
/// leaf.
#[gtest]
fn a_rebuild_builds_list_items_through_the_forms_list() {
    let mut form = rebuilt(
        &[
            ("name", "Sea"),
            ("shade", "TEAL"),
            ("shades", "#0"),
            ("shades.#0", "NAVY"),
            ("mood", "Calm"),
        ],
        loud::<Palette>(),
    );
    expect_that!(
        form.validate(),
        some(eq(&Palette {
            name: "Sea".to_string(),
            shade: Shade("teal".to_string()),
            shades: vec![Shade("navy".to_string())],
            mood: Mood::Calm,
        }))
    );
}

/// The same for a variant chosen after the first walk, whose fields are built
/// only then.
#[gtest]
fn a_rebuild_builds_a_chosen_variants_fields_through_the_forms_list() {
    let mut form = rebuilt(
        &[
            ("name", "Sea"),
            ("shade", "TEAL"),
            ("shades", ""),
            ("mood", "Bold"),
            ("mood.$Bold.0", "CORAL"),
        ],
        loud::<Palette>(),
    );
    expect_that!(
        form.validate(),
        some(eq(&Palette {
            name: "Sea".to_string(),
            shade: Shade("teal".to_string()),
            shades: Vec::new(),
            mood: Mood::Bold(Shade("coral".to_string())),
        }))
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Swatches {
    shades: HashSet<Shade>,
}

/// `path!(…[= v])` writes the item's text through the `FormoxusTypes` in
/// scope, as the form's node does, so the key matches the item. In `loud`'s
/// list `Shade("navy")` is written `NAVY`, which only that list's field knows.
mod keyed {
    use super::{Shade, Swatches};
    use formoxus::prelude::*;

    use super::LoudTypes as FormoxusTypes;

    pub(super) fn no_navy(s: &Swatches) -> Vec<ValidationError<Swatches>> {
        let navy = Shade("navy".to_string());
        if s.shades.contains(&navy) {
            vec![ValidationError::at(
                path!(Swatches.shades[= &navy]),
                "Not navy.",
            )]
        } else {
            Vec::new()
        }
    }

    pub(super) fn spec() -> FormSpec<Swatches> {
        form! { Swatches { shades => { sort_by: |a: &Shade, b: &Shade| a.0.cmp(&b.0) } } }
            .with_validator(no_navy)
    }
}

#[gtest]
fn a_set_item_is_found_by_its_value_through_the_forms_list() {
    let swatches = Swatches {
        shades: [Shade("navy".to_string()), Shade("teal".to_string())].into(),
    };
    let mut form = form_for(&swatches, keyed::spec());
    expect_that!(form.validate(), none());
    let at = form
        .collect_values()
        .into_iter()
        .find(|(_, value)| value == "NAVY")
        .map(|(path, _)| path)
        .expect("navy is an item, written through the loud list");
    expect_that!(
        form.collect_errors().fields.get(&at),
        some(elements_are![eq(&ValidationMessage::from("Not navy."))])
    );
}

// ── A library's types and forms ──────────────────────────────────────────

fn card() -> Card {
    Card {
        title: "Hello".to_string(),
        badge: Badge("gold".to_string()),
        stamp: Stamp("ink".to_string()),
        pinned: Stamp("wax".to_string()),
    }
}

/// The library's form carries the library's list: it builds here, in a crate
/// whose own lists never mention `Badge`, with nothing included.
#[gtest]
fn a_librarys_form_brings_its_own_types() {
    let form = form_for(&card(), formoxus_fixture_lib::forms::card());
    expect_that!(value_at(&form, "badge"), some(eq("gold")));
    expect_that!(
        paths_of(&form),
        elements_are![eq("title"), eq("badge"), eq("stamp"), eq("pinned")]
    );
}

formoxus::types! {
    struct WithFixture;
    include formoxus_fixture_lib::FixtureTypes;
    Shade => newtype String,
}

/// Including a library brings its entries, so this crate's own forms read
/// its types too, checked at compile time like any of this crate's own.
mod including {
    use formoxus::prelude::*;
    use formoxus_fixture_lib::Card;

    use super::WithFixture as FormoxusTypes;

    pub(super) fn spec() -> FormSpec<Card> {
        form! { Card { badge => { label: "Badge" } } }
    }
}

#[gtest]
fn a_list_that_includes_a_library_reads_its_types() {
    let form = form_for(&card(), including::spec());
    expect_that!(value_at(&form, "badge"), some(eq("gold")));
    expect_that!(
        across_the_wire_with(WithFixture::registry(), &card()),
        ok(eq(&card()))
    );
}

// ── Overrides: the installed `replace` lines ─────────────────────────────

/// The suite's override for the fixture's `Stamp`: upper case out, lower case
/// in, like `ShoutShade`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LoudStamp;

impl Field for LoudStamp {
    type Value = Stamp;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn format(&self, value: &Stamp) -> String {
        value.0.to_uppercase()
    }

    fn parse(&self, raw: &str) -> Result<Stamp, ValidationMessage> {
        Ok(Stamp(raw.to_lowercase()))
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

/// The suite's own entry for the fixture's `Seal`, in a list that does not
/// include `Seal`'s: an entry, not a `replace`, so never installed (AF.3).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LoudSeal;

impl Field for LoudSeal {
    type Value = Seal;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn format(&self, value: &Seal) -> String {
        value.0.to_uppercase()
    }

    fn parse(&self, raw: &str) -> Result<Seal, ValidationMessage> {
        Ok(Seal(raw.to_lowercase()))
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

/// An app's `replace` reaches a library's form, on a path the form left to
/// the types.
#[gtest]
fn an_installed_replace_reaches_a_librarys_form() {
    install_overrides();
    let form = form_for(&card(), formoxus_fixture_lib::forms::card());
    expect_that!(value_at(&form, "stamp"), some(eq("INK")));
}

/// It never reaches a path the form configured: `pinned` names its own field
/// in the library's form (AB.7).
#[gtest]
fn an_installed_replace_never_reaches_a_path_the_form_configured() {
    install_overrides();
    let form = form_for(&card(), formoxus_fixture_lib::forms::card());
    expect_that!(value_at(&form, "pinned"), some(eq("pinned:wax")));
}

/// The overrides come before a form's own list, so they reach this crate's
/// forms too, whatever list those were built with.
#[gtest]
fn an_installed_replace_beats_a_forms_own_list() {
    install_overrides();
    let form = form_for(&card(), including::spec());
    expect_that!(value_at(&form, "stamp"), some(eq("INK")));
    expect_that!(
        across_the_wire_with(WithFixture::registry(), &card()),
        ok(eq(&card()))
    );
}

/// A library type the app did not replace keeps the library's field, even
/// with the overrides installed.
#[gtest]
fn a_type_the_app_did_not_replace_keeps_its_lists_field() {
    install_overrides();
    let form = form_for(&card(), formoxus_fixture_lib::forms::card());
    expect_that!(value_at(&form, "badge"), some(eq("gold")));
}

/// The suite's list gives `Seal` an entry of its own, which is not a
/// `replace`, so it is not installed and the library's form is untouched.
#[gtest]
fn an_entry_that_is_not_a_replace_is_not_installed() {
    install_overrides();
    let sealed = SealedCard {
        seal: Seal("wax".to_string()),
    };
    let form = form_for(&sealed, formoxus_fixture_lib::forms::sealed::card());
    expect_that!(value_at(&form, "seal"), some(eq("wax")));
}

/// One process, one install: a second is refused (decision M).
#[gtest]
#[should_panic(expected = "installed")]
fn installing_a_second_list_panics() {
    install_overrides();
    formoxus::type_registry::TypeRegistry::new().install();
}
