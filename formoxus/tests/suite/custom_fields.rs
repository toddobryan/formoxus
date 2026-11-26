//! Custom fields (`CUSTOM_FIELDS_PLAN.md`, issue #28): a type the app gives a
//! field, a path that names its own field, and a type the app gives a widget.
//!
//! Box 2 wrote these against a `formoxus::proxy` on the type; box 5.2
//! (2026-11-25) rewrote them against the registry once Todd ruled out
//! formoxus-specific attributes on models; box 5.9 (2026-11-26) moved them to a
//! `types!` list (decision AB). The three declarations, each kept where the
//! thing it needs is visible:
//!
//! 1. **The crate lists a type's field**, `TimeLimit => field TimeLimitField`
//!    in this module's `types!`. Found by `Shape`, so it works in a list item,
//!    in a variant and inside an `Option`, and on the server, because each
//!    form carries its list in its spec. `form!` checks a listed type at
//!    compile time through `FormoxusTypes`.
//! 2. **A path declares its field**, `field: X` in `form!`. It beats the
//!    list: a path is more specific than a type (decision H).
//! 3. **A type's widget**, section 3. Box 6 redesigns it as a `types!` entry,
//!    so its tests are ignored until then (decision AH), with their bodies
//!    as box 5.2 wrote them.
//!
//! A newtype is no exception (decision W): formoxus ignores
//! `#[facet(transparent)]`, so section 3's `Note` and `Secret` are plain
//! newtypes, listed as `Note => newtype String`.
//!
//! Nothing here is installed. A test that needs a type the list does NOT know
//! uses a type of its own, never one listed there.

use super::{FORM_ID, across_the_wire_with, panic_message, render_to_html};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::{AttrSet, FieldType, FieldTypeSet, Tag};
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::path::NodePath;
use formoxus::prelude::*;
use formoxus::stores::ValuesByPath;
use formoxus::type_registry::TypeRegistry;
use formoxus::widgets::{
    AnyWidget, LeafWidget, NumberInput, TextInput, Textarea, Widget, WidgetProps,
};
use googletest::prelude::*;
use std::marker::PhantomData;

// This module's types (`CUSTOM_FIELDS_PLAN.md` decision AB): its forms
// read them through `FormoxusTypes`, and a spec built by hand takes them
// with `with_types`. Nothing is installed.
formoxus::types! {
    struct CustomTypes;
    TimeLimit => field TimeLimitField,
    Topic => field TopicField,
    Note => newtype String,
    Remark => newtype String,
    Secret => newtype String,
    Ref<Question> => field RefField<Question>,
}
use CustomTypes as FormoxusTypes;

// ── Helpers ──────────────────────────────────────────────────────────────

/// Every value the form would send, as `(path, raw text)`.
fn values_of<T>(form: &FormState<T>) -> Vec<(String, String)>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    form.collect_values()
        .into_iter()
        .map(|(p, v)| (p.to_string(), v))
        .collect()
}

fn paths_of<T>(form: &FormState<T>) -> Vec<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    values_of(form).into_iter().map(|(p, _)| p).collect()
}

/// The raw text the form would send at `path`, if it sends any.
fn value_at<T>(form: &FormState<T>, path: &str) -> Option<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    values_of(form)
        .into_iter()
        .find_map(|(p, v)| (p == path).then_some(v))
}

fn typed(pairs: &[(&str, &str)]) -> Vec<(NodePath, String)> {
    pairs
        .iter()
        .map(|(k, v)| (NodePath::new(k.to_string()), v.to_string()))
        .collect()
}

fn wire(pairs: &[(&str, &str)]) -> ValuesByPath {
    typed(pairs).into_iter().collect()
}

/// The messages at `path`, or none.
fn errors_at<T>(form: &FormState<T>, path: &str) -> Vec<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    refused_at(&form.collect_errors(), path)
}

fn refused_at(errors: &FormErrors, path: &str) -> Vec<String> {
    errors
        .fields
        .get(path)
        .map(|errors| errors.iter().map(|e| e.0.clone()).collect())
        .unwrap_or_default()
}

// ═════════════════════════════════════════════════════════════════════════
// 1. A type the app gives a field: `TypeRegistry::field_for`
// ═════════════════════════════════════════════════════════════════════════

/// A time limit held in seconds and typed as `1h15m`: the case from #28 and
/// apcsp's assignment settings. A plain newtype over a `u32`, with nothing on
/// it for formoxus: its text form is its registered field's.
#[derive(Facet, Clone, Debug, PartialEq)]
struct TimeLimit(u32);

/// The field's own message, which the user sees as written.
fn not_a_time(text: &str) -> String {
    format!("expected a time like 1h15m, not {text:?}")
}

#[derive(Clone, Debug, Default, PartialEq)]
struct TimeLimitField;

impl Field for TimeLimitField {
    type Value = TimeLimit;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    /// `1h15m`, `1h` or `15m`. A bare number is refused: `90` could be
    /// minutes or seconds, which is the ambiguity the format exists to avoid.
    fn parse(&self, text: &str) -> std::result::Result<TimeLimit, ValidationMessage> {
        let refuse = || ValidationMessage(not_a_time(text));
        let (hours, rest) = match text.split_once('h') {
            Some((h, rest)) => (Some(h), rest),
            None => (None, text),
        };
        let minutes = match rest {
            "" => None,
            m => Some(m.strip_suffix('m').ok_or_else(refuse)?),
        };
        if hours.is_none() && minutes.is_none() {
            return Err(refuse());
        }
        let number = |part: Option<&str>| -> std::result::Result<u32, ValidationMessage> {
            part.map_or(Ok(0), |p| p.parse().map_err(|_| refuse()))
        };
        Ok(TimeLimit(number(hours)? * 3600 + number(minutes)? * 60))
    }

    fn format(&self, limit: &TimeLimit) -> String {
        let (h, m) = (limit.0 / 3600, limit.0 % 3600 / 60);
        match (h, m) {
            (0, m) => format!("{m}m"),
            (h, 0) => format!("{h}h"),
            (h, m) => format!("{h}h{m}m"),
        }
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    title: String,
    limit: TimeLimit,
}

fn quiz() -> Quiz {
    Quiz {
        title: "Unit 1".to_string(),
        limit: TimeLimit(4500),
    }
}

// ── One leaf, not a fieldset or a newtype ────────────────────────────────

#[gtest]
fn a_registered_type_is_one_leaf_at_its_own_path() {
    let form = empty_form::<Quiz>(FormSpec::default().with_types(CustomTypes::registry()));
    expect_that!(
        paths_of(&form),
        elements_are![eq("title"), eq("limit")],
        "a tuple struct with a registered field must not become `limit.0`"
    );
}

#[component]
fn EmptyQuiz() -> Element {
    use_form(|| empty_form::<Quiz>(FormSpec::default().with_types(CustomTypes::registry())))
        .render(using_fns! {})
}

#[gtest]
fn a_registered_type_renders_its_fields_default_widget() {
    // A text input, not a number input: the inner `u32` is not what is typed.
    let html = render_to_html(EmptyQuiz);
    expect_that!(
        html,
        contains_regex(format!(
            r#"<input class="fx-control fx-input" type="text" id="{FORM_ID}-limit" name="limit""#
        ))
    );
    expect_that!(html, not(contains_substring(r#"type="number""#)));
}

// ── Parsing through the registered field ─────────────────────────────────

#[gtest]
fn a_registered_type_parses_through_its_field() {
    let mut form = empty_form::<Quiz>(FormSpec::default().with_types(CustomTypes::registry()));
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "1h15m")]));
    expect_that!(form.validate(), some(eq(&quiz())));
}

#[gtest]
fn a_registered_type_refuses_bad_text_with_its_fields_own_message() {
    let mut form = empty_form::<Quiz>(FormSpec::default().with_types(CustomTypes::registry()));
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "90")]));
    expect_that!(form.validate(), none());
    expect_that!(
        errors_at(&form, "limit"),
        elements_are![eq(&not_a_time("90"))],
        "the field's message exactly"
    );
}

#[gtest]
fn the_inner_values_text_form_is_refused() {
    // `"4500"` is what the walk would take if it still unwrapped a newtype
    // over `u32` (decision N retires that). The registered field's text form
    // replaces the inner type's; it does not add to it.
    let mut form = empty_form::<Quiz>(FormSpec::default().with_types(CustomTypes::registry()));
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "4500")]));
    expect_that!(form.validate(), none());
    expect_that!(
        errors_at(&form, "limit"),
        elements_are![eq(&not_a_time("4500"))]
    );
}

#[gtest]
fn an_empty_registered_field_is_required_like_any_other() {
    // `""` is absence (`facet_form_design_decisions.md`), so it never reaches
    // the field's `parse`: its message would be the wrong one here.
    let mut form = empty_form::<Quiz>(FormSpec::default().with_types(CustomTypes::registry()));
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "")]));
    expect_that!(form.validate(), none());
    let errors = errors_at(&form, "limit");
    expect_that!(errors, len(eq(1)));
    expect_that!(errors, not(contains(eq(&not_a_time("")))));
}

// ── Refilling from an existing value ─────────────────────────────────────

#[gtest]
fn a_registered_type_refills_its_input_through_its_fields_format() {
    let form = form_for(
        &quiz(),
        FormSpec::default().with_types(CustomTypes::registry()),
    );
    expect_that!(
        value_at(&form, "limit"),
        some(eq("1h15m")),
        "the field's text, not the inner `4500`"
    );
}

#[component]
fn FilledQuiz() -> Element {
    use_form(|| {
        form_for(
            &quiz(),
            FormSpec::default().with_types(CustomTypes::registry()),
        )
    })
    .render(using_fns! {})
}

#[gtest]
fn a_refilled_registered_input_shows_its_fields_text() {
    let html = render_to_html(FilledQuiz);
    expect_that!(html, contains_substring(r#"name="limit" value="1h15m""#));
}

#[gtest]
fn a_registered_type_round_trips_in_memory_and_across_the_wire() {
    let mut form = form_for(
        &quiz(),
        FormSpec::default().with_types(CustomTypes::registry()),
    );
    expect_that!(form.validate(), some(eq(&quiz())));
    expect_that!(
        across_the_wire_with(CustomTypes::registry(), &quiz()),
        ok(eq(&quiz()))
    );
}

// ── On the server ────────────────────────────────────────────────────────

/// No runtime and no Dioxus context: only the process-wide registry, which a
/// server function sees because the server runs the same `main` (5.1).
#[gtest]
fn submission_accept_parses_a_registered_type_from_raw_wire_text() {
    let accepted = Submission::<Quiz>::accept(
        FormSpec::default().with_types(CustomTypes::registry()),
        &wire(&[("title", "Unit 1"), ("limit", "1h15m")]),
    )
    .map(Submission::into_model);
    expect_that!(accepted, ok(eq(&quiz())));
}

#[gtest]
fn submission_accept_refuses_a_registered_type_with_its_fields_own_message() {
    let refused = Submission::<Quiz>::accept(
        FormSpec::default().with_types(CustomTypes::registry()),
        &wire(&[("title", "Unit 1"), ("limit", "90")]),
    )
    .expect_err("90 is not a time like 1h15m");
    expect_that!(
        refused_at(&refused, "limit"),
        elements_are![eq(&not_a_time("90"))]
    );
}

// ── Optional ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct MaybeTimed {
    title: String,
    limit: Option<TimeLimit>,
}

#[gtest]
fn an_empty_optional_registered_field_is_none() {
    // Trap 6: `Option<TimeLimit>` peels to the registered leaf, so the
    // registry arm comes after the `Option` arm.
    let mut form =
        empty_form::<MaybeTimed>(FormSpec::default().with_types(CustomTypes::registry()));
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "")]));
    expect_that!(
        form.validate(),
        some(eq(&MaybeTimed {
            title: "Unit 1".to_string(),
            limit: None,
        }))
    );
}

#[gtest]
fn a_filled_optional_registered_field_is_some() {
    let mut form =
        empty_form::<MaybeTimed>(FormSpec::default().with_types(CustomTypes::registry()));
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "30m")]));
    expect_that!(
        form.validate(),
        some(eq(&MaybeTimed {
            title: "Unit 1".to_string(),
            limit: Some(TimeLimit(1800)),
        }))
    );
}

#[gtest]
fn an_optional_registered_field_round_trips_both_ways() {
    for model in [
        MaybeTimed {
            title: "Unit 1".to_string(),
            limit: None,
        },
        MaybeTimed {
            title: "Unit 1".to_string(),
            limit: Some(TimeLimit(3600)),
        },
    ] {
        expect_that!(
            across_the_wire_with(CustomTypes::registry(), &model),
            ok(eq(&model))
        );
    }
}

// ── In a list item and in a variant: met by `Shape` alone ────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Stages {
    limits: Vec<TimeLimit>,
}

fn stages() -> Stages {
    Stages {
        limits: vec![TimeLimit(4500), TimeLimit(900)],
    }
}

#[gtest]
fn a_registered_list_item_is_one_leaf_per_item() {
    let form = form_for(
        &stages(),
        FormSpec::default().with_types(CustomTypes::registry()),
    );
    expect_that!(value_at(&form, "limits.#0"), some(eq("1h15m")));
    expect_that!(value_at(&form, "limits.#1"), some(eq("15m")));
    expect_that!(
        across_the_wire_with(CustomTypes::registry(), &stages()),
        ok(eq(&stages()))
    );
}

#[gtest]
fn a_registered_list_item_refuses_bad_text_on_its_own_item() {
    let mut form = form_for(
        &stages(),
        FormSpec::default().with_types(CustomTypes::registry()),
    );
    form.distribute_form_values(&typed(&[("limits.#0", "1h"), ("limits.#1", "soon")]));
    expect_that!(form.validate(), none());
    expect_that!(errors_at(&form, "limits.#0"), is_empty());
    expect_that!(
        errors_at(&form, "limits.#1"),
        elements_are![eq(&not_a_time("soon"))]
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
enum Pace {
    Untimed,
    Timed { limit: TimeLimit },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Paced {
    pace: Pace,
}

fn timed_for_an_hour() -> Paced {
    Paced {
        pace: Pace::Timed {
            limit: TimeLimit(3600),
        },
    }
}

#[gtest]
fn a_registered_field_in_a_variant_parses_and_refills() {
    let mut form = empty_form::<Paced>(FormSpec::default().with_types(CustomTypes::registry()));
    form.distribute_form_values(&typed(&[("pace", "Timed"), ("pace.$Timed.limit", "1h")]));
    expect_that!(form.validate(), some(eq(&timed_for_an_hour())));
    expect_that!(
        value_at(
            &form_for(
                &timed_for_an_hour(),
                FormSpec::default().with_types(CustomTypes::registry())
            ),
            "pace.$Timed.limit"
        ),
        some(eq("1h"))
    );
    expect_that!(
        across_the_wire_with(CustomTypes::registry(), &timed_for_an_hour()),
        ok(eq(&timed_for_an_hour()))
    );
}

#[gtest]
fn a_registered_field_in_a_variant_refuses_bad_text() {
    let mut form = empty_form::<Paced>(FormSpec::default().with_types(CustomTypes::registry()));
    form.distribute_form_values(&typed(&[("pace", "Timed"), ("pace.$Timed.limit", "90")]));
    expect_that!(form.validate(), none());
    expect_that!(
        errors_at(&form, "pace.$Timed.limit"),
        elements_are![eq(&not_a_time("90"))]
    );
}

// ── `form!` on a registered type ─────────────────────────────────────────

#[component]
fn QuizWithATextarea() -> Element {
    use_form(|| {
        empty_form(form! {
            Quiz { limit => { widget: textarea, label: "Time allowed" } }
        })
    })
    .render(using_fns! {})
}

/// O.2: `form!` cannot see the registry, so `kind` lets a widget through on a
/// plain newtype, and the build checks it against the registered field's
/// `KIND`. A textarea renders text, so this one passes.
#[gtest]
fn form_accepts_a_text_widget_on_a_registered_type() {
    let html = render_to_html(QuizWithATextarea);
    expect_that!(
        html,
        contains_substring(r#"<textarea class="fx-control fx-textarea""#)
    );
    expect_that!(html, contains_substring("Time allowed"));
}

/// A widget that renders only integers, standing in for a stepper. No
/// built-in fits: every text-like input accepts text, since the field parses
/// what is typed, and `range` needs `min:` and `max:`.
#[derive(Clone, Debug, Default, PartialEq)]
struct IntOnly;

impl Widget for IntOnly {
    const NAME: &'static str = "int_only";
    const EXTENDS: AttrSet = AttrSet::EMPTY;
    const PARTS: &'static [Tag] = &[];
}

impl LeafWidget for IntOnly {
    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Int);

    fn render(&self, _props: WidgetProps) -> Element {
        rsx! { div { class: "int-only" } }
    }
}

// The other half of O.2, a `widget:` that cannot render a listed type's field
// (`IntOnly` on a `TimeLimit` path), is a compile error under AB, since
// `form!` sees the list: `tests/ui-pending/form_widget_cannot_render_a_listed_type.rs`.

// ── `field:` beats the registry ──────────────────────────────────────────

/// A second field for `TimeLimit`, reading plain minutes, for one path.
#[derive(Clone, Debug, PartialEq)]
struct MinutesOnlyField;

impl Field for MinutesOnlyField {
    type Value = TimeLimit;
    type Context = ();
    const KIND: FieldType = FieldType::Int;

    fn parse(&self, raw: &str) -> std::result::Result<TimeLimit, ValidationMessage> {
        raw.parse::<u32>()
            .map(|minutes| TimeLimit(minutes * 60))
            .map_err(|_| ValidationMessage::from("whole minutes"))
    }

    fn format(&self, limit: &TimeLimit) -> String {
        (limit.0 / 60).to_string()
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        NumberInput.into()
    }
}

fn minutes_quiz_spec() -> FormSpec<Quiz> {
    form! { Quiz { limit => { field: MinutesOnlyField } } }
}

/// Decision H: a path is more specific than a type, so the path's field is
/// the one that parses, formats and picks the widget.
#[gtest]
fn a_named_field_beats_the_registered_field_for_its_type() {
    let mut form = empty_form(minutes_quiz_spec());
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "75")]));
    expect_that!(form.validate(), some(eq(&quiz())));
    expect_that!(
        value_at(&form_for(&quiz(), minutes_quiz_spec()), "limit"),
        some(eq("75"))
    );
}

// ── The registry's own checks (M, O.b) ───────────────────────────────────

/// Checked when the type's widget is registered, so the app fails at startup
/// rather than when a form first meets the type: `TimeLimitField` reads
/// text, and `IntOnly` renders only integers.
#[gtest]
fn registering_a_widget_that_cannot_render_the_types_field_panics() {
    let message = panic_message(|| {
        let _ = TypeRegistry::new()
            .field_for(TimeLimitField)
            .widget_for::<TimeLimit>(IntOnly);
    });
    expect_that!(
        message,
        all![
            contains_substring("TimeLimit"),
            contains_substring("int_only")
        ]
    );
}

#[gtest]
fn registering_the_field_after_a_widget_it_cannot_use_panics_too() {
    let message = panic_message(|| {
        let _ = TypeRegistry::new()
            .widget_for::<TimeLimit>(IntOnly)
            .field_for(TimeLimitField);
    });
    expect_that!(
        message,
        all![
            contains_substring("TimeLimit"),
            contains_substring("int_only")
        ]
    );
}

/// Decision B (2026-11-26): a type registered twice is a mistake, and a
/// silent replacement would hide which registration won.
#[gtest]
fn registering_a_second_field_for_a_type_panics_naming_the_type() {
    let message = panic_message(|| {
        let _ = TypeRegistry::new()
            .field_for(TimeLimitField)
            .field_for(MinutesOnlyField);
    });
    expect_that!(
        message,
        all![
            contains_substring("TimeLimit"),
            contains_substring("already")
        ]
    );
}

#[gtest]
fn registering_a_second_widget_for_a_type_panics_naming_the_type() {
    let message = panic_message(|| {
        let _ = TypeRegistry::new()
            .widget_for::<Topic>(Textarea)
            .widget_for::<Topic>(TextInput);
    });
    expect_that!(
        message,
        all![contains_substring("Topic"), contains_substring("already")]
    );
}

/// One field and one widget for the same type is not a duplicate.
#[gtest]
fn a_field_and_a_widget_for_the_same_type_are_not_duplicates() {
    let _ = TypeRegistry::new()
        .field_for(TopicField)
        .widget_for::<Topic>(Textarea);
}

// ── A registered field AND a registered widget for one type ──────────────

/// A plain newtype the registry gives both a field (whose default widget is a
/// text input) and a widget (a textarea).
#[derive(Facet, Clone, Debug, PartialEq)]
struct Topic(String);

#[derive(Clone, Debug, Default, PartialEq)]
struct TopicField;

impl Field for TopicField {
    type Value = Topic;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn parse(&self, raw: &str) -> std::result::Result<Topic, ValidationMessage> {
        Ok(Topic(raw.to_string()))
    }

    fn format(&self, topic: &Topic) -> String {
        topic.0.clone()
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Course {
    topic: Topic,
}

#[component]
fn TopicCourse() -> Element {
    use_form(|| empty_form::<Course>(FormSpec::default().with_types(CustomTypes::registry())))
        .render(using_fns! {})
}

/// Decision H's order among the two the registry holds: the type's widget
/// beats its field's `default_widget`.
#[gtest]
#[ignore = "box 6: widget entries in types!"]
fn the_registered_widget_beats_the_registered_fields_default_widget() {
    let html = render_to_html(TopicCourse);
    expect_that!(html, contains_regex(r#"<textarea [^>]*name="topic""#));
    let course = Course {
        topic: Topic("loops".to_string()),
    };
    expect_that!(
        across_the_wire_with(CustomTypes::registry(), &course),
        ok(eq(&course))
    );
}

// ═════════════════════════════════════════════════════════════════════════
// 2. A path that names its own field: `field:` in `form!`
// ═════════════════════════════════════════════════════════════════════════

/// The design note's example: a text field whose text must "compile". Here,
/// balanced parentheses and at most `max_lines` lines, a setting the dev
/// passes in, so `field:` takes an expression rather than a type.
#[derive(Clone, Debug, PartialEq)]
struct ProgramSnippetField {
    max_lines: usize,
}

const UNBALANCED: &str = "does not compile: unbalanced parentheses";

impl Field for ProgramSnippetField {
    type Value = String;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn check(&self, code: &String) -> Vec<ValidationMessage> {
        let mut problems = Vec::new();
        let mut depth = 0i32;
        for c in code.chars() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                break;
            }
        }
        if depth != 0 {
            problems.push(ValidationMessage::from(UNBALANCED));
        }
        let lines = code.lines().count();
        if lines > self.max_lines {
            problems.push(ValidationMessage(format!(
                "at most {} lines, not {lines}",
                self.max_lines
            )));
        }
        problems
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        Textarea.into()
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Lesson {
    title: String,
    snippet: String,
}

fn lesson_spec() -> FormSpec<Lesson> {
    form! { Lesson { snippet => { field: ProgramSnippetField { max_lines: 10 } } } }
}

#[component]
fn SnippetLesson() -> Element {
    use_form(|| empty_form(lesson_spec())).render(using_fns! {})
}

#[gtest]
fn a_named_field_renders_its_own_default_widget() {
    // No `widget:`: the textarea comes from the field's `default_widget`.
    let html = render_to_html(SnippetLesson);
    expect_that!(html.matches("<textarea").count(), eq(1));
    expect_that!(html, contains_regex(r#"<textarea [^>]*name="snippet""#));
    expect_that!(html, contains_substring(r#"name="title""#));
}

#[gtest]
fn a_named_fields_check_runs_in_the_page() {
    let mut form = empty_form(lesson_spec());
    form.distribute_form_values(&typed(&[("title", "Loops"), ("snippet", "print((1)")]));
    expect_that!(form.validate(), none());
    expect_that!(errors_at(&form, "snippet"), elements_are![eq(UNBALANCED)]);
}

#[gtest]
fn a_named_fields_check_runs_on_the_server() {
    // The spec is built on both sides, so `field:` reaches `Submission::accept`.
    let refused = Submission::accept(
        lesson_spec(),
        &wire(&[("title", "Loops"), ("snippet", "print((1)")]),
    )
    .expect_err("the parentheses do not balance");
    expect_that!(
        refused_at(&refused, "snippet"),
        elements_are![eq(UNBALANCED)]
    );

    let accepted = Submission::accept(
        lesson_spec(),
        &wire(&[("title", "Loops"), ("snippet", "print(1)")]),
    );
    expect_that!(accepted.is_ok(), eq(true));
}

#[gtest]
fn a_named_fields_setting_is_an_expression_from_the_call_site() {
    // `field:` takes an expression, evaluated where `form!` is written, so a
    // setting can come from a local binding as `title:` can.
    let max_lines = 1;
    let spec = form! { Lesson { snippet => { field: ProgramSnippetField { max_lines } } } };
    let mut form = empty_form(spec);
    form.distribute_form_values(&typed(&[("title", "Loops"), ("snippet", "a()\nb()")]));
    expect_that!(form.validate(), none());
    expect_that!(
        errors_at(&form, "snippet"),
        elements_are![eq("at most 1 lines, not 2")]
    );
}

#[gtest]
fn a_named_fields_default_format_refills_the_stored_text() {
    // `ProgramSnippetField` writes no `format`, so the default (facet's
    // display, as `raw_value` does today) refills the input.
    let lesson = Lesson {
        title: "Loops".to_string(),
        snippet: "print(1)".to_string(),
    };
    let form = form_for(&lesson, lesson_spec());
    expect_that!(value_at(&form, "snippet"), some(eq("print(1)")));
}

#[component]
fn SnippetAsText() -> Element {
    use_form(|| {
        empty_form(form! {
            Lesson { snippet => { field: ProgramSnippetField { max_lines: 10 }, widget: text } }
        })
    })
    .render(using_fns! {})
}

#[gtest]
fn a_widget_on_the_path_overrides_the_named_fields_default() {
    let html = render_to_html(SnippetAsText);
    expect_that!(html, not(contains_substring("<textarea")));
    expect_that!(
        html,
        contains_regex(format!(
            r#"<input class="fx-control fx-input" type="text" id="{FORM_ID}-snippet""#
        ))
    );
}

#[gtest]
fn a_widget_on_the_path_keeps_the_named_fields_check() {
    let spec = form! {
        Lesson { snippet => { field: ProgramSnippetField { max_lines: 10 }, widget: text } }
    };
    let mut form = empty_form(spec);
    form.distribute_form_values(&typed(&[("title", "Loops"), ("snippet", "(")]));
    expect_that!(form.validate(), none());
    expect_that!(errors_at(&form, "snippet"), elements_are![eq(UNBALANCED)]);
}

// ── A field on a struct-shaped path (trap 4) ─────────────────────────────

/// A struct formoxus would otherwise render as a fieldset of two number
/// inputs. `SpanField` makes it ONE text input, `3-7`.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Span {
    start: u32,
    end: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct SpanField;

const NOT_A_SPAN: &str = "expected start-end, like 3-7";
const BACKWARDS: &str = "the span ends before it starts";

impl Field for SpanField {
    type Value = Span;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn parse(&self, raw: &str) -> std::result::Result<Span, ValidationMessage> {
        let (start, end) = raw
            .split_once('-')
            .ok_or_else(|| ValidationMessage::from(NOT_A_SPAN))?;
        let number = |n: &str| {
            n.trim()
                .parse()
                .map_err(|_| ValidationMessage::from(NOT_A_SPAN))
        };
        Ok(Span {
            start: number(start)?,
            end: number(end)?,
        })
    }

    fn format(&self, span: &Span) -> String {
        format!("{}-{}", span.start, span.end)
    }

    fn check(&self, span: &Span) -> Vec<ValidationMessage> {
        if span.end < span.start {
            vec![ValidationMessage::from(BACKWARDS)]
        } else {
            Vec::new()
        }
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Highlight {
    note: String,
    span: Span,
}

fn highlight() -> Highlight {
    Highlight {
        note: "the loop".to_string(),
        span: Span { start: 3, end: 7 },
    }
}

fn highlight_spec() -> FormSpec<Highlight> {
    form! { Highlight { span => { field: SpanField } } }
}

#[gtest]
fn a_named_field_on_a_struct_is_one_leaf_not_a_fieldset() {
    // The named field has to win at the top of `node_for_shape`; consulted
    // only for scalars, as today, it would never be reached here.
    let form = empty_form(highlight_spec());
    expect_that!(
        paths_of(&form),
        elements_are![eq("note"), eq("span")],
        "not `span.start` and `span.end`"
    );
}

#[gtest]
fn a_named_field_parses_a_struct_from_text() {
    let mut form = empty_form(highlight_spec());
    form.distribute_form_values(&typed(&[("note", "the loop"), ("span", "3-7")]));
    expect_that!(form.validate(), some(eq(&highlight())));
}

#[gtest]
fn a_named_fields_parse_error_and_check_both_reach_the_path() {
    let mut form = empty_form(highlight_spec());
    form.distribute_form_values(&typed(&[("note", "the loop"), ("span", "seven")]));
    expect_that!(form.validate(), none());
    expect_that!(errors_at(&form, "span"), elements_are![eq(NOT_A_SPAN)]);

    let mut form = empty_form(highlight_spec());
    form.distribute_form_values(&typed(&[("note", "the loop"), ("span", "7-3")]));
    expect_that!(form.validate(), none());
    expect_that!(errors_at(&form, "span"), elements_are![eq(BACKWARDS)]);
}

#[gtest]
fn a_named_field_refills_through_its_own_format() {
    let form = form_for(&highlight(), highlight_spec());
    expect_that!(value_at(&form, "span"), some(eq("3-7")));
}

#[gtest]
fn a_named_field_on_a_struct_round_trips_across_the_wire() {
    // `across_the_wire` uses the default spec, which has no `field:`, so the
    // round trip is written out with this one on both ends.
    let wire = WireForm::from_model(&highlight(), highlight_spec());
    let accepted = Submission::accept(highlight_spec(), wire.values()).map(Submission::into_model);
    expect_that!(accepted, ok(eq(&highlight())));
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Highlights {
    spans: Vec<Span>,
    maybe: Option<Span>,
}

fn highlights_spec() -> FormSpec<Highlights> {
    form! {
        Highlights {
            spans[] => { field: SpanField },
            maybe? => { field: SpanField },
        }
    }
}

#[gtest]
fn a_named_field_reaches_list_items() {
    let model = Highlights {
        spans: vec![Span { start: 1, end: 2 }, Span { start: 5, end: 9 }],
        maybe: None,
    };
    let form = form_for(&model, highlights_spec());
    expect_that!(value_at(&form, "spans.#0"), some(eq("1-2")));
    expect_that!(value_at(&form, "spans.#1"), some(eq("5-9")));
    let wire = WireForm::from_model(&model, highlights_spec());
    expect_that!(
        Submission::accept(highlights_spec(), wire.values()).map(Submission::into_model),
        ok(eq(&model))
    );
}

#[gtest]
fn a_named_field_on_an_option_is_none_when_empty_and_some_when_filled() {
    let mut form = empty_form(highlights_spec());
    form.distribute_form_values(&typed(&[("maybe", "")]));
    expect_that!(
        form.validate(),
        some(eq(&Highlights {
            spans: Vec::new(),
            maybe: None,
        }))
    );

    let mut form = empty_form(highlights_spec());
    form.distribute_form_values(&typed(&[("maybe", "2-4")]));
    expect_that!(
        form.validate(),
        some(eq(&Highlights {
            spans: Vec::new(),
            maybe: Some(Span { start: 2, end: 4 }),
        }))
    );
}

// ═════════════════════════════════════════════════════════════════════════
// 3. A type's widget, named once by the app: `TypeRegistry::widget_for`
// ═════════════════════════════════════════════════════════════════════════

/// apcsp's `Markdown`: a newtype over its text, listed as
/// `Note => newtype String`, and given a widget. Every form had to give it
/// `widget: MarkdownWidget` by hand.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Note(String);

/// Stands in for apcsp's `MarkdownWidget`, which lives in a crate (`ui`)
/// the model's crate cannot depend on.
#[derive(Clone, Debug, Default, PartialEq)]
struct NoteWidget;

impl Widget for NoteWidget {
    const NAME: &'static str = "note_widget";
    const EXTENDS: AttrSet = AttrSet::EMPTY;
    const PARTS: &'static [Tag] = &[];
}

impl LeafWidget for NoteWidget {
    fn render(&self, props: WidgetProps) -> Element {
        let path = props.path().clone();
        rsx! { div { class: "note-widget", "data-path": "{path}" } }
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Journal {
    title: String,
    entry: Note,
    notes: Vec<Note>,
    aside: Option<Note>,
}

fn journal() -> Journal {
    Journal {
        title: "Week 1".to_string(),
        entry: Note("began".to_string()),
        notes: vec![Note("one".to_string()), Note("two".to_string())],
        aside: None,
    }
}

#[component]
fn RegisteredJournal() -> Element {
    use_form(|| {
        form_for(
            &journal(),
            FormSpec::default().with_types(CustomTypes::registry()),
        )
    })
    .render(using_fns! {})
}

/// The control: a newtype registered with a field but NO widget. `Note` has
/// a widget for the whole process, so this needs a type of its own.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Remark(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Diary {
    entry: Remark,
}

#[component]
fn PlainDiary() -> Element {
    use_form(|| {
        form_for(
            &Diary {
                entry: Remark("began".to_string()),
            },
            FormSpec::default().with_types(CustomTypes::registry()),
        )
    })
    .render(using_fns! {})
}

#[gtest]
fn without_a_registered_widget_a_newtype_renders_its_inner_fields_widget() {
    let html = render_to_html(PlainDiary);
    expect_that!(html, not(contains_substring("note-widget")));
    expect_that!(
        html,
        contains_regex(format!(
            r#"<input class="fx-control fx-input" type="text" id="{FORM_ID}-entry" name="entry""#
        ))
    );
}

#[gtest]
#[ignore = "box 6: widget entries in types!"]
fn the_registered_widget_for_a_type_renders_every_field_of_that_type() {
    // No `widget:` anywhere: on a field, on each list item (met by `Shape`
    // alone), and through an `Option`.
    let html = render_to_html(RegisteredJournal);
    for path in ["entry", "notes.#0", "notes.#1", "aside"] {
        expect_that!(
            html,
            contains_substring(format!(r#"class="note-widget" data-path="{path}""#)),
            "{path}"
        );
    }
    expect_that!(html, not(contains_substring(r#"name="entry""#)));
}

#[gtest]
#[ignore = "box 6: widget entries in types!"]
fn the_registered_widget_for_a_type_leaves_other_types_alone() {
    let html = render_to_html(RegisteredJournal);
    expect_that!(
        html,
        contains_regex(format!(
            r#"<input class="fx-control fx-input" type="text" id="{FORM_ID}-title" name="title""#
        ))
    );
}

/// No component provides anything and the `VirtualDom` has no root context:
/// the registry is process-wide, not a Dioxus context (decision M).
#[gtest]
#[ignore = "box 6: widget entries in types!"]
fn the_registered_widget_needs_no_context() {
    let mut dom = VirtualDom::new(RegisteredJournal);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    expect_that!(html, contains_substring(r#"data-path="entry""#));
}

#[component]
fn JournalWithATextareaEntry() -> Element {
    use_form(|| {
        form_for(
            &journal(),
            form! { Journal { entry => { widget: textarea } } },
        )
    })
    .render(using_fns! {})
}

#[gtest]
#[ignore = "box 6: widget entries in types!"]
fn a_widget_on_the_path_overrides_the_registered_widget_for_its_type() {
    let html = render_to_html(JournalWithATextareaEntry);
    expect_that!(html, contains_regex(r#"<textarea [^>]*name="entry""#));
    expect_that!(html, not(contains_substring(r#"data-path="entry""#)));
    expect_that!(
        html,
        contains_substring(r#"data-path="notes.#0""#),
        "the other `Note`s keep the registered widget"
    );
}

// ── Chosen at build, so the server agrees (decision P) ───────────────────

/// Registered as a newtype over `String` and with `PasswordInput`. A password
/// is not trimmed, since a space is part of the secret, and that is read off
/// the widget.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Secret(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Login {
    name: String,
    secret: Secret,
}

#[component]
fn EmptyLogin() -> Element {
    use_form(|| empty_form::<Login>(FormSpec::default().with_types(CustomTypes::registry())))
        .render(using_fns! {})
}

#[gtest]
#[ignore = "box 6: widget entries in types!"]
fn a_registered_password_widget_renders_as_a_password_input() {
    let html = render_to_html(EmptyLogin);
    expect_that!(
        html,
        contains_regex(r#"<input [^>]*type="password"[^>]*name="secret""#)
    );
}

/// What decision L could not promise and P can: the server builds the form
/// with the same widget, so it does not trim the password either, while the
/// plain text beside it is trimmed as usual.
#[gtest]
#[ignore = "box 6: widget entries in types!"]
fn a_registered_password_widget_is_not_trimmed_on_the_server() {
    let accepted = Submission::<Login>::accept(
        FormSpec::default().with_types(CustomTypes::registry()),
        &wire(&[("name", "  ada  "), ("secret", "  hunter 2  ")]),
    )
    .map(Submission::into_model);
    expect_that!(
        accepted,
        ok(eq(&Login {
            name: "ada".to_string(),
            secret: Secret("  hunter 2  ".to_string()),
        }))
    );
}

// ── The order: `widget:`, then the registered widget, then the field's ───

/// A field of the dev's own for `Note`, whose default is a textarea.
#[derive(Clone, Debug, PartialEq)]
struct NoteField;

impl Field for NoteField {
    type Value = Note;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn parse(&self, raw: &str) -> std::result::Result<Note, ValidationMessage> {
        Ok(Note(raw.to_string()))
    }

    fn format(&self, note: &Note) -> String {
        note.0.clone()
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        Textarea.into()
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Memo {
    body: Note,
}

fn memo_spec() -> FormSpec<Memo> {
    form! { Memo { body => { field: NoteField } } }
}

#[component]
fn RegisteredMemo() -> Element {
    use_form(|| empty_form(memo_spec())).render(using_fns! {})
}

/// Decision H's order, pinned: the registered widget for `Note` beats the
/// named field's `default_widget`. It has to beat a BUILT-IN field's default,
/// or it would never show at all, and the order is kept the same for a dev's
/// field. If that ever reads wrong, this is the test to flip. (A named field's
/// default with nothing registered is section 2's
/// `a_named_field_renders_its_own_default_widget`.)
#[gtest]
#[ignore = "box 6: widget entries in types!"]
fn the_registered_widget_for_a_type_beats_a_named_fields_default_widget() {
    let html = render_to_html(RegisteredMemo);
    expect_that!(html, contains_substring(r#"data-path="body""#));
    expect_that!(html, not(contains_substring("<textarea")));
}

// ═════════════════════════════════════════════════════════════════════════
// 4. A typed reference: `Ref<Question>` with a registered field
// ═════════════════════════════════════════════════════════════════════════

/// The storage-agnostic `Ref<T>` decision I assumes, standing in until the
/// storage repo's core crate exists: a typed key that names no database and
/// carries nothing for formoxus. It has no `DefaultField`, which is what
/// today's `choice_field_for` needs, so `choices:` on it waits for Q.
#[derive(Facet, Debug)]
struct Ref<T: 'static> {
    key: String,
    #[facet(skip)]
    _of: PhantomData<T>,
}

impl<T> Ref<T> {
    fn new(key: &str) -> Self {
        Ref {
            key: key.to_string(),
            _of: PhantomData,
        }
    }
}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Ref::new(&self.key)
    }
}

impl<T> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

/// Reads and writes a `Ref<T>` as its key. Registered once per concrete `T`
/// (M): `Ref<Question>` and `Ref<Assignment>` are different shapes.
#[derive(Debug)]
struct RefField<T>(PhantomData<T>);

impl<T> Default for RefField<T> {
    fn default() -> Self {
        RefField(PhantomData)
    }
}

impl<T> Clone for RefField<T> {
    fn clone(&self) -> Self {
        RefField::default()
    }
}

impl<T> PartialEq for RefField<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Field for RefField<Question> {
    type Value = Ref<Question>;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn parse(&self, raw: &str) -> std::result::Result<Ref<Question>, ValidationMessage> {
        Ok(Ref::new(raw))
    }

    fn format(&self, r: &Ref<Question>) -> String {
        r.key.clone()
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Question {
    text: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    name: String,
    questions: Vec<Ref<Question>>,
}

fn assignment(keys: &[&str]) -> Assignment {
    Assignment {
        name: "Unit 1 review".to_string(),
        questions: keys.iter().map(|k| Ref::new(k)).collect(),
    }
}

#[gtest]
fn a_registered_reference_is_one_leaf_per_item_and_round_trips_as_its_key() {
    let model = assignment(&["question:q1", "question:q2"]);
    let form = form_for(
        &model,
        FormSpec::default().with_types(CustomTypes::registry()),
    );
    expect_that!(value_at(&form, "questions.#0"), some(eq("question:q1")));
    expect_that!(value_at(&form, "questions.#1"), some(eq("question:q2")));
    expect_that!(
        paths_of(&form),
        not(contains(eq("questions.#0.key"))),
        "one leaf, not a fieldset around `key`"
    );
    expect_that!(
        across_the_wire_with(CustomTypes::registry(), &model),
        ok(eq(&model))
    );
}

/// A registered type is ONE input, so a path into its fields names nothing
/// that is built (5.5.6). Refused when the spec is built, as a path into a
/// scalar is, rather than left to match nothing.
#[gtest]
fn a_path_into_a_registered_types_fields_is_refused() {
    let message = panic_message(|| {
        let _ = empty_form(form! { Assignment { questions[].key => { label: "Key" } } });
    });
    expect_that!(
        message,
        all![contains_substring("key"), contains_substring("one input")]
    );
}

/// `choices:` on a registered type, DEFERRED with decision Q (2026-11-25).
/// Kept compiled out rather than deleted: these are the tests Q's box starts
/// from. `form!` cannot build `choices:` over a type with no `DefaultField`
/// until Q's fallback exists.
#[cfg(any())]
mod deferred_q {
    use super::*;

    /// The course's questions as apcsp would pass them: the key's text, then
    /// the question's name.
    const COURSE_QUESTIONS: &[(&str, &str)] = &[
        ("question:q1", "One"),
        ("question:q2", "Two"),
        ("question:q3", "Three"),
    ];

    fn assignment_spec() -> FormSpec<Assignment> {
        form! { Assignment { questions[] => { choices: COURSE_QUESTIONS } } }
    }

    #[gtest]
    fn registered_items_among_the_choices_pass() {
        let model = assignment(&["question:q1", "question:q3"]);
        let mut form = form_for(&model, assignment_spec());
        expect_that!(form.validate(), some(eq(&model)));
    }

    #[gtest]
    fn a_registered_item_outside_the_choices_is_refused_on_its_item() {
        let mut form = form_for(
            &assignment(&["question:q1", "question:q99"]),
            assignment_spec(),
        );
        expect_that!(form.validate(), none());
        expect_that!(errors_at(&form, "questions.#0"), is_empty());
        expect_that!(
            errors_at(&form, "questions.#1"),
            elements_are![eq("not one of the available choices")]
        );
    }

    #[gtest]
    fn a_forged_registered_item_is_refused_by_submission_accept() {
        let wire = WireForm::from_model(
            &assignment(&["question:q1", "question:q99"]),
            FormSpec::default(),
        );
        let refused = Submission::accept(assignment_spec(), wire.values())
            .expect_err("q99 is not one of the course's questions");
        expect_that!(
            refused_at(&refused, "questions.#1"),
            elements_are![eq("not one of the available choices")]
        );
        expect_that!(refused.fields.len(), eq(1));
    }
}
