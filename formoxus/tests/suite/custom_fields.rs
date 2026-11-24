//! Custom fields (`CUSTOM_FIELDS_PLAN.md`, issue #28): a type that brings its
//! own text form, a path that names its own field, and an app that names a
//! type's widget once.
//!
//! Written as box 2 of the plan, BEFORE any of it exists, so this module does
//! not compile until boxes 3 to 6 do. Each section pins one of the plan's
//! three declarations, kept where the thing it needs is visible (decision C):
//!
//! 1. **The type declares its text form**, `#[facet(formoxus::proxy =
//!    String)]` (decision A). Seen from the `Shape` alone, so it works in a
//!    list item, in a variant, and on the server with no runtime.
//! 2. **A path declares its field**, `field: X` in `form!`. The spec is built
//!    on both sides, so `Submission::accept` runs the field's own checks.
//! 3. **The app declares a type's default widget**,
//!    `Formoxus::with_widget_for::<T>(W)`. Presentation only, so a Dioxus
//!    context the server never sees is enough for it.
//!
//! The API these tests choose, for boxes 3 to 6 to build or to change:
//! `Field::KIND: FieldType` (what `form!` checks a `widget:` against when a
//! path names a field) and `Field::format` (the reverse of `parse`, which
//! refills an input from an existing value).

use super::{FORM_ID, across_the_wire, render_to_html};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::{AttrSet, FieldType, Tag};
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::path::NodePath;
use formoxus::prelude::*;
use formoxus::stores::ValuesByPath;
use formoxus::widgets::{AnyWidget, LeafWidget, TextInput, Textarea, Widget, WidgetProps};
use googletest::prelude::*;
use std::marker::PhantomData;

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
// 1. A type with its own text form: `formoxus::proxy`
// ═════════════════════════════════════════════════════════════════════════

/// A time limit held in seconds and typed as `1h15m`: the case from #28 and
/// apcsp's assignment settings. A one-field tuple struct over a `u32`, so
/// without the proxy it would be a newtype whose wire value is `"4500"`.
#[derive(Facet, Clone, Debug, PartialEq)]
#[facet(formoxus::proxy = String)]
struct TimeLimit(u32);

/// The dev's own message, which the user must see as written (decision D),
/// not wrapped in facet's "Custom deserialization of shape …".
fn not_a_time(text: &str) -> String {
    format!("expected a time like 1h15m, not {text:?}")
}

impl TryFrom<String> for TimeLimit {
    type Error = String;

    /// `1h15m`, `1h` or `15m`. A bare number is refused: `90` could be
    /// minutes or seconds, which is the ambiguity the format exists to avoid.
    fn try_from(text: String) -> std::result::Result<Self, String> {
        let (hours, rest) = match text.split_once('h') {
            Some((h, rest)) => (Some(h), rest),
            None => (None, text.as_str()),
        };
        let minutes = match rest {
            "" => None,
            m => Some(m.strip_suffix('m').ok_or_else(|| not_a_time(&text))?),
        };
        if hours.is_none() && minutes.is_none() {
            return Err(not_a_time(&text));
        }
        let number = |part: Option<&str>| -> std::result::Result<u32, String> {
            part.map_or(Ok(0), |p| p.parse().map_err(|_| not_a_time(&text)))
        };
        Ok(TimeLimit(number(hours)? * 3600 + number(minutes)? * 60))
    }
}

impl TryFrom<&TimeLimit> for String {
    type Error = std::convert::Infallible;

    fn try_from(limit: &TimeLimit) -> std::result::Result<String, Self::Error> {
        let (h, m) = (limit.0 / 3600, limit.0 % 3600 / 60);
        Ok(match (h, m) {
            (0, m) => format!("{m}m"),
            (h, 0) => format!("{h}h"),
            (h, m) => format!("{h}h{m}m"),
        })
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
fn a_proxied_type_is_one_leaf_at_its_own_path() {
    let form = empty_form::<Quiz>(FormSpec::default());
    expect_that!(
        paths_of(&form),
        elements_are![eq("title"), eq("limit")],
        "a proxied tuple struct must not become `limit.0`"
    );
}

#[component]
fn EmptyQuiz() -> Element {
    use_form(|| empty_form::<Quiz>(FormSpec::default())).render(using_fns! {})
}

#[gtest]
fn a_proxied_type_renders_as_a_text_input() {
    // Not a number input: the inner `u32` is what the proxy hides.
    let html = render_to_html(EmptyQuiz);
    expect_that!(
        html,
        contains_regex(format!(
            r#"<input class="fx-control fx-input" type="text" id="{FORM_ID}-limit" name="limit""#
        ))
    );
    expect_that!(html, not(contains_substring(r#"type="number""#)));
}

// ── Parsing through the proxy ────────────────────────────────────────────

#[gtest]
fn a_proxied_type_parses_its_own_text_form() {
    let mut form = empty_form::<Quiz>(FormSpec::default());
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "1h15m")]));
    expect_that!(form.validate(), some(eq(&quiz())));
}

#[gtest]
fn a_proxied_type_refuses_bad_text_with_the_devs_own_message() {
    let mut form = empty_form::<Quiz>(FormSpec::default());
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "90")]));
    expect_that!(form.validate(), none());
    expect_that!(
        errors_at(&form, "limit"),
        elements_are![eq(&not_a_time("90"))],
        "the dev's message exactly, with nothing of facet's wrapped around it"
    );
}

#[gtest]
fn the_inner_values_text_form_is_refused() {
    // `"4500"` is what the field would take if it were still a newtype over
    // `u32`. The proxy replaces the inner type's text form; it does not add to it.
    let mut form = empty_form::<Quiz>(FormSpec::default());
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "4500")]));
    expect_that!(
        errors_at(&form, "limit"),
        elements_are![eq(&not_a_time("4500"))]
    );
}

#[gtest]
fn an_empty_proxied_field_is_required_like_any_other() {
    // `""` is absence (`facet_form_design_decisions.md`), so it never reaches
    // `TryFrom<String>`: the dev's message would be the wrong one here.
    let mut form = empty_form::<Quiz>(FormSpec::default());
    form.distribute_form_values(&typed(&[("title", "Unit 1"), ("limit", "")]));
    expect_that!(form.validate(), none());
    let errors = errors_at(&form, "limit");
    expect_that!(errors, len(eq(1)));
    expect_that!(errors, not(contains(eq(&not_a_time("")))));
}

// ── Refilling from an existing value ─────────────────────────────────────

#[gtest]
fn a_proxied_type_refills_its_input_through_the_proxy() {
    let form = form_for(&quiz(), FormSpec::default());
    expect_that!(
        value_at(&form, "limit"),
        some(eq("1h15m")),
        "the proxy's text, not the inner `4500`"
    );
}

#[component]
fn FilledQuiz() -> Element {
    use_form(|| form_for(&quiz(), FormSpec::default())).render(using_fns! {})
}

#[gtest]
fn a_refilled_proxied_input_shows_the_proxys_text() {
    let html = render_to_html(FilledQuiz);
    expect_that!(html, contains_substring(r#"name="limit" value="1h15m""#));
}

#[gtest]
fn a_proxied_type_round_trips_in_memory_and_across_the_wire() {
    let mut form = form_for(&quiz(), FormSpec::default());
    expect_that!(form.validate(), some(eq(&quiz())));
    expect_that!(across_the_wire(&quiz()), ok(eq(&quiz())));
}

// ── On the server ────────────────────────────────────────────────────────

#[gtest]
fn submission_accept_parses_a_proxied_type_from_raw_wire_text() {
    let accepted = Submission::<Quiz>::accept(
        FormSpec::default(),
        &wire(&[("title", "Unit 1"), ("limit", "1h15m")]),
    )
    .map(Submission::into_model);
    expect_that!(accepted, ok(eq(&quiz())));
}

#[gtest]
fn submission_accept_refuses_a_proxied_type_with_the_devs_own_message() {
    let refused = Submission::<Quiz>::accept(
        FormSpec::default(),
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
fn an_empty_optional_proxied_field_is_none() {
    // Trap 6: `Option<TimeLimit>` peels to the proxied leaf, as
    // `Option<Markdown>` peels to a newtype.
    let mut form = empty_form::<MaybeTimed>(FormSpec::default());
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
fn a_filled_optional_proxied_field_is_some() {
    let mut form = empty_form::<MaybeTimed>(FormSpec::default());
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
fn an_optional_proxied_field_round_trips_both_ways() {
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
        expect_that!(across_the_wire(&model), ok(eq(&model)));
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
fn a_proxied_list_item_is_one_leaf_per_item() {
    let form = form_for(&stages(), FormSpec::default());
    expect_that!(value_at(&form, "limits.#0"), some(eq("1h15m")));
    expect_that!(value_at(&form, "limits.#1"), some(eq("15m")));
    expect_that!(across_the_wire(&stages()), ok(eq(&stages())));
}

#[gtest]
fn a_proxied_list_item_refuses_bad_text_on_its_own_item() {
    let mut form = form_for(&stages(), FormSpec::default());
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
fn a_proxied_field_in_a_variant_parses_and_refills() {
    let mut form = empty_form::<Paced>(FormSpec::default());
    form.distribute_form_values(&typed(&[("pace", "Timed"), ("pace.$Timed.limit", "1h")]));
    expect_that!(form.validate(), some(eq(&timed_for_an_hour())));
    expect_that!(
        value_at(
            &form_for(&timed_for_an_hour(), FormSpec::default()),
            "pace.$Timed.limit"
        ),
        some(eq("1h"))
    );
    expect_that!(
        across_the_wire(&timed_for_an_hour()),
        ok(eq(&timed_for_an_hour()))
    );
}

#[gtest]
fn a_proxied_field_in_a_variant_refuses_bad_text() {
    let mut form = empty_form::<Paced>(FormSpec::default());
    form.distribute_form_values(&typed(&[("pace", "Timed"), ("pace.$Timed.limit", "90")]));
    expect_that!(
        errors_at(&form, "pace.$Timed.limit"),
        elements_are![eq(&not_a_time("90"))]
    );
}

// ── `form!` on a proxied type ────────────────────────────────────────────

#[component]
fn QuizWithATextarea() -> Element {
    use_form(|| {
        empty_form(form! {
            Quiz { limit => { widget: textarea, label: "Time allowed" } }
        })
    })
    .render(using_fns! {})
}

/// Trap 2: `form!`'s `const _` checks classify a proxied type as text, so a
/// text widget is accepted on it. Without that, this would not compile.
#[gtest]
fn form_accepts_a_text_widget_on_a_proxied_type() {
    let html = render_to_html(QuizWithATextarea);
    expect_that!(
        html,
        contains_substring(r#"<textarea class="fx-control fx-textarea""#)
    );
    expect_that!(html, contains_substring("Time allowed"));
}

// ── Only `String` proxies, for now ───────────────────────────────────────

/// Proxied to a `u32`, which decision E leaves for later.
#[derive(Facet, Clone, Debug, PartialEq)]
#[facet(formoxus::proxy = u32)]
struct Minutes(u32);

impl TryFrom<u32> for Minutes {
    type Error = String;
    fn try_from(n: u32) -> std::result::Result<Self, String> {
        Ok(Minutes(n))
    }
}

impl TryFrom<&Minutes> for u32 {
    type Error = std::convert::Infallible;
    fn try_from(m: &Minutes) -> std::result::Result<u32, Self::Error> {
        Ok(m.0)
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Timer {
    minutes: Minutes,
}

/// Decision E: a proxy to anything but `String` is refused by a panic that
/// names the type, rather than being quietly read as a newtype over `u32`.
/// The exact wording is box 5's; this pins only that the type is named.
#[gtest]
#[should_panic(expected = "Minutes")]
fn a_proxy_to_anything_but_string_panics_naming_the_type() {
    let _ = empty_form::<Timer>(FormSpec::default());
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
    // The spec is built on both sides, so `field:` reaches `Submission::accept`
    // where an app-wide Dioxus context would not (the plan's point 4).
    let refused = Submission::accept(
        lesson_spec(),
        &wire(&[("title", "Loops"), ("snippet", "print((1)")]),
    )
    .expect_err("the parentheses do not balance");
    expect_that!(refused_at(&refused, "snippet"), elements_are![eq(UNBALANCED)]);

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
    expect_that!(errors_at(&form, "span"), elements_are![eq(NOT_A_SPAN)]);

    let mut form = empty_form(highlight_spec());
    form.distribute_form_values(&typed(&[("note", "the loop"), ("span", "7-3")]));
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
// 3. A type's default widget, named once by the app
// ═════════════════════════════════════════════════════════════════════════

/// apcsp's `Markdown`: a newtype formoxus already reads, which every form
/// had to give `widget: MarkdownWidget` by hand.
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

fn with_note_widget() -> Formoxus {
    Formoxus::new().with_widget_for::<Note>(NoteWidget)
}

#[component]
fn PlainJournal() -> Element {
    use_form(|| form_for(&journal(), FormSpec::default())).render(using_fns! {})
}

#[component]
fn AppWideJournal() -> Element {
    provide_defaults(with_note_widget());
    use_form(|| form_for(&journal(), FormSpec::default())).render(using_fns! {})
}

#[gtest]
fn without_a_registration_a_newtype_renders_as_its_inner_type() {
    // The control: nothing names a widget for `Note`, so it is a text input.
    let html = render_to_html(PlainJournal);
    expect_that!(html, not(contains_substring("note-widget")));
    expect_that!(html, contains_substring(r#"name="entry""#));
}

#[gtest]
fn the_apps_widget_for_a_type_renders_every_field_of_that_type() {
    // No `widget:` anywhere: on a field, on each list item (met by `Shape`
    // alone), and through an `Option`.
    let html = render_to_html(AppWideJournal);
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
fn the_apps_widget_for_a_type_leaves_other_types_alone() {
    let html = render_to_html(AppWideJournal);
    expect_that!(
        html,
        contains_regex(format!(
            r#"<input class="fx-control fx-input" type="text" id="{FORM_ID}-title" name="title""#
        ))
    );
}

#[component]
fn JournalWithATextareaEntry() -> Element {
    provide_defaults(with_note_widget());
    use_form(|| form_for(&journal(), form! { Journal { entry => { widget: textarea } } }))
        .render(using_fns! {})
}

#[gtest]
fn a_widget_on_the_path_overrides_the_apps_widget_for_its_type() {
    let html = render_to_html(JournalWithATextareaEntry);
    expect_that!(html, contains_regex(r#"<textarea [^>]*name="entry""#));
    expect_that!(html, not(contains_substring(r#"data-path="entry""#)));
    expect_that!(
        html,
        contains_substring(r#"data-path="notes.#0""#),
        "the other `Note`s keep the app's widget"
    );
}

/// What `LaunchBuilder::with_context(config)` does for the client root and
/// the server-side render: the config arrives as a ROOT context, with no
/// component calling `provide_defaults`. As far as a suite test can reach a
/// launch.
#[gtest]
fn the_apps_widget_for_a_type_arrives_as_a_launch_context() {
    let mut dom = VirtualDom::new(PlainJournal).with_root_context(with_note_widget());
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    expect_that!(html, contains_substring(r#"data-path="entry""#));
}

/// `LaunchBuilder::with_context` asks for `Any + Clone + Send + Sync`.
#[gtest]
fn formoxus_with_a_widget_for_a_type_can_be_a_launch_context() {
    fn launchable<T: std::any::Any + Clone + Send + Sync + 'static>(_: &T) {}
    launchable(&with_note_widget());
}

#[component]
fn AppWideQuiz() -> Element {
    provide_defaults(Formoxus::new().with_widget_for::<TimeLimit>(NoteWidget));
    use_form(|| form_for(&quiz(), FormSpec::default())).render(using_fns! {})
}

#[gtest]
fn the_apps_widget_for_a_type_reaches_a_proxied_type() {
    let html = render_to_html(AppWideQuiz);
    expect_that!(html, contains_substring(r#"data-path="limit""#));
}

// ── The order: `widget:`, then the app's per-type widget, then the field's ─

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
fn PlainMemo() -> Element {
    use_form(|| empty_form(memo_spec())).render(using_fns! {})
}

#[component]
fn AppWideMemo() -> Element {
    provide_defaults(with_note_widget());
    use_form(|| empty_form(memo_spec())).render(using_fns! {})
}

#[gtest]
fn a_named_fields_default_widget_renders_when_the_app_names_none() {
    let html = render_to_html(PlainMemo);
    expect_that!(html, contains_regex(r#"<textarea [^>]*name="body""#));
}

/// Decision H's order, pinned: the app's widget for `Note` beats the named
/// field's `default_widget`. It has to beat a BUILT-IN field's default, or it
/// would never show at all, and the order is kept the same for a dev's field.
/// If that ever reads wrong, this is the test to flip.
#[gtest]
fn the_apps_widget_for_a_type_beats_a_named_fields_default_widget() {
    let html = render_to_html(AppWideMemo);
    expect_that!(html, contains_substring(r#"data-path="body""#));
    expect_that!(html, not(contains_substring("<textarea")));
}

// ═════════════════════════════════════════════════════════════════════════
// 4. `choices:` on a proxied list item (decision F)
// ═════════════════════════════════════════════════════════════════════════

/// The storage-agnostic `Ref<T>` decision I assumes, standing in until the
/// storage repo's core crate exists: a typed key, proxied to `String`, that
/// names no database. Its crate cannot depend on formoxus, so it has no
/// `DefaultField`, which is what today's `choice_field_for` needs.
#[derive(Facet, Debug)]
#[facet(formoxus::proxy = String)]
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

impl<T> TryFrom<String> for Ref<T> {
    type Error = String;
    fn try_from(key: String) -> std::result::Result<Self, String> {
        Ok(Ref {
            key,
            _of: PhantomData,
        })
    }
}

impl<T> TryFrom<&Ref<T>> for String {
    type Error = std::convert::Infallible;
    fn try_from(r: &Ref<T>) -> std::result::Result<String, Self::Error> {
        Ok(r.key.clone())
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

/// The course's questions as apcsp would pass them: the key's text, then
/// the question's name.
const COURSE_QUESTIONS: &[(&str, &str)] = &[
    ("question:q1", "One"),
    ("question:q2", "Two"),
    ("question:q3", "Three"),
];

fn assignment(keys: &[&str]) -> Assignment {
    Assignment {
        name: "Unit 1 review".to_string(),
        questions: keys.iter().map(|k| Ref::new(k)).collect(),
    }
}

fn assignment_spec() -> FormSpec<Assignment> {
    form! { Assignment { questions[] => { choices: COURSE_QUESTIONS } } }
}

#[gtest]
fn a_proxied_item_without_choices_round_trips_as_its_key() {
    let model = assignment(&["question:q1", "question:q2"]);
    let form = form_for(&model, FormSpec::default());
    expect_that!(value_at(&form, "questions.#0"), some(eq("question:q1")));
    expect_that!(value_at(&form, "questions.#1"), some(eq("question:q2")));
    expect_that!(across_the_wire(&model), ok(eq(&model)));
}

#[gtest]
fn proxied_items_among_the_choices_pass() {
    let model = assignment(&["question:q1", "question:q3"]);
    let mut form = form_for(&model, assignment_spec());
    expect_that!(form.validate(), some(eq(&model)));
}

#[gtest]
fn a_proxied_item_outside_the_choices_is_refused_on_its_item() {
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
fn a_forged_proxied_item_is_refused_by_submission_accept() {
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
