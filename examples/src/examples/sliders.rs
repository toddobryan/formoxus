//! `widget: range`: an integer slider, a float one, and one without its number.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Feedback {
    satisfaction: u8,
    hours_a_week: f64,
    loudness: u8,
}

/// Each slider starts where the browser draws an empty one, so an untouched
/// form submits what it shows: `satisfaction` at 3, halfway from 1 to 5, and
/// `hours_a_week` at 10. A float range must say its `step:`, since a range
/// steps by 1 unless told; `omit_value` drops the number beside `loudness`.
fn spec() -> FormSpec<Feedback> {
    form! {
        Feedback {
            satisfaction => { label: "How satisfied are you?", widget: range, min: 1, max: 5 },
            hours_a_week => {
                label: "Hours a week you use it",
                widget: range, min: 0.0, max: 20.0, step: 0.5,
            },
            loudness => {
                label: "Too quiet ← → Too loud",
                widget: range, min: 0, max: 10, omit_value,
            },
            buttons: {
                send: { type: submit, text: "Send feedback" },
            }
        }
    }
}

#[component]
pub fn Sliders() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| empty_form(spec()));

    rsx! {
        {form.render(using_fns! {
            send: move |model: Feedback| async move {
                submitted.set(Some(format!("{model:?}")));
            },
        })}

        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}
