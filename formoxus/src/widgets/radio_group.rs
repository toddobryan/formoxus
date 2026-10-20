//! Radio buttons over a fixed set of choices.

use dioxus::prelude::*;
use formoxus_spec::{Attr, AttrSet, BuiltinWidget, HtmlElement};

use crate::classes::Slot;
use crate::field_attrs::FieldAttrs;
use crate::nodes::ValuesStore;
use crate::widgets::label::required_marker;
use crate::widgets::types::bool_choices;
use crate::widgets::{
    Choice, FieldErrors, FieldProps, Widget, get_current, widget_props::WidgetProps, write_value,
};

/// Radio buttons, one per choice, for a REQUIRED field: a picked radio
/// can't be un-picked. Over a `bool` without choices it offers true and
/// false.
///
/// Its class slots are the fieldset's parts. Per-field `group_class` and
/// `input_class` props (W-A) come with `form!`'s props box, where a dev can
/// first write one.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RadioGroup;

impl Widget for RadioGroup {
    const NAME: &'static str = BuiltinWidget::RadioGroup.form_name();

    /// The `<fieldset>`'s, where the spread lands.
    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Fieldset);

    fn render(&self, props: WidgetProps) -> Element {
        reject_if_not_required(&props.attrs, &props.path);
        rsx! {
            RadioGroupView {
                field_attrs: props.attrs,
                values: props.values,
                choices: props.choices.unwrap_or_else(bool_choices),
                props: FieldProps {
                    id: props.id,
                    path: props.path,
                    label: props.label,
                    classes: props.classes,
                    errors: props.errors,
                },
                attrs: props.extra_attrs,
            }
        }
    }

    fn default_classes() -> Vec<(&'static str, Slot)> {
        vec![
            (
                "fieldset",
                Slot::new("fx-form-field fx-radio-group").with_invalid("fx-invalid"),
            ),
            ("legend", Slot::new("fx-legend fx-field-label")),
            ("div", Slot::new("fx-choice")),
            ("input", Slot::new("fx-control fx-radio")),
            ("label", Slot::new("fx-choice-label")),
        ]
    }
}

/// A radio group cannot say "no answer", so an optional field is refused.
/// `form!` refuses it at compile time; this catches a hand-built spec.
pub(crate) fn reject_if_not_required(attrs: &FieldAttrs, path: &str) {
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
/// rejected outright, at compile time by `field_kind::is_optional` and at
/// render by `ScalarWidget`. In create mode nothing is checked and `required`
/// makes the browser block submit until the user picks; in edit mode the stored
/// value comes back as `current` and checks its own radio.
#[component]
pub(crate) fn RadioGroupView(
    field_attrs: FieldAttrs,
    values: ValuesStore,
    choices: Vec<Choice>,
    props: FieldProps,
    #[props(extends = fieldset)] attrs: Vec<Attribute>,
) -> Element {
    let my_widget = BuiltinWidget::RadioGroup;
    let is_invalid = props.is_invalid();

    // TODO: figure out where to put the attributes. Currently in the fieldset,
    //       but we might want people to be able to set attrs on each input

    // `required` and `aria-invalid` are both judgments about the GROUP, but each
    // spec puts its own in a different place. HTML has `required` only on an
    // `<input>`, never a `<fieldset>`, and one required radio makes the whole
    // group required; so it goes on each radio and out of the spread. WAI-ARIA
    // 1.2 defines `aria-invalid` on role `radiogroup` and NOT on `radio`; so it
    // stays in the spread, which lands on the fieldset given that role below.
    let required = field_attrs.contains(Attr::Required);
    let attrs: Vec<Attribute> = field_attrs.without(Attr::Required).merge_with_attrs(attrs);

    let field_class = props.classes.part_class(my_widget, "fieldset", is_invalid);

    let current = get_current(&props.path, values);

    let div_class = props.classes.part_class(my_widget, "div", is_invalid);
    let input_class = props.classes.part_class(my_widget, "input", is_invalid);
    let label_class = props.classes.part_class(my_widget, "label", is_invalid);
    let legend_class = props.classes.part_class(my_widget, "legend", is_invalid);

    // Built out here rather than in a `for` inside the `rsx!`, because each
    // radio needs its OWN `path` to move into its own `onchange` and an rsx
    // loop body takes nodes, not statements. `FieldSet` splices its children
    // the same way.
    let radios: Vec<Element> = choices
        .into_iter()
        .enumerate()
        .map(|(index, choice)| {
            let path = props.path.clone();
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
                        onchange: move |e: FormEvent| write_value(&path, values, e.value()),
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
                    if field_attrs.contains(Attr::Required) {
                        {required_marker(&props.classes)}
                    }
                }
            }
            { radios.into_iter() }
            FieldErrors { field_id: &props.id, errors: props.errors, classes: props.classes.clone() }
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
        reject_if_not_required(&FieldAttrs::default(), "title");
    }

    #[gtest]
    fn a_required_field_is_accepted() {
        reject_if_not_required(
            &FieldAttrs::from([(AttrKey::Std(Attr::Required), AttrValue::Flag)]),
            "title",
        );
    }
}
