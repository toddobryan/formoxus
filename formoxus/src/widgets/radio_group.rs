//! Radio buttons over a fixed set of choices.

use dioxus::prelude::*;
use formoxus_spec::{Attr, AttrSet, FieldType, FieldTypeSet, HtmlElement, Tag, WidgetName};

use crate::field_attrs::FieldAttrs;
use crate::path::NodePath;
use crate::widgets::field_row::required_marker;
use crate::widgets::types::bool_choices;
use crate::widgets::{FieldErrors, Widget, field_help, widget_props::WidgetProps};

/// Radio buttons, one per choice, for a REQUIRED field: a picked radio
/// can't be un-picked. Over a `bool` without choices it offers true and
/// false.
///
/// Its class slots are the fieldset's parts. A field's own `class` and
/// `class_plus` go on the `<fieldset>`, where its other attributes land; the
/// inner elements are restyled for every radio group at once, through
/// `Classes` or a form's `classes_plus:`, or per field by CSS from the
/// fieldset's class.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RadioGroup;

impl Widget for RadioGroup {
    const NAME: &'static str = WidgetName::RadioGroup.form_name();

    /// The `<fieldset>`'s, where the spread lands.
    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Fieldset);

    const PARTS: &'static [Tag] = &[Tag::Fieldset, Tag::Legend, Tag::Div, Tag::Input, Tag::Label];

    /// `Bool` on its own, offering true and false; any other type only with
    /// `choices`.
    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Bool);

    /// A picked radio cannot be un-picked, so there is no way back to "no
    /// value". Also enforced at render, by `reject_if_not_required`.
    const ACCEPTS_OPTIONAL: bool = false;

    const TAKES_CHOICES: bool = true;

    fn render(&self, props: WidgetProps) -> Element {
        reject_if_not_required(&props.base_attrs, &props.path);
        rsx! {
            RadioGroupView { widget_props: props }
        }
    }
}

/// A radio group cannot say "no answer", so an optional field is refused.
/// `form!` refuses it at compile time; this catches a hand-built spec.
pub(crate) fn reject_if_not_required(attrs: &FieldAttrs, path: &NodePath) {
    assert!(
        attrs.contains(Attr::Required),
        "do not use `radio_group` because the field {path} is not required - use `select` instead",
    );
}

/// A group of radio buttons over a fixed set of choices, bound to one path.
///
/// Reads its current value from `values` rather than taking it as a prop, for
/// the reason `SelectView` spells out: computing `checked` for a
/// prop would mean reading the store in `Field::render`, which has no scope
/// of its own and so would subscribe the whole form.
///
/// **Nothing is pre-selected, and there is no "none" radio.** A picked radio
/// can't be un-picked, so a synthetic absent option is the only way back — and
/// `--none--` is a poor stand-in for the explicit "None of the above" that a
/// genuinely unanswerable question wants. That is also why an optional field is
/// rejected outright, at compile time by `field_kind::is_optional` and, for a
/// hand-built `FormSpec`, at render by [`reject_if_not_required`]. In create
/// mode nothing is checked and `required` makes the browser block submit until
/// the user picks; in edit mode the stored value comes back as `current` and
/// checks its own radio.
#[component]
pub(crate) fn RadioGroupView(widget_props: WidgetProps) -> Element {
    // Not `props`: Dioxus reads a component's lone `props` parameter as its
    // whole props struct, which `WidgetProps` is not.
    let props = widget_props;
    let handle = props.field_handle();
    let is_invalid = props.is_invalid();
    let base_attrs = &props.base_attrs;
    let current_attrs = props.current_attrs();
    let choices = props.choices.unwrap_or_else(bool_choices);

    // `required` and `aria-invalid` are both judgments about the GROUP, but each
    // spec puts its own in a different place. HTML has `required` only on an
    // `<input>`, never a `<fieldset>`, and one required radio makes the whole
    // group required; so it goes on each radio and out of the spread. WAI-ARIA
    // 1.2 defines `aria-invalid` on role `radiogroup` and NOT on `radio`; so it
    // stays in the spread, which lands on the fieldset given that role below.
    let required = base_attrs.contains(Attr::Required);
    let attrs: Vec<Attribute> = current_attrs
        .without(Attr::Required)
        .merge_with_attrs(props.extra_attrs);

    let field_class = base_attrs.class(props.classes.part::<RadioGroup>(Tag::Fieldset), is_invalid);

    let current = handle.current();

    let div_class = props.classes.part_class::<RadioGroup>(Tag::Div, is_invalid);
    let input_class = props
        .classes
        .part_class::<RadioGroup>(Tag::Input, is_invalid);
    let label_class = props
        .classes
        .part_class::<RadioGroup>(Tag::Label, is_invalid);
    let legend_class = props
        .classes
        .part_class::<RadioGroup>(Tag::Legend, is_invalid);

    // Built out here rather than in a `for` inside the `rsx!`, because each
    // radio needs its OWN `path` to move into its own `onchange` and an rsx
    // loop body takes nodes, not statements. `StructNode` splices its children
    // the same way.
    let radios: Vec<Element> = choices
        .into_iter()
        .enumerate()
        .map(|(index, choice)| {
            rsx! {
                div {
                    class: &div_class,
                    input {
                        class: &input_class,
                        r#type: "radio",
                        // Every radio in the group carries the SAME name, but
                        // a different id: that is what makes them one group to
                        // the browser, and what `distribute_form_values` collects
                        // the pick under, but allows each label to refer to just
                        // its choice.
                        id: "{props.id}-{index}",
                        name: "{props.path}",
                        value: "{choice.value}",
                        checked: choice.value == current,
                        // On every radio, though one would do for the browser:
                        // each then announces "required" when it has focus.
                        required,
                        onchange: handle.on_change(),
                    }
                    label {
                        class: &label_class,
                        r#for: "{props.id}-{index}",
                        "{choice.display}"
                    }
                }
            }
        })
        .collect();

    rsx! {
        fieldset {
            class: field_class,
            // A bare `<fieldset>` is role `group`, where ARIA leaves
            // `aria-invalid` undefined.
            role: "radiogroup",
            ..attrs,
            // The group's label is the `legend`, not a `label` — a `<label>`
            // can only name a single control, and there are several here.
            legend { class: legend_class,
                // No label, no marker — an asterisk with nothing to qualify
                // means nothing. `aria-hidden` because it is visual only: on a
                // radio group `required` applies to the whole group and that is
                // what a screen reader announces.
                if let Some(text) = props.label {
                    "{text}"
                    if required {
                        {required_marker(&props.classes)}
                    }
                }
            }
            // After the legend, as `field_row` puts it after a caption; the
            // fieldset's `aria-describedby`, from `current_attrs`, names it.
            {field_help(&props.id, props.help_text.as_deref(), &props.classes)}
            { radios.into_iter() }
            FieldErrors {
                field_id: &props.id,
                paths: vec![props.path.clone()],
                errors: props.errors,
                classes: props.classes.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formoxus_spec::{AttrKey, AttrValue};
    use googletest::prelude::*;

    // Tested here, not through `RadioGroup::render` in the suite: Dioxus
    // catches a panic raised while a component renders, so `should_panic`
    // never sees it there.

    #[gtest]
    #[should_panic(expected = "the field title is not required")]
    fn a_field_that_is_not_required_is_refused() {
        reject_if_not_required(&FieldAttrs::default(), &NodePath::new("title"));
    }

    #[gtest]
    fn a_required_field_is_accepted() {
        reject_if_not_required(
            &FieldAttrs::from([(AttrKey::Std(Attr::Required), AttrValue::Flag)]),
            &NodePath::new("title"),
        );
    }
}
