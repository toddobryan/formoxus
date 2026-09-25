//! Radio buttons over a fixed set of choices.

use dioxus::prelude::*;

use crate::ValuesByPath;
use crate::widgets::{FieldErrors, FieldProps, SelectChoice, get_current, write_value};

/// A group of radio buttons over a fixed set of choices, bound to one path.
///
/// Reads its current value from `values` rather than taking it as a prop, for
/// the reason [`Select`](super::Select) spells out: computing `checked` for a
/// prop would mean reading the store in `FormField::render`, which has no scope
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
pub fn RadioGroup(values: ValuesByPath, choices: Vec<SelectChoice>, props: FieldProps) -> Element {
    let FieldProps {
        path,
        label: label_text,
        required,
        errors,
    } = props;

    let current = get_current(&path, values);

    // See `Input` for why this is `Option` rather than a plain bool.
    let invalid = (!errors.is_empty()).then_some("true");

    // Built out here rather than in a `for` inside the `rsx!`, because each
    // radio needs its OWN `path` to move into its own `onchange` and an rsx
    // loop body takes nodes, not statements. `FieldSet` splices its children
    // the same way.
    let radios: Vec<Element> = choices
        .into_iter()
        .map(|choice| {
            let path = path.clone();
            rsx! {
                label {
                    input {
                        r#type: "radio",
                        // Every radio in the group carries the SAME name: that
                        // is what makes them one group to the browser, and what
                        // `apply_form_values` collects the pick under.
                        name: "{path}",
                        value: "{choice.value}",
                        checked: choice.value == current,
                        // On a radio, `required` applies to the whole group, so
                        // repeating it per input asks for one pick, not one per
                        // button.
                        required,
                        aria_invalid: invalid,
                        onchange: move |e: FormEvent| write_value(&path, values, e.value()),
                    }
                    "{choice.display}"
                }
            }
        })
        .collect();

    rsx! {
        fieldset { class: "form-field radio-group",
            // The group's label is the `legend`, not a `label` — a `<label>`
            // can only name a single control, and there are several here.
            legend {
                if let Some(text) = label_text {
                    "{text}"
                }
                if required {
                    span { class: "required", " *" }
                }
            }
            { radios.into_iter() }
            FieldErrors { errors }
        }
    }
}
