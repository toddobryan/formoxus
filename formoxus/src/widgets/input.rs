//! The `<input>` family: every `type=` formoxus renders, routed by
//! its `InputBody` to one of three bodies. Text-like types (whose value
//! the DOM hands back as plain text) read caption-then-input; a checkbox
//! reads box-then-caption and binds `checked`; a hidden input renders bare.

use std::marker::PhantomData;

use dioxus::prelude::*;
use formoxus_spec::{
    Attr, AttrSet, AttrValue, Bound, FieldType, FieldTypeSet, HtmlElement, InputType, Step, Tag,
    WidgetName,
};

use super::field_row::{LabelSide, field_row};
use crate::field_attrs::{FieldAttrs, float_bound, int_bound};
use crate::widgets::{Widget, widget::LeafWidget, widget_props::WidgetProps};

/// One [`Widget`] per text-like `<input type=…>`, each a struct with no props.
///
/// Everything a type needs follows from its name and its `InputType`
/// variant. `NAME` is `form!`'s spelling, read from the `WidgetName` variant
/// of the same name, so the two cannot drift. `EXTENDS` is what HTML allows on
/// that `<input>`, and `PARTS`, `ACCEPTS` and `render` are shared. Hidden and
/// checkbox render differently, so they are written by hand.
macro_rules! text_like_inputs {
    ($( $(#[$doc:meta])* $widget:ident => $variant:ident, )+) => {
        $(
            $(#[$doc])*
            #[derive(Clone, Debug, PartialEq, Default)]
            pub struct $widget;

            impl Widget for $widget {
                const NAME: &'static str =
                    WidgetName::$widget.form_name();

                const PARTS: &'static [Tag] = &[Tag::Div, Tag::Label, Tag::Input];

                const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Input(InputType::$variant));
            }

            impl LeafWidget for $widget {
                /// Never `Bool`: a text box cannot say true or false. A number
                /// is accepted, since the field parses the text back.
                const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Text).with(FieldType::Int).with(FieldType::Float);

                const ACCEPTS_OPTIONAL: bool = true;

                const TAKES_CHOICES: bool = false;

                fn render(&self, props: WidgetProps) -> Element {
                    render_input::<Self>(InputBody::TextLike(InputType::$variant), props)
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
    const NAME: &'static str = WidgetName::HiddenInput.form_name();

    const EXTENDS: AttrSet = AttrSet::EMPTY;

    const PARTS: &'static [Tag] = &[];
}

impl LeafWidget for HiddenInput {
    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY
        .with(FieldType::Text)
        .with(FieldType::Int)
        .with(FieldType::Float);

    const ACCEPTS_OPTIONAL: bool = true;

    const TAKES_CHOICES: bool = false;

    fn render(&self, props: WidgetProps) -> Element {
        render_input::<Self>(InputBody::Hidden, props)
    }
}

/// `<input type="checkbox">`, the default for a `bool`: box first, then its
/// caption.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CheckboxInput;

impl Widget for CheckboxInput {
    const NAME: &'static str = WidgetName::CheckboxInput.form_name();

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Input(InputType::Checkbox));

    const PARTS: &'static [Tag] = &[Tag::Div, Tag::Input, Tag::Label];
}

impl LeafWidget for CheckboxInput {
    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Bool);

    /// Two states, and an `Option<bool>` has three.
    const ACCEPTS_OPTIONAL: bool = false;

    const TAKES_CHOICES: bool = false;

    fn render(&self, props: WidgetProps) -> Element {
        render_input::<Self>(InputBody::Checkbox, props)
    }
}

/// `<input type="range" ...>`
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RangeInput {
    pub omit_value: bool,
}

impl Widget for RangeInput {
    const NAME: &'static str = WidgetName::RangeInput.form_name();

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Input(InputType::Range));

    const PARTS: &'static [Tag] = &[Tag::Div, Tag::Label, Tag::Input, Tag::Output];
}

impl LeafWidget for RangeInput {
    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY
        .with(FieldType::Int)
        .with(FieldType::Float);

    /// A slider can't be unset, so is not optional
    const ACCEPTS_OPTIONAL: bool = false;

    const TAKES_CHOICES: bool = false;

    fn render(&self, props: WidgetProps) -> Element {
        render_input::<Self>(
            InputBody::Range {
                omit_value: self.omit_value,
            },
            props,
        )
    }

    /// Where the browser draws an empty range, so the field holds what the user
    /// sees (`WIDGETS_PLAN.md` decision A).
    ///
    /// HTML's rule: the default value is `min + (max − min) / 2`, and a value
    /// off the step moves to the NEAREST step counted from `min`, the larger of
    /// two equally near. With `span = max − min` and step `s`, that is
    /// `min + k·s` for `k = floor((span + s) / (2·s))`. A range's step is 1
    /// unless given. `None` without both bounds, which `form!` refuses, so only
    /// a hand-built spec gets an empty field there.
    fn value_when_empty(&self, attrs: &FieldAttrs) -> Option<String> {
        let (Some(AttrValue::Bound(min)), Some(AttrValue::Bound(max))) =
            (attrs.get(Attr::Min), attrs.get(Attr::Max))
        else {
            return None;
        };
        let step = match attrs.get(Attr::Step) {
            Some(AttrValue::Step(Step::By(step))) => Some(*step),
            // `any`: no snapping, so the midpoint itself (on an int field,
            // stepping by 1, since an int cannot hold the half).
            _ => None,
        };
        // `resolve_bounds` has made both bounds the field's kind, so an int
        // field's are `Int` and a float field's `Float`. A step is not
        // resolved, so it is converted here.
        match (*min, *max) {
            (Bound::Int(min), Bound::Int(max)) => {
                let s = step.map_or(1, |step| int_bound(step, "step")).max(1);
                range_start_int(min, max, s).map(|v| v.to_string())
            }
            (min, max) => {
                let (min, max) = (float_bound(min), float_bound(max));
                let start = match (step, attrs.get(Attr::Step)) {
                    (Some(step), _) => range_start_float(min, max, float_bound(step)),
                    (None, Some(AttrValue::Step(Step::Any))) => range_start_any(min, max),
                    (None, _) => range_start_float(min, max, 1.0),
                };
                Some(start.to_string())
            }
        }
    }
}

/// [`RangeInput::value_when_empty`] for an integer field, exactly. `max < min`
/// starts at `min`, as HTML does; `None` only if the arithmetic overflows,
/// which no bound of a Rust integer type can make it do.
fn range_start_int(min: i128, max: i128, s: i128) -> Option<i128> {
    let span = max.checked_sub(min)?;
    if span < 0 {
        return Some(min);
    }
    let k = span.checked_add(s)?.div_euclid(s.checked_mul(2)?);
    let start = min.checked_add(k.checked_mul(s)?)?;
    // The nearer step never lies past `max` (it is below the midpoint's
    // mirror image), but a step past it is never drawn, so guard anyway.
    Some(if start > max { start - s } else { start })
}

/// As [`range_start_int`], for a float field with a positive step.
fn range_start_float(min: f64, max: f64, s: f64) -> f64 {
    if max < min || s <= 0.0 {
        return min;
    }
    let k = ((max - min + s) / (2.0 * s)).floor();
    let start = min + k * s;
    if start > max { start - s } else { start }
}

/// `step: any` on a float field: the midpoint, unsnapped.
fn range_start_any(min: f64, max: f64) -> f64 {
    if max < min {
        min
    } else {
        min + (max - min) / 2.0
    }
}

/// Which of `InputView`'s four bodies a widget renders. Each widget names
/// its own, so a new `<input>` widget is routed where it is declared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum InputBody {
    Hidden,
    Checkbox,
    TextLike(InputType),
    Range { omit_value: bool },
}

/// Every `<input>` widget's `render`: `props` split into what `InputView`
/// takes. A plain fn, called from `render`, so it must not call hooks either.
fn render_input<W: Widget>(body: InputBody, props: WidgetProps) -> Element {
    rsx! {
        InputView::<W> { body, props }
    }
}

#[component]
pub(crate) fn InputView<W: Widget>(
    body: InputBody,
    props: WidgetProps,
    #[props(default)] widget: PhantomData<W>,
) -> Element {
    match body {
        InputBody::Hidden => hidden(&props),
        InputBody::Checkbox => checkbox(&props),
        InputBody::TextLike(input_type) => text_like::<W>(input_type, &props),
        InputBody::Range { omit_value } => range(omit_value, &props),
    }
}

/// A hidden input renders BARE. Every other body wraps the input with a
/// `<label>` caption and a required marker, which for `type="hidden"` would put
/// visible text and an asterisk on screen beside a widget nobody can see, and
/// label an unlabelable element for a screen reader.
fn hidden(props: &WidgetProps) -> Element {
    let handle = props.field_handle();
    let current = handle.current();
    rsx! {
        input {
            r#type: "hidden",
            name: "{props.path}",
            value: current,
            oninput: handle.on_input(),
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
fn text_like<W: Widget>(input_type: InputType, props: &WidgetProps) -> Element {
    let handle = props.field_handle();
    let current = handle.current();
    let attrs: Vec<Attribute> = props
        .current_attrs()
        .merge_with_attrs(props.extra_attrs.clone());

    let control = rsx! {
        input {
            class: props.control_class::<W>(Tag::Input),
            r#type: input_type.html_type(),
            id: "{props.id}",
            name: "{props.path}",
            value: "{current}",
            oninput: handle.on_input(),
            onblur: handle.on_blur(),
            // Last, and it has to be: rsx! treats everything after a spread
            // as children, so an attribute below this is a parse error.
            ..attrs,
        }
    };
    let marked = props.base_attrs.contains(Attr::Required);
    field_row::<W>(props, marked, LabelSide::Before, control)
}

fn checkbox(props: &WidgetProps) -> Element {
    let handle = props.field_handle();
    let attrs: Vec<Attribute> = props
        .current_attrs()
        .merge_with_attrs(props.extra_attrs.clone());

    let control = rsx! {
        input {
            class: props.control_class::<CheckboxInput>(Tag::Input),
            name: "{props.path}",
            id: "{props.id}",
            r#type: "checkbox",
            checked: handle.current() == "true",
            onchange: handle.on_change(),
            // Last: rsx! reads anything after a spread as children.
            ..attrs,
        }
    };
    // Marked by `required_true`, not `required`: on a bool, `required` is
    // presence, which a checkbox always has. Until 2026-09-29 this widget
    // wrapped an unclassed `<label>` only when it had text, so
    // `.fx-form-field` and `.fx-field-label` both silently missed every
    // checkbox on the page; `field_row` is what keeps it in step now.
    let marked = props.base_attrs.contains(Attr::RequiredTrue);
    field_row::<CheckboxInput>(props, marked, LabelSide::After, control)
}

fn range(omit_value: bool, props: &WidgetProps) -> Element {
    let handle = props.field_handle();
    let current = handle.current();
    // Out of the spread, as `RadioGroup` takes it off its `<fieldset>`: HTML does
    // not allow `required` on a range, and a slider always has a value, so it
    // would enforce nothing. It stays in `base_attrs`, where it says the FIELD
    // is required, which is how every widget learns that.
    let attrs: Vec<Attribute> = props
        .current_attrs()
        .without(Attr::Required)
        .merge_with_attrs(props.extra_attrs.clone());

    let control = rsx! {
        input {
            class: props.control_class::<RangeInput>(Tag::Input),
            r#type: "range",
            name: "{props.path}",
            id: "{props.id}",
            value: "{current}",
            oninput: handle.on_input(),
            onchange: handle.on_change(),
            ..attrs
        }
        if !omit_value {
            output {
                class: props.part_class::<RangeInput>(Tag::Output),
                "for": "{props.id}",
                "{current}"
            }
        }
    };
    // Range always has a value, so it's not marked required
    let marked = false;
    field_row::<RangeInput>(props, marked, LabelSide::Before, control)
}
