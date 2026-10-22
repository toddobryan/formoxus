//! The `<input>` family: every `type=` formoxus renders, routed by
//! [`RenderedInputType`] to one of three bodies. Text-like types (whose value
//! the DOM hands back as plain text) read caption-then-input; a checkbox
//! reads box-then-caption and binds `checked`; a hidden input renders bare.

use std::marker::PhantomData;

use dioxus::prelude::*;
use formoxus_spec::{
    Attr, AttrSet, BuiltinWidget, FieldControl, FieldType, FieldTypeSet, HtmlElement, InputType,
    RenderedInputType, Tag,
};

use super::field_row::{LabelSide, field_row};
use super::values::{get_current, write_value};
use crate::nodes::ValuesStore;
use crate::widgets::{Widget, widget_props::WidgetProps};

/// One [`Widget`] per text-like `<input type=…>`, each a struct with no props.
///
/// Everything a type needs follows from its `RenderedInputType` variant:
/// `NAME` is `form!`'s spelling of it (`BuiltinWidget::form_name`, so the two
/// cannot drift), `EXTENDS` is what HTML allows on that `<input>`, and
/// `PARTS`, `ACCEPTS` and `render` are shared. The variant names one in both
/// `RenderedInputType` and `InputType`, which spell them alike. Hidden and
/// checkbox render differently, so they are written by hand.
macro_rules! text_like_inputs {
    ($( $(#[$doc:meta])* $widget:ident => $variant:ident, )+) => {
        $(
            $(#[$doc])*
            #[derive(Clone, Debug, PartialEq, Default)]
            pub struct $widget;

            impl Widget for $widget {
                const NAME: &'static str =
                    BuiltinWidget::Input(RenderedInputType::$variant).form_name();

                const PARTS: &'static [Tag] = &[Tag::Div, Tag::Label, Tag::Input];

                const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Control(
                    FieldControl::Input(InputType::$variant),
                ));

                /// Never `Bool`: a text box cannot say true or false. A number
                /// is accepted, since the field parses the text back.
                const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Text).with(FieldType::Int).with(FieldType::Float);

                const ACCEPTS_OPTIONAL: bool = true;

                const TAKES_CHOICES: bool = false;

                fn render(&self, props: WidgetProps) -> Element {
                    render_input::<Self>(RenderedInputType::$variant, props)
                }
            }
        )+
    };
}

text_like_inputs! {
    /// `<input type="text">`, the default for a `String` or a number.
    TextInput => Text,
    /// `<input type="password">`. Echoes its value back, like the rest.
    PasswordInput => Password,
    /// `<input type="number">`. Never a default: a half-typed number reads as
    /// `""`, so a dev chooses it.
    NumberInput => Number,
    /// `<input type="email">`.
    EmailInput => Email,
    /// `<input type="tel">`.
    TelInput => Telephone,
    /// `<input type="url">`.
    UrlInput => Url,
    /// `<input type="search">`.
    SearchInput => Search,
    /// `<input type="color">`.
    ColorInput => Color,
    /// `<input type="date">`.
    DateInput => Date,
    /// `<input type="time">`.
    TimeInput => Time,
    /// `<input type="datetime-local">`.
    DatetimeLocalInput => DatetimeLocal,
    /// `<input type="month">`.
    MonthInput => Month,
    /// `<input type="week">`.
    WeekInput => Week,
}

/// `<input type="hidden">`: bare, with no label, caption or errors, since
/// nothing is shown to wrap.
///
/// `EXTENDS` is EMPTY, not what HTML allows on a hidden input: `InputView`
/// renders it without spreading any attribute, so one written for it would
/// be dropped. Empty makes `form!` refuse it instead (the props box).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HiddenInput;

impl Widget for HiddenInput {
    const NAME: &'static str = BuiltinWidget::Input(RenderedInputType::Hidden).form_name();

    const EXTENDS: AttrSet = AttrSet::EMPTY;

    const PARTS: &'static [Tag] = &[];

    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY
        .with(FieldType::Text)
        .with(FieldType::Int)
        .with(FieldType::Float);

    const ACCEPTS_OPTIONAL: bool = true;

    const TAKES_CHOICES: bool = false;

    fn render(&self, props: WidgetProps) -> Element {
        render_input::<Self>(RenderedInputType::Hidden, props)
    }
}

/// `<input type="checkbox">`, the default for a `bool`: box first, then its
/// caption.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CheckboxInput;

impl Widget for CheckboxInput {
    const NAME: &'static str = BuiltinWidget::Input(RenderedInputType::Checkbox).form_name();

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Control(FieldControl::Input(
        InputType::Checkbox,
    )));

    const PARTS: &'static [Tag] = &[Tag::Div, Tag::Input, Tag::Label];

    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Bool);

    /// Two states, and an `Option<bool>` has three.
    const ACCEPTS_OPTIONAL: bool = false;

    const TAKES_CHOICES: bool = false;

    fn render(&self, props: WidgetProps) -> Element {
        render_input::<Self>(RenderedInputType::Checkbox, props)
    }
}

/// Every `<input>` widget's `render`: `props` split into what `InputView`
/// takes. A plain fn, called from `render`, so it must not call hooks either.
fn render_input<W: Widget>(input_type: RenderedInputType, props: WidgetProps) -> Element {
    rsx! {
        InputView::<W> { input_type, props }
    }
}

#[component]
pub(crate) fn InputView<W: Widget>(
    input_type: RenderedInputType,
    props: WidgetProps,
    #[props(default)] _widget: PhantomData<W>,
) -> Element {
    // No wildcard arm, on purpose: a type that joins `RenderedInputType`
    // (`Range` is the likely one) has to be routed here before it compiles.
    match input_type {
        RenderedInputType::Hidden => hidden(props.path, props.values),
        RenderedInputType::Checkbox => checkbox(&props),
        RenderedInputType::Text
        | RenderedInputType::Password
        | RenderedInputType::Number
        | RenderedInputType::Email
        | RenderedInputType::Telephone
        | RenderedInputType::Url
        | RenderedInputType::Search
        | RenderedInputType::Color
        | RenderedInputType::Date
        | RenderedInputType::Time
        | RenderedInputType::DatetimeLocal
        | RenderedInputType::Month
        | RenderedInputType::Week => text_like::<W>(input_type, &props),
    }
}

/// A hidden input renders BARE. Every other body wraps the input with a
/// `<label>` caption and a required marker, which for `type="hidden"` would put
/// visible text and an asterisk on screen beside a widget nobody can see, and
/// label an unlabelable element for a screen reader.
fn hidden(path: String, values: ValuesStore) -> Element {
    let current = get_current(&path, values);
    rsx! {
        input {
            r#type: "hidden",
            name: "{path}",
            value: current,
            oninput: move |e: FormEvent| {
                let raw = e.value();
                write_value(&path, values, raw);
            },
        }
    }
}

// A PASSWORD IS AN ORDINARY CONTROLLED INPUT HERE. `type="password"` masks the
// glyphs; nothing else about it is special, and `value` is bound like every
// other field's.
//
// Django's `render_value=False` and Rails' non-echoing `password_field` are
// real conventions, but they are SERVER-RENDERING ones: there the value would
// land in an HTTP response body, and so in proxy logs, shared caches, browser
// history and view-source. No response body carries it here — the value goes
// keystroke -> DOM -> a wasm-side store in the user's own browser, and binding
// it back writes to the very node they typed into. The only viewer is the
// person who just typed it. A server-rendered response with a populated
// password would bring the concern back; this architecture produces none,
// because the SSR pass renders an empty form and every later re-render is
// client-side.
//
// Withholding it cost more than it bought: a field nothing could CLEAR
// programmatically (so "reset after a successful change" became impossible),
// and the only asymmetric input in the set.
fn text_like<W: Widget>(input_type: RenderedInputType, props: &WidgetProps) -> Element {
    let values = props.values;
    let path = props.path.clone();
    let current = get_current(&path, values);
    let attrs: Vec<Attribute> = props.attrs.merge_with_attrs(props.extra_attrs.clone());

    let control = rsx! {
        input {
            class: props.control_class::<W>(Tag::Input),
            r#type: input_type.to_input_type().html_type(),
            id: "{props.id}",
            name: "{path}",
            value: "{current}",
            oninput: move |e: FormEvent| {
                let raw = e.value();
                write_value(&path, values, raw);
            },
            // Last, and it has to be: rsx! treats everything after a spread
            // as children, so an attribute below this is a parse error.
            ..attrs,
        }
    };
    let marked = props.attrs.contains(Attr::Required);
    field_row::<W>(props, marked, LabelSide::Before, control)
}

fn checkbox(props: &WidgetProps) -> Element {
    let values = props.values;
    let path = props.path.clone();
    let attrs: Vec<Attribute> = props.attrs.merge_with_attrs(props.extra_attrs.clone());

    let control = rsx! {
        input {
            class: props.control_class::<CheckboxInput>(Tag::Input),
            name: "{path}",
            id: "{props.id}",
            r#type: "checkbox",
            checked: get_current(&path, values) == "true",
            onchange: move |e: FormEvent| write_value(&path, values, e.value()),
            // Last: rsx! reads anything after a spread as children.
            ..attrs,
        }
    };
    // Marked by `required_true`, not `required`: on a bool, `required` is
    // presence, which a checkbox always has. Until 2026-09-29 this widget
    // wrapped an unclassed `<label>` only when it had text, so
    // `.fx-form-field` and `.fx-field-label` both silently missed every
    // checkbox on the page; `field_row` is what keeps it in step now.
    let marked = props.attrs.contains(Attr::RequiredTrue);
    field_row::<CheckboxInput>(props, marked, LabelSide::After, control)
}
