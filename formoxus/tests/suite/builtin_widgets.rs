//! The built-in widget types (`FIELDS_PLAN.md` Step 5, W-C), driven through
//! `Widget::render` on an `AnyWidget` directly, the way `FieldNode` does, so
//! each is tested apart from the node that hands it its props.

// `form!` and `path!` read the `FormoxusTypes` in scope (decision AC).
use dioxus::core::Attribute;
use dioxus::prelude::*;
use formoxus::FormoxusTypes;
use formoxus::ValidationMessage;
use formoxus::attrs::AttrSet;
use formoxus::attrs::{Attr, AttrKey, AttrValue, InputType as T};
use formoxus::classes::Classes;
use formoxus::field_attrs::FieldAttrs;
use formoxus::path::NodePath;
use formoxus::stores::{ErrorsByPath, ValuesByPath};
use formoxus::widgets::Choice;
use formoxus::widgets::WidgetProps;
use formoxus::widgets::{
    AnyWidget, CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput,
    MonthInput, NumberInput, PasswordInput, RadioGroup, SearchInput, Select, TelInput, TextInput,
    Textarea, TimeInput, UrlInput, WeekInput, Widget,
};
use googletest::prelude::*;
use std::rc::Rc;

const ID: &str = "rand0123-title";
const PATH: &str = "title";
/// The hint every widget is handed, so "every prop" includes it.
const HELP: &str = "As it appears on the cover.";

/// Every text-like widget type, beside the `type=` it renders. Nothing here
/// notices one added to the macro and left out of this list; that every
/// built-in has a `WidgetName` row is `formoxus`'s own
/// `each_builtins_row_names_the_builtin_itself`.
fn text_likes() -> Vec<(AnyWidget, T)> {
    vec![
        (TextInput.into(), T::Text),
        (PasswordInput.into(), T::Password),
        (NumberInput.into(), T::Number),
        (EmailInput.into(), T::Email),
        (TelInput.into(), T::Telephone),
        (UrlInput.into(), T::Url),
        (SearchInput.into(), T::Search),
        (ColorInput.into(), T::Color),
        (DateInput.into(), T::Date),
        (TimeInput.into(), T::Time),
        (DatetimeLocalInput.into(), T::DatetimeLocal),
        (MonthInput.into(), T::Month),
        (WeekInput.into(), T::Week),
    ]
}

/// SSR of `root` given `props`: `render_to_html`, for a root that takes props,
/// so one root serves every widget in the table.
fn html_of<P: Clone + 'static>(root: fn(P) -> Element, props: P) -> String {
    let mut dom = VirtualDom::new_with_props(root, props);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

// ── One set of props, handed to every widget ────────────────────────────
//
// Everything a widget can be given, so a part a widget drops shows up as a
// missing substring: a label, an error (so the invalid classes and the owned
// `aria-` attributes appear), a field attribute and an author one.

fn field_attrs() -> FieldAttrs {
    FieldAttrs::from([
        (AttrKey::Std(Attr::MinLength), AttrValue::Int(2)),
        (AttrKey::Std(Attr::Required), AttrValue::Flag),
    ])
}

fn extra_attrs() -> Vec<Attribute> {
    vec![Attribute::new("autocomplete", "off", None, false)]
}

fn errors() -> Vec<ValidationMessage> {
    vec![ValidationMessage::from("too short")]
}

fn values() -> ValuesByPath {
    ValuesByPath::from([(NodePath::new(PATH), "A".to_string())])
}

fn through_widget((widget, choices): (AnyWidget, Option<Vec<Choice>>)) -> Element {
    let mut props = WidgetProps::detached(
        ID,
        NodePath::new(PATH),
        field_attrs(),
        values(),
        ErrorsByPath::from([(NodePath::new(PATH), errors())]),
        Rc::new(Classes::fx()),
    )
    .with_label("Title")
    .with_help_text(HELP);
    props.extra_attrs = extra_attrs();
    props.choices = choices;
    widget.render(props)
}

// ── The text-like inputs ─────────────────────────────────────────────────

/// Each piece of the props reaches the markup.
#[gtest]
fn each_text_like_renders_every_prop() {
    for (widget, input_type) in text_likes() {
        let html = html_of(through_widget, (widget.clone(), None));
        expect_that!(
            html,
            all![
                contains_substring(format!(r#"type="{}""#, input_type.html_type())),
                contains_substring(r#"value="A""#),
                contains_substring("Title"),
                contains_substring("too short"),
                contains_substring("minlength=2"),
                contains_substring(r#"autocomplete="off""#),
                contains_substring("fx-invalid"),
                contains_substring(format!(
                    r#"<div class="fx-help-text" id="{ID}-help">{HELP}</div>"#
                )),
            ],
            "{}",
            widget.name()
        );
    }
}

/// Bare: nothing a user sees, so nothing to label or mark invalid.
#[gtest]
fn hidden_input_renders_bare() {
    let html = html_of(through_widget, (HiddenInput.into(), None));
    expect_that!(
        html,
        all![
            contains_substring(r#"type="hidden""#),
            contains_substring(r#"value="A""#),
            not(contains_substring("Title")),
            not(contains_substring("too short")),
            not(contains_substring("<label")),
            not(contains_substring(HELP)),
        ]
    );
}

#[gtest]
fn checkbox_and_textarea_render_every_prop() {
    for (widget, element) in [
        (AnyWidget::from(CheckboxInput), r#"type="checkbox""#),
        (Textarea.into(), "<textarea"),
    ] {
        expect_that!(
            html_of(through_widget, (widget.clone(), None)),
            all![
                contains_substring(element),
                contains_substring("Title"),
                contains_substring("too short"),
                contains_substring(r#"autocomplete="off""#),
                contains_substring("fx-invalid"),
                contains_substring(HELP),
            ],
            "{}",
            widget.name()
        );
    }
}

/// `NAME` is `form!`'s spelling, so a `classes_plus:` selector keeps working
/// once `Classes` is keyed on it.
#[gtest]
fn each_builtin_is_named_as_form_spells_it() {
    expect_that!(Textarea::NAME, eq("textarea"));
    expect_that!(TextInput::NAME, eq("text"));
    expect_that!(TelInput::NAME, eq("tel"));
    expect_that!(DatetimeLocalInput::NAME, eq("datetime_local"));
}

/// What a text `<input>` takes, less what formoxus owns.
#[gtest]
fn text_input_extends_a_text_inputs_attributes() {
    let extends = TextInput::EXTENDS;
    expect_that!(extends.contains(Attr::Placeholder), eq(true));
    expect_that!(extends.contains(Attr::MinLength), eq(true));
    expect_that!(extends.contains(Attr::Rows), eq(false), "a textarea's");
    expect_that!(extends.contains(Attr::Step), eq(false), "a number input's");
    expect_that!(extends.contains(Attr::Required), eq(false), "owned");
}

/// The reason W-C gives each input type its own Rust type: what an `<input>`
/// takes depends on its `type`.
#[gtest]
fn each_input_type_extends_its_own_attributes() {
    expect_that!(NumberInput::EXTENDS.contains(Attr::Step), eq(true));
    expect_that!(NumberInput::EXTENDS.contains(Attr::MinLength), eq(false));
    expect_that!(ColorInput::EXTENDS.contains(Attr::Alpha), eq(true));
    expect_that!(TextInput::EXTENDS.contains(Attr::Alpha), eq(false));
    expect_that!(DateInput::EXTENDS.contains(Attr::Min), eq(true));
    expect_that!(EmailInput::EXTENDS.contains(Attr::Pattern), eq(true));
    expect_that!(NumberInput::EXTENDS.contains(Attr::Pattern), eq(false));
    // Valid on an email input, but owned (a leaf holds one string).
    expect_that!(EmailInput::EXTENDS.contains(Attr::Multiple), eq(false));
}

/// Empty, because `InputView` spreads nothing onto a hidden input: anything
/// written for one would be dropped, so `form!` should refuse it.
#[gtest]
fn hidden_input_extends_nothing() {
    expect_that!(HiddenInput::EXTENDS, eq(AttrSet::EMPTY));
}

#[gtest]
fn checkbox_and_textarea_extend_their_own_elements() {
    expect_that!(Textarea::EXTENDS.contains(Attr::Rows), eq(true));
    expect_that!(Textarea::EXTENDS.contains(Attr::MaxLength), eq(true));
    expect_that!(Textarea::EXTENDS.contains(Attr::Pattern), eq(false));
    expect_that!(
        CheckboxInput::EXTENDS.contains(Attr::Placeholder),
        eq(false)
    );
    expect_that!(CheckboxInput::EXTENDS.contains(Attr::Disabled), eq(true));
}

// ── The choosers ─────────────────────────────────────────────────────────

fn states() -> Vec<Choice> {
    vec![Choice::new("NY", "New York"), Choice::new("OH", "Ohio")]
}

/// The field's choices reach the widget through `WidgetProps`, and win over
/// a bool's true/false.
#[gtest]
fn each_chooser_renders_the_choices_it_is_given() {
    for widget in [AnyWidget::from(Select), RadioGroup.into()] {
        let html = html_of(through_widget, (widget.clone(), Some(states())));
        expect_that!(
            html,
            all![
                contains_substring(r#"value="NY""#),
                contains_substring("Ohio"),
                not(contains_substring(r#"value="true""#)),
            ],
            "{}",
            widget.name()
        );
    }
}

/// With no list, a chooser over a bool offers true and false.
#[gtest]
fn each_chooser_falls_back_to_a_bools_choices() {
    for widget in [AnyWidget::from(Select), RadioGroup.into()] {
        expect_that!(
            html_of(through_widget, (widget.clone(), None)),
            all![
                contains_substring(r#"value="true""#),
                contains_substring(r#"value="false""#),
            ],
            "{}",
            widget.name()
        );
    }
}

#[gtest]
fn the_choosers_extend_their_own_elements() {
    expect_that!(Select::EXTENDS.contains(Attr::Size), eq(true));
    expect_that!(Select::EXTENDS.contains(Attr::Placeholder), eq(false));
    // The spread lands on the fieldset, so a radio's attributes are not here.
    expect_that!(RadioGroup::EXTENDS.contains(Attr::Disabled), eq(true));
    expect_that!(RadioGroup::EXTENDS.contains(Attr::Checked), eq(false));
}

// ── a seeded textarea ────────────────────────────────────────────────────

#[derive(facet::Facet, Clone, Debug, PartialEq)]
struct Bio {
    about: String,
}

/// A form seeded with `about`, its textarea widget, rendered the way a
/// fullstack server does: `pre_render`, with the hydration markers that a
/// plain `render` leaves out.
fn pre_render_seeded(about: &'static str) -> String {
    #[component]
    fn App(about: &'static str) -> Element {
        let bio = Bio {
            about: about.into(),
        };
        let form = formoxus::use_form(move || {
            formoxus::form_for(
                &bio,
                formoxus::form! { Bio { about => { widget: textarea } } },
            )
        });
        form.render(formoxus::using_fns! {})
    }
    let mut dom = VirtualDom::new_with_props(App, AppProps { about });
    dom.rebuild_in_place();
    dioxus_ssr::pre_render(&dom)
}

/// **The value is the textarea's CONTENT.** A textarea ignores a `value`
/// attribute, and hydration never sets the property, so without content a
/// seeded textarea is blank in the browser. The content is exactly the text:
/// a dynamic text child would carry `<!--node-id…-->` markers here, which a
/// textarea shows literally.
#[gtest]
fn a_seeded_textarea_holds_its_value_as_its_content() {
    let html = pre_render_seeded("one\ntwo");
    expect_that!(html, contains_substring(">one\ntwo</textarea>"));
}

/// Content is parsed as HTML text, so it is escaped: unescaped, the `&`
/// would start a character reference and `</textarea>` would end the element.
#[gtest]
fn a_seeded_textareas_content_is_escaped() {
    let html = pre_render_seeded("a < b && </textarea> c");
    expect_that!(
        html,
        contains_substring(">a &lt; b &amp;&amp; &lt;/textarea&gt; c</textarea>")
    );
}

// ── Which (value type, widget) pairs build ──────────────────────────────
//
// What was `examples/src/bin/widget_matrix.rs`, a table printed by hand,
// until 2026-11-27, when a hand-built spec started getting the refusal
// `form!` gives (`CUSTOM_FIELDS_PLAN.md` decision V, reversed). The run time
// and `form!` now judge a pair by the same test, each widget's `ACCEPTS`, so
// what is left to check is that the two agree in practice: a pair a widget
// accepts builds AND renders, and every other pair is refused when the form
// is built. `range` is left out, since it needs `min:` and `max:` to build at
// all; its own tests cover it.

#[derive(facet::Facet, Clone, Debug, PartialEq)]
struct Sampler {
    text: String,
    int: i64,
    float: f64,
    flag: bool,
    maybe_text: Option<String>,
    maybe_flag: Option<bool>,
}

/// Each sampled path, its type, and whether it is an `Option`'s inside.
const SAMPLED: [(&str, formoxus::attrs::FieldType, bool); 6] = [
    ("text", formoxus::attrs::FieldType::Text, false),
    ("int", formoxus::attrs::FieldType::Int, false),
    ("float", formoxus::attrs::FieldType::Float, false),
    ("flag", formoxus::attrs::FieldType::Bool, false),
    ("maybe_text", formoxus::attrs::FieldType::Text, true),
    ("maybe_flag", formoxus::attrs::FieldType::Bool, true),
];

fn sampled_path(name: &str) -> formoxus::ModelPath<Sampler> {
    match name {
        "text" => formoxus::path!(Sampler.text),
        "int" => formoxus::path!(Sampler.int),
        "float" => formoxus::path!(Sampler.float),
        "flag" => formoxus::path!(Sampler.flag),
        "maybe_text" => formoxus::path!(Sampler.maybe_text?),
        _ => formoxus::path!(Sampler.maybe_flag?),
    }
}

/// A widget's `form!` name, the widget, and whether it renders a type, and
/// an `Option` of one: what `form!` asks with no choices.
type Sample = (
    &'static str,
    AnyWidget,
    fn(formoxus::attrs::FieldType, bool) -> bool,
);

fn sample<LW: formoxus::widgets::LeafWidget + Default>() -> Sample {
    (LW::NAME, LW::default().into(), |t, optional| {
        LW::ACCEPTS.contains(t) && (!optional || LW::ACCEPTS_OPTIONAL)
    })
}

fn every_single_value_widget() -> [Sample; 18] {
    [
        sample::<TextInput>(),
        sample::<PasswordInput>(),
        sample::<HiddenInput>(),
        sample::<NumberInput>(),
        sample::<EmailInput>(),
        sample::<TelInput>(),
        sample::<UrlInput>(),
        sample::<SearchInput>(),
        sample::<ColorInput>(),
        sample::<DateInput>(),
        sample::<TimeInput>(),
        sample::<DatetimeLocalInput>(),
        sample::<MonthInput>(),
        sample::<WeekInput>(),
        sample::<CheckboxInput>(),
        sample::<Textarea>(),
        sample::<Select>(),
        sample::<RadioGroup>(),
    ]
}

fn sampler_spec(name: &str, widget: AnyWidget) -> formoxus::FormSpec<Sampler> {
    formoxus::FormSpec::<Sampler>::new().with_widget(&sampled_path(name), widget)
}

#[derive(Props, Clone, PartialEq)]
struct OneSampleProps {
    name: &'static str,
    widget: AnyWidget,
}

#[allow(non_snake_case)]
fn OneSample(props: OneSampleProps) -> Element {
    let form = formoxus::use_form(move || {
        formoxus::empty_form(sampler_spec(props.name, props.widget.clone()))
    });
    form.render(formoxus::using_fns! {})
}

#[gtest]
fn a_pair_a_widget_accepts_renders_and_every_other_is_refused_at_build() {
    for (widget_name, widget, accepts) in every_single_value_widget() {
        for (name, field_type, optional) in SAMPLED {
            if accepts(field_type, optional) {
                let html = html_of(
                    OneSample,
                    OneSampleProps {
                        name,
                        widget: widget.clone(),
                    },
                );
                expect_that!(
                    html,
                    contains_substring(format!(r#"name="{name}""#)),
                    "`{widget_name}` on {field_type:?} (optional: {optional}) is accepted, so \
                     it must render"
                );
            } else {
                let message = super::panic_message(|| {
                    let _ = formoxus::empty_form(sampler_spec(name, widget.clone()));
                });
                expect_that!(
                    message,
                    contains_substring(format!("the widget `{widget_name}` cannot render")),
                    "`{widget_name}` on {field_type:?} (optional: {optional})"
                );
            }
        }
    }
}
