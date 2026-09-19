//! The widget boundary: what turns a leaf into markup.
//!
//! One component per leaf, and that is the load-bearing part. A component is
//! the unit of reactivity in Dioxus: a store read inside one subscribes *that*
//! scope. `FormMember::render` is a plain function with no scope of its own, so
//! reading a value there would subscribe whoever called it — and a single
//! keystroke would re-render the entire form. Spawning a component per leaf is
//! what keeps a write to one path local to one input.

use dioxus::prelude::*;

use crate::error::FieldError;
use crate::label_case::{LabelCase, ToCase};
use crate::fields::ValueKind;
use crate::{Edit, ValuesByPath};

/// The per-field error list, rendered under every control.
///
/// `small` is Pico's convention for help text under an input, which is where
/// this lands; formoxus ships no stylesheet, so `field-errors`/`field-error`
/// are its own class names and any framework styles them like ordinary markup.
/// The only real cost is semantic — `small` means "fine print", which an error
/// message arguably is not. **When error rendering becomes overridable (the
/// same mechanism as custom widgets), this is the component to swap**, and the
/// choice stops being global.
///
/// The `aria-invalid` half lives on each widget's own control and is NOT a Pico
/// dependency: it is the W3C ARIA attribute assistive technology reads to
/// announce a field as errored. Leaving it off is an accessibility defect, not
/// a styling preference.
#[component]
pub fn FieldErrors(errors: Vec<FieldError>) -> Element {
    rsx! {
        if !errors.is_empty() {
            small { class: "field-errors",
                for error in errors.iter() {
                    span { class: "field-error", "{error.0}" }
                }
            }
        }
    }
}

#[derive(Clone)]
pub enum ControlType {
    Input(InputType),
    Textarea,
    Select,
    SelectMultiple,
    Checkbox,
    CheckboxMultiple,
    RadioGroup,
    File,
    /// A widget the author supplied, via `form!`'s `custom(MyWidget)`.
    ///
    /// The escape hatch for a value kind the built-in controls cannot serve —
    /// `Markdown` needs a live preview, a `Ref<Source>` needs an async-fed
    /// combobox. Neither is expressible as an `<input type=…>`, and neither
    /// belongs in `ValueKind`: they are presentation, not value family.
    ///
    /// **`fn` pointer, NOT `Box<dyn Fn>` — the derives force it.** `ControlType`
    /// is `Clone + Debug + PartialEq`, and a boxed closure supplies none of the
    /// three. A non-capturing closure coerces to a plain `fn`, which is `Copy`,
    /// clones trivially, and compares by address. That is what lets this variant
    /// exist without disturbing anything that already holds a `ControlType`.
    ///
    /// The cost is `Debug`: a fn pointer prints as an address. Hence `name`,
    /// which `form!` fills in from the widget's own path so panics and test
    /// assertions read `custom(MarkdownWidget)` rather than `0x7f…`.
    Custom {
        name: &'static str,
        render: fn(ControlProps) -> Element,
    },
}

// `Debug` and `PartialEq` are hand-written rather than derived, and only because
// of `Custom`'s `fn` pointer. Both impls reproduce the derive exactly for every
// other variant.
impl std::fmt::Debug for ControlType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(t) => write!(f, "Input({t:?})"),
            Self::Textarea => f.write_str("Textarea"),
            Self::Select => f.write_str("Select"),
            Self::SelectMultiple => f.write_str("SelectMultiple"),
            Self::Checkbox => f.write_str("Checkbox"),
            Self::CheckboxMultiple => f.write_str("CheckboxMultiple"),
            Self::RadioGroup => f.write_str("RadioGroup"),
            Self::File => f.write_str("File"),
            // `form!`'s own spelling, so the panic in `ScalarInput` quotes back
            // what the author wrote. Deriving this would print the fn pointer's
            // address beside the name, which is noise in every message it
            // appears in.
            Self::Custom { name, .. } => write!(f, "custom({name})"),
        }
    }
}

impl PartialEq for ControlType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Input(a), Self::Input(b)) => a == b,
            // Two custom controls are the same control when they name the same
            // widget. Comparing the `fn` pointers is what rustc warns about and
            // is genuinely meaningless here: identical functions may be merged
            // to one address, and one function may be duplicated across codegen
            // units, so neither equality nor inequality tells you anything about
            // which widget you have.
            (Self::Custom { name: a, .. }, Self::Custom { name: b, .. }) => a == b,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}

/// What a custom widget is handed: the same pair every built-in control gets.
///
/// One struct rather than two parameters because `render` is a `fn` pointer and
/// a single argument keeps that signature stable as the boundary grows.
///
/// Note this is the widget boundary the design notes fix — `(path, label,
/// required, errors)` in `props`, plus the value store — and NOT a
/// `FormField<T>`. A widget never sees the typed field: it reads and writes raw
/// strings through `values`, exactly as `HtmlInput` does.
#[derive(Clone, PartialEq)]
pub struct ControlProps {
    pub values: ValuesByPath,
    pub props: FieldProps,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputType {
    Text,
    Password,
    Hidden,
    Number,
    Email,
    Telephone,
    Url,
    Search,
    Color,
    Date,
    Time,
    DatetimeLocal,
    Month,
    Week,
}

impl InputType {
    /// The `type=` attribute this renders as.
    ///
    /// HTML's spelling, which is not always the variant's: `tel`, and
    /// `datetime-local` with the hyphen an ident could not carry. `form!`'s
    /// vocabulary spells these `tel` and `datetime_local`, so `InputType` is the
    /// pivot with HTML's names on both sides of it.
    pub fn html_type(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Password => "password",
            Self::Hidden => "hidden",
            // Selectable, but NOT the default for a numeric field — see
            // `FormField::default_control`. `type="number"` hands back `""` for
            // anything the browser dislikes, so a half-typed value vanishes
            // mid-keystroke. A numeric renders as text and `ValueKind` parses it.
            // Anyone who wants the spinner and the mobile keypad can ask.
            Self::Number => "number",
            Self::Email => "email",
            Self::Telephone => "tel",
            Self::Url => "url",
            Self::Search => "search",
            Self::Color => "color",
            Self::Date => "date",
            Self::Time => "time",
            Self::DatetimeLocal => "datetime-local",
            Self::Month => "month",
            Self::Week => "week",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldProps {
    pub path: String,
    pub label: Option<String>,
    // whether the field(s) below are considered required in the form,
    // modulo weird things like not being able to mark checkboxes required
    pub required: bool,
    pub errors: Vec<FieldError>,
}

/// This path's current raw value, or `""` if the map has no entry for it.
///
/// `pub` because a `custom(…)` widget lives in the CONSUMING crate and needs
/// exactly what the built-in controls use — without it, every custom widget
/// would reimplement the missing-key rule and get it subtly wrong. A path the
/// schema has but the map doesn't is normal, not an error: a variant chosen
/// after mount reveals leaves that were never populated, and an absent key
/// reads as empty, which is the same "empty IS absence" rule `apply_leaves`
/// follows.
pub fn get_current(path: &str, values: ValuesByPath) -> String {
    let slot = values.get_unchecked(path.to_string());
    slot.try_read().map(|v| v.clone()).unwrap_or_default()
}

/// Write this path's raw value back, inserting the key if it wasn't there.
///
/// `pub` for the same reason as [`get_current`]: a custom widget has to be able
/// to write, and the insert-vs-set distinction is not something each one should
/// have to rediscover.
pub fn write_value(path: &str, mut values: ValuesByPath, raw: String) {
    let populated = values.peek().contains_key(path);
    if populated {
        values.get_unchecked(path.to_string()).set(raw);
    } else {
        values.insert(path.to_string(), raw);
    }
}



/// A single-line text input bound to one path in the value map.
///
/// `values` + `path` rather than a pre-lensed child store, because a path that
/// the schema has but the map doesn't is a normal state, not an error: a
/// variant chosen after mount reveals leaves that were never populated. A missing
/// key reads as `""`, which is the same "empty IS absence" rule `apply_leaves`
/// already follows when a path is absent from submitted values.
#[component]
pub fn ScalarInput(
    value_kind: ValueKind,
    control: ControlType,
    values: ValuesByPath,
    props: FieldProps,
) -> Element {
    match (&value_kind, &control) {
        // One arm for every `<input type=…>`, over any value kind that is a
        // single scalar. The value crosses as a string either way — `ValueKind`
        // is what parses it back, and it is NOT consulted here on purpose, so
        // that a presentational override cannot change how a value is read.
        //
        // `Int`/`Float` land here too: their default control is `Text`, because
        // `type="number"` would eat a half-typed value — but `number` is a
        // perfectly good override, and so is `text` on a numeric.
        (
            ValueKind::Text { .. } | ValueKind::Int { .. } | ValueKind::Float,
            ControlType::Input(input_type),
        ) => {
            rsx! { HtmlInput { input_type: input_type.clone(), values, props } }
        }
        (ValueKind::Text { .. }, ControlType::Textarea) => {
            rsx! { TextareaInput { values, props } }
        }
        (ValueKind::Bool, ControlType::Checkbox) => {
            rsx! { BooleanInput { values, props } }
        }
        (ValueKind::Bool, ControlType::Select) => {
            rsx! { SelectInput { values, choices: bool_choices(), props } }
        }
        // Matches ANY value kind, deliberately. A custom widget exists precisely
        // because the built-in controls can't serve its type, so gating it on
        // the kinds we happen to enumerate would defeat it — `Markdown` and
        // `Ref<Source>` are `Text` to the parser and nothing to a `<select>`.
        // The author named this widget for this field; that IS the evidence.
        (_, ControlType::Custom { render, .. }) => render(ControlProps { values, props }),
        _ => panic!("{control:?} cannot render a {value_kind:?} (field {})", props.path),
    }
}

#[component]
pub fn HtmlInput(
    input_type: InputType,
    values: ValuesByPath,
    props: FieldProps,
) -> Element {
    let FieldProps { path, label: label_text, required, errors } = props;

    let current = get_current(&path, values);

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
    // and the only asymmetric widget in the set.

    // Present ONLY when there is an error. `aria-invalid="false"` is NOT the
    // neutral value — it asserts "checked, and passed", which Pico duly paints
    // green with a tick, so an untouched form would claim to have validated
    // every field. Absent is the only neutral state. Dioxus omits an attribute
    // whose value is `None`, which is what makes absence expressible at all.
    //
    // Unlike the `small` in `FieldErrors`, this is not a styling choice with a
    // framework behind it: `aria-invalid` is the W3C ARIA attribute assistive
    // technology reads to announce a field as errored, so it belongs here
    // whatever CSS the consumer brings.
    let invalid = (!errors.is_empty()).then_some("true");

    // A hidden input renders BARE. The wrapper below is a `label` with a caption
    // and a required marker, which for `type="hidden"` would put visible text
    // and an asterisk on screen beside a control nobody can see — and label an
    // unlabelable element for a screen reader.
    if matches!(input_type, InputType::Hidden) {
        return rsx! {
            input {
                r#type: "hidden",
                name: "{path}",
                value: current,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
            }
        };
    }

    rsx! {
        label { class: "form-field",
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
            }
            if required {
                span { class: "required", " *" }
            }
            input {
                r#type: input_type.html_type(),
                name: "{path}",
                value: "{current}",
                required,
                aria_invalid: invalid,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
            }
            FieldErrors { errors }
        }
    }
}

/// A multi-line text input bound to one path in the value map.
///
/// Otherwise identical to [`HtmlInput`] — same controlled-value binding, same
/// `aria-invalid` rule, same label layout. There is no hidden-input branch to
/// mirror: `Textarea` is only ever chosen as an override on a `Text` field, and
/// nothing here needs the extra `InputType` cases (`Password`'s masking,
/// `Hidden`'s bare markup) that make `HtmlInput` carry one.
#[component]
pub fn TextareaInput(
    values: ValuesByPath,
    props: FieldProps,
) -> Element {
    let FieldProps { path, label: label_text, required, errors } = props;

    let current = get_current(&path, values);

    let invalid = (!errors.is_empty()).then_some("true");

    rsx! {
        label { class: "form-field",
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
            }
            if required {
                span { class: "required", " *" }
            }
            textarea {
                name: "{path}",
                value: "{current}",
                required,
                aria_invalid: invalid,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
            }
            FieldErrors { errors }
        }
    }
}

#[component]
pub fn BooleanInput(
    mut values: ValuesByPath,
    props: FieldProps,
) -> Element {
    // An `Option<bool>` has three states and a checkbox has two, so it needs a
    // select. Delegating rather than inlining one keeps a single implementation
    // of the "no value" option and the required/optional asymmetry.

    let FieldProps { path, label, errors, .. } = props;

    // `required` is deliberately dropped rather than forwarded. HTML `required`
    // on a checkbox means "must be ticked", which is not what a required `bool`
    // field asks for — unticked is a complete answer. For the same reason there
    // is no ` *` marker: it would promise a rule nothing enforces.
    // See `TextInput`. Pico skips a checkbox for the invalid *icon* (there is
    // nowhere to put one), but the border and the adjacent `small` still key off
    // this, and it is what a screen reader announces either way.
    let invalid = (!errors.is_empty()).then_some("true");

    let input_element = rsx! {
        input {
            name: "{path}",
            r#type: "checkbox",
            checked: get_current(&path, values) == "true",
            aria_invalid: invalid,
            onchange: move |e: FormEvent| write_value(&path, values, e.value())
        }
        FieldErrors { errors: errors.clone() }
    };

    if let Some(label_text) = label.clone() {
        rsx! {
            label {
                "{label_text}"
                { input_element }
            }
        }
    } else {
        input_element
    }
}

/// One option in a [`SelectInput`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectChoice {
    /// The raw string written into the value map, so it has to be exactly what
    /// `parse_scalar` expects for this field's type — `"true"`, not `"True"`.
    /// That the two can differ at all is why this isn't just a `Vec<String>`.
    pub value: String,
    /// What the user reads.
    pub display: String,
}

impl SelectChoice {
    pub fn new(value: impl Into<String>, display: impl Into<String>) -> Self {
        Self { value: value.into(), display: display.into() }
    }
}

/// The three states of an `Option<bool>`, minus the absent one — that comes
/// from `SelectInput`'s own "no value" option, so it is spelled in exactly one
/// place rather than once per caller.
fn bool_choices() -> Vec<SelectChoice> {
    vec![SelectChoice::new("true", "True"), SelectChoice::new("false", "False")]
}

/// A `<select>` over a fixed set of choices, bound to one path in the value map.
///
/// Reads its current value from `values` like every other leaf widget rather
/// than taking it as a prop. That is not just consistency: computing `selected`
/// for a prop would mean reading the store in `FormField::render`, a plain
/// function with no scope of its own, which subscribes *the caller* — so one
/// change here would re-render the whole form. [`VariantSelect`] takes its
/// selection as a prop precisely because a variant choice is NOT a leaf and has
/// no path to read.
///
/// An option's value is the raw string itself, NOT an index into `choices`.
/// Indexing is the usual dodge for a `T` that might not survive a round trip
/// through a string, but here `T -> String -> T` is a guaranteed identity for
/// every builtin scalar (see the `roundtrip` tests), so the indirection — and
/// the silent `unwrap_or_default()` it needs when an index doesn't match — buys
/// nothing and loses the value.
#[component]
pub fn SelectInput(
    values: ValuesByPath,
    choices: Vec<SelectChoice>,
    props: FieldProps,
) -> Element {
    let FieldProps { path, label: label_text, required, errors } = props;

    let current = get_current(&path, values);

    // See `TextInput` for why this is `Option` rather than a plain bool.
    let invalid = (!errors.is_empty()).then_some("true");

    rsx! {
        label { class: "form-field",
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
            }
            if required {
                span { class: "required", " *" }
            }
            select {
                aria_invalid: invalid,
                // Unlike `VariantSelect`, this one IS a leaf, so it must carry a
                // `name` or `apply_form_values` would never see it.
                name: "{path}",
                required,
                // No branch on emptiness: the "no value" option's value is `""`,
                // and `""` IS absence at both boundaries, so the same write does
                // for every option.
                onchange: move |e: FormEvent| write_value(&path, values, e.value()),
                // Required and unanswered: an unselectable placeholder, so the
                // browser's own validation blocks submit and the user can't
                // choose their way back to "unanswered". Optional: a real
                // selectable entry, because absent is a legitimate answer.
                if required && current.is_empty() {
                    option { value: "", selected: true, disabled: true, hidden: true, "Choose..." }
                } else if !required {
                    option { value: "", selected: current.is_empty(), "{ABSENT_DISPLAY}" }
                }
                for choice in choices {
                    option {
                        value: "{choice.value}",
                        selected: choice.value == current,
                        "{choice.display}"
                    }
                }
            }
            FieldErrors { errors }
        }
    }
}

/// The "leave this out" entry in an optional enum's picker.
///
/// Display only, and it stays that way for a structural reason rather than a
/// cosmetic one: the `<select>` carries no `name`, so nothing it holds is ever
/// collected by `FormData::values()` and this text cannot come back as a value.
/// That is what keeps it from reintroducing the sentinel problem
/// [`VariantChoice`](crate::VariantChoice) exists to avoid — a model
/// with a genuine `None` variant would otherwise be indistinguishable from an
/// unanswered optional field. What the select actually emits is `""`, which
/// `VariantSelect` turns into `ChooseVariant { variant: None }`.
///
/// [`SelectInput`] shows the same text, and it *does* carry a `name` — but it is
/// safe there for the same reason by a different route: the option's value is
/// `""`, never this text, and `""` is absence at both boundaries.
pub(crate) const ABSENT_DISPLAY: &str = "--none--";

#[component]
pub fn VariantSelect(
    path: String,
    label: Option<String>,
    required: bool,
    errors: Vec<FieldError>,
    variants: Vec<&'static str>,
    selected: Option<String>,
    on_edit: Callback<Edit>,
) -> Element {
    let label_text = label;

    rsx! {
        label { class: "form-field",
            // The star annotates the LABEL, so it only appears when there is
            // one. Rendered inside a `VariantSet`'s fieldset there isn't: the
            // legend carries both, and a lone `*` floating in front of the
            // select reads as belonging to nothing.
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
                if required {
                    span { class: "required", " *" }
                }
            }
            select {
                required,
                onchange: move |e: FormEvent| {
                    let v = e.value();
                    let variant = (!v.is_empty()).then_some(v);
                    on_edit.call(Edit::new_choose_variant(&path, variant.as_deref()));
                },
                // Required + unchosen: an unselectable placeholder that keeps the browser's
                // own validation on the hook. Not required: a real "--none--" the user can
                // pick, which routes through the empty arm above to Unchosen.
                if required && selected.is_none() {
                    option { value: "", selected: true, disabled: true, hidden: true, "Choose..." }
                } else if !required {
                    option { value: "", selected: selected.is_none(), "{ABSENT_DISPLAY}" }
                }
                for v in variants {
                    option {
                        value: "{v}",
                        selected: selected.as_deref() == Some(v),
                        "{v.to_case(LabelCase::Title)}"
                    }
                }
            }
            FieldErrors { errors }
        }
    }
}

/// The control that appends a row to a list.
///
/// Like [`VariantSelect`], it reads nothing from the value store — adding a row
/// is a change to the form's *shape*, so all it does is put an [`Edit`] on the
/// wire. `type="button"` is load-bearing: inside a `<form>` a bare `<button>`
/// defaults to `type="submit"`, so omitting it would submit the form instead of
/// adding a row.
#[component]
pub fn AddRowButton(path: String, on_edit: Callback<Edit>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "add-row",
            onclick: move |_| {
                // Append. `before` exists for mid-list insertion, which needs a
                // control between every pair of rows — a UI question that hasn't
                // been answered yet, not a limitation of the edit.
                on_edit.call(Edit::AddRow { path: path.clone(), before: None });
            },
            "Add"
        }
    }
}

/// The control that drops one row from a list.
///
/// Addressed by POSITION, not by the row's key: the list is what applies the
/// edit and it works in terms of `rows`, so a position is what it can act on
/// directly. Keys identify a row across edits; a position locates one at an
/// instant, which is all a click needs to say.
#[component]
pub fn RemoveRowButton(path: String, index: usize, on_edit: Callback<Edit>) -> Element {
    // 1-based for humans: this is the only place a row's position is spoken
    // aloud, and it is never used as a path segment.
    let ordinal = index + 1;
    rsx! {
        button {
            r#type: "button",
            class: "remove-row",
            aria_label: "Remove row {ordinal}",
            onclick: move |_| {
                on_edit.call(Edit::RemoveRow { path: path.clone(), index });
            },
            "Remove"
        }
    }
}
