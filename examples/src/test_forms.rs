//! Small single-purpose forms, one feature each, at `/t/<slug>`.
//!
//! **Not part of the gallery, and deliberately not linked from it.** The two
//! serve different audiences: the gallery is for someone deciding whether to use
//! formoxus, so its forms are realistic and show several features at once; these
//! are for `just e2e`, so each one is as small as it can be. A form with one
//! field and one button needs no scoped selectors, no `nth()`, and no test ids.
//!
//! There is precedent for test-only artifacts living in this crate:
//! `bin/widget_matrix.rs` calls itself "a development tool rather than a teaching
//! example".
//!
//! **Nothing human keeps these from rotting**, because nobody browses them. The
//! e2e tests are the only guard, so a form without a test is dead weight — add
//! them in pairs.
//!
//! One dynamic route rather than one variant per form: adding a form is then a
//! function plus a match arm, and nothing links to these so the type-safe `Link`
//! that variants would buy has no user. An unknown slug renders a visible
//! complaint rather than a blank page, so a typo in a test URL fails loudly.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;
use std::fmt::Debug;

/// Dispatch on the slug. Every form is listed here and nowhere else.
#[component]
pub fn TestForm(slug: String) -> Element {
    match slug.as_str() {
        "required-text" => rsx! { RequiredText {} },
        "required-text-novalidate" => rsx! { RequiredTextNoValidate {} },
        "pattern" => rsx! { Pattern {} },
        "pattern-novalidate" => rsx! { PatternNoValidate {} },
        "lengths" => rsx! { Lengths {} },
        "bounds" => rsx! { Bounds {} },
        "checkbox" => rsx! { CheckboxForm {} },
        "select" => rsx! { SelectForm {} },
        "radio-group" => rsx! { RadioGroupForm {} },
        "textarea" => rsx! { TextareaForm {} },
        "buttons" => rsx! { Buttons {} },
        other => rsx! {
            main { class: "container",
                h1 { "No such test form" }
                p { "Nothing is registered for the slug {other:?}." }
                p { "Add it to `examples/src/test_forms.rs`." }
            }
        },
    }
}

/// The frame every test form sits in: a heading saying what it is for, the form,
/// and the submitted model.
///
/// The model is rendered with `{:#?}` rather than `{:?}` — one field per line,
/// which reads better for a human AND makes a Playwright `to_contain_text` match
/// a whole line unambiguously instead of straddling two fields. It needs the
/// `<pre>` to keep those newlines.
#[component]
fn Harness(title: String, note: String, submitted: String, children: Element) -> Element {
    rsx! {
        main { class: "container",
            h1 { "{title}" }
            p { class: "note", "{note}" }
            {children}
            section { class: "submitted",
                "Validated model:"
                br {}
                // `#submitted` is the one selector every test shares; `Session`
                // reads it. Present even when empty, so a test can assert
                // NOTHING was submitted without waiting on a missing element.
                pre { id: "submitted", "{submitted}" }
            }
        }
    }
}

/// Renders the frame around a form, wiring the submit handler to record the
/// validated model. Factored out because every test form wants exactly this.
fn harness<T>(title: &str, note: &str, spec: FormSpec<T>) -> Element
where
    T: Clone + Debug + PartialEq + Facet<'static> + 'static,
{
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(move || empty_form(spec.clone()));
    rsx! {
        Harness { title: title.to_string(), note: note.to_string(), submitted: submitted().unwrap_or_default(),
            {
                form.render(using_fns! {
                    save: move |model: T| async move {
                        submitted.set(Some(format!("{model:#?}")));
                    },
                })
            }
        }
    }
}

// ── required-text ────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct OneField {
    name: String,
}

/// Browser validation ON: submitting empty must be blocked by the browser, so
/// `onsubmit` never fires and nothing is ever recorded.
#[component]
fn RequiredText() -> Element {
    harness(
        "required-text",
        "One required text field, browser validation ON. Submitting it empty should be blocked by the browser.",
        form! {
            OneField {
                browser_validation: on,
                buttons: { save: { type: submit } }
            }
        },
    )
}

/// The same form with validation OFF: the browser lets it through, formoxus
/// validates, and its own "This field is required." renders. Between them these
/// two are the proof that `novalidate` neutralizes what the other emits.
#[component]
fn RequiredTextNoValidate() -> Element {
    harness(
        "required-text-novalidate",
        "The same field with browser validation OFF. Submitting it empty should reach formoxus, which reports it.",
        form! {
            OneField {
                browser_validation: off,
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── pattern ──────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Zip {
    code: String,
}

/// A five-digit `pattern`, validation ON. The browser should refuse a bad value
/// before `onsubmit` runs.
#[component]
fn Pattern() -> Element {
    harness(
        "pattern",
        "A five-digit pattern, browser validation ON.",
        form! {
            Zip {
                browser_validation: on,
                code => { pattern: r"\d{5}" },
                buttons: { save: { type: submit } }
            }
        },
    )
}

/// The same pattern with validation OFF — formoxus does the rejecting, and its
/// message is reachable. Also the one place the anchoring matters end to end: a
/// value with five digits EMBEDDED in it must still be rejected.
#[component]
fn PatternNoValidate() -> Element {
    harness(
        "pattern-novalidate",
        "The same pattern with browser validation OFF, so formoxus reports it.",
        form! {
            Zip {
                browser_validation: off,
                code => { pattern: r"\d{5}" },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── lengths ──────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Nick {
    nick: String,
}

/// `minlength` and `maxlength`, validation ON.
#[component]
fn Lengths() -> Element {
    harness(
        "lengths",
        "min_length 3, max_length 6, browser validation ON.",
        form! {
            Nick {
                browser_validation: on,
                nick => { min_length: 3, max_length: 6 },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── bounds ───────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Age {
    age: i64,
}

/// `min`/`max` on a number. **`widget: number` is deliberate**: `Int` defaults to
/// a TEXT input, and a browser ignores `min`/`max` there — so without the
/// override this form would prove nothing about browser enforcement. It is also
/// the case the `step` decision is about, though an integer is unaffected by the
/// implicit `step=1`.
#[component]
fn Bounds() -> Element {
    harness(
        "bounds",
        "min 18, max 120 on a number input, browser validation ON.",
        form! {
            Age {
                browser_validation: on,
                age => { min: 18, max: 120, widget: number },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── checkbox ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Agree {
    agreed: bool,
}

/// A bool as a checkbox. **Unticked is a complete answer** — formoxus has no way
/// to demand a tick, which is issue #6. So submitting it untouched succeeds with
/// `agreed: false`, and that is the behaviour under test rather than a gap in it.
#[component]
fn CheckboxForm() -> Element {
    harness(
        "checkbox",
        "A bool as a checkbox. Unticked is a complete answer — see issue #6.",
        form! {
            Agree {
                agreed => { widget: checkbox },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── select and radio-group ───────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Pick {
    state: String,
}

const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska"), ("AZ", "Arizona")];

/// A value choice as a `<select>`. The stored value is the code, the reader sees
/// the name — so a round trip proves they stayed separate.
#[component]
fn SelectForm() -> Element {
    harness(
        "select",
        "A required value choice as a select. The value is the code, the display is the name.",
        form! {
            Pick {
                state => { widget: select { choices: STATES } },
                buttons: { save: { type: submit } }
            }
        },
    )
}

/// The same choices as radios. Nothing is pre-selected, deliberately — a library
/// that checked the first one would make a required radio group unable to fail
/// its own required check.
#[component]
fn RadioGroupForm() -> Element {
    harness(
        "radio-group",
        "The same choices as radios. Nothing is pre-selected.",
        form! {
            Pick {
                state => { widget: radio_group { choices: STATES } },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── textarea ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Bio {
    about: String,
}

/// A `<textarea>`, which is also where a multi-line value round-trips — the
/// newline semantics `fields.rs` tests in Rust, seen from the browser.
#[component]
fn TextareaForm() -> Element {
    harness(
        "textarea",
        "A textarea, including a value with a newline in it.",
        form! {
            Bio {
                about => { widget: textarea },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── buttons ──────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Person {
    name: String,
}

/// Reset and submit, over a SEEDED form — reset has nothing to restore otherwise.
/// This is the only test form built with `form_for` rather than `empty_form`.
#[component]
fn Buttons() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let seeded = Person { name: "Ada".into() };
    let form = use_form(move || {
        form_for(
            &seeded,
            form! {
                Person {
                    buttons: {
                        undo: { type: reset },
                        save: { type: submit },
                    }
                }
            },
        )
    });
    rsx! {
        Harness {
            title: "buttons".to_string(),
            note: "A form seeded with \"Ada\": reset restores it, submit records it.".to_string(),
            submitted: submitted().unwrap_or_default(),
            {
                form.render(using_fns! {
                    undo: move || async move { form.reset() },
                    save: move |model: Person| async move {
                        submitted.set(Some(format!("{model:#?}")));
                    },
                })
            }
        }
    }
}
