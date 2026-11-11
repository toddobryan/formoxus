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
use formoxus::{nodes::ItemValue, prelude::*};
use std::collections::BTreeSet;
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
        "range" => rsx! { RangeForm {} },
        "checkbox" => rsx! { CheckboxForm {} },
        "must-agree" => rsx! { MustAgree {} },
        "must-agree-novalidate" => rsx! { MustAgreeNoValidate {} },
        "select" => rsx! { SelectForm {} },
        "radio-group" => rsx! { RadioGroupForm {} },
        "textarea" => rsx! { TextareaForm {} },
        "textarea-seeded" => rsx! { TextareaSeeded {} },
        "enum" => rsx! { EnumForm {} },
        "enum-submit" => rsx! { EnumSubmit {} },
        "buttons" => rsx! { Buttons {} },
        "bare" => rsx! { Bare {} },
        "reorder" => rsx! { Reorder {} },
        "struct-items" => rsx! { StructItems {} },
        "assignment" => rsx! { AssignmentForm {} },
        "adder" => rsx! { Adder {} },
        "tags" => rsx! { Tags {} },
        "tags-pattern" => rsx! { TagsPattern {} },
        "live-text" => rsx! { LiveText {} },
        "live-click" => rsx! { LiveClick {} },
        "live-server-error" => rsx! { LiveServerError {} },
        "live-select" => rsx! { LiveSelect {} },
        "live-off" => rsx! { LiveOff {} },
        "live-custom" => rsx! { LiveCustom {} },
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

// ── range ────────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Ranges {
    plain: i32,
    by_three: i32,
    by_four: i32,
    tie: i32,
}

/// Four sliders, each on an empty form, chosen for where an EMPTY range starts
/// (`WIDGETS_PLAN.md` decision A): 0–10 at the midpoint, 5; with `step: 3` at 6,
/// the nearer step; with `step: 4` at 4; and 1–4 at 3, a tie that takes the
/// larger.
///
/// **The bare inputs after the form are the reference.** formoxus renders the
/// value it computed, so its own sliders agree with the form whatever the
/// formula says. Each `#ref-…` is a plain `<input type="range">` with the same
/// bounds and NO value, so what the browser draws there is HTML's own default,
/// which is what the e2e test holds formoxus to.
#[component]
fn RangeForm() -> Element {
    rsx! {
        {
            harness(
                "range",
                "Four empty sliders, then bare reference sliders with the same bounds and no value.",
                form! {
                    Ranges {
                        plain => { label: "Plain", widget: range, min: 0, max: 10 },
                        by_three => { label: "By three", widget: range, min: 0, max: 10, step: 3 },
                        by_four => { label: "By four", widget: range, min: 0, max: 10, step: 4 },
                        tie => { label: "Tie", widget: range, min: 1, max: 4 },
                        buttons: { save: { type: submit } }
                    }
                },
            )
        }
        section { id: "reference",
            input { r#type: "range", id: "ref-plain", min: "0", max: "10" }
            input { r#type: "range", id: "ref-by_three", min: "0", max: "10", step: "3" }
            input { r#type: "range", id: "ref-by_four", min: "0", max: "10", step: "4" }
            input { r#type: "range", id: "ref-tie", min: "1", max: "4" }
        }
    }
}

// ── checkbox ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Agree {
    agreed: bool,
}

/// A bool as a checkbox. **Unticked is a complete answer**, so submitting it
/// untouched succeeds with `agreed: false`, and that is the behaviour under test.
/// Demanding a tick takes `required_true`; see `must-agree` below.
#[component]
fn CheckboxForm() -> Element {
    harness(
        "checkbox",
        "A bool as a checkbox. Unticked is a complete answer.",
        form! {
            Agree {
                agreed => { widget: checkbox },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── must-agree ───────────────────────────────────────────────────────────

/// `required_true` on a bool, validation ON: the box must be ticked, and the
/// browser blocks an unticked submit itself, from the `required` attribute.
#[component]
fn MustAgree() -> Element {
    harness(
        "must-agree",
        "A checkbox that must be ticked, browser validation ON. Submitting it unticked should be blocked by the browser.",
        form! {
            Agree {
                browser_validation: on,
                agreed => { required_true },
                buttons: { save: { type: submit } }
            }
        },
    )
}

/// The same rule with validation OFF, so formoxus does the rejecting. This is
/// the half that a forged request also meets, since the browser's `required` is
/// only a convenience.
#[component]
fn MustAgreeNoValidate() -> Element {
    harness(
        "must-agree-novalidate",
        "The same checkbox with browser validation OFF. Submitting it unticked should reach formoxus, which reports it.",
        form! {
            Agree {
                browser_validation: off,
                agreed => { required_true },
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
                state => { choices: STATES },
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
                state => { widget: radio_group, choices: STATES },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── an enum field ────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Contact {
    Email,
    Phone,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Reach {
    contact: Contact,
}

/// An enum field: a `<select>` over its variants, inside a fieldset whose
/// legend carries the field's label. The select has no label of its own, so
/// this is where its accessible name is checked.
#[component]
fn EnumForm() -> Element {
    harness(
        "enum",
        "A required enum field. Its variant select sits under the fieldset's legend.",
        form! {
            Reach {
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── enum-submit ──────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Footprint {
    Circle { radius: f64 },
    Square { side: f64 },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Plot {
    footprint: Footprint,
}

/// A data-carrying enum chosen and filled IN THE BROWSER, then submitted:
/// issue #13's path. The variant lives only in the form's state, never in the
/// value store, so this is where a real `onchange` has to reach the wire.
///
/// Records three things: the model the client validated (`#submitted`, as
/// every test form does), what `Submission::accept` builds from `to_wire()`
/// (`#accepted`, run in the page, since no test form has a server fn), and how
/// many submits there have been (`#submits`), so validating twice is visible.
#[component]
fn EnumSubmit() -> Element {
    let spec = || form! { Plot { buttons: { save: { type: submit } } } };
    let mut submitted = use_signal(|| None::<String>);
    let mut accepted = use_signal(String::new);
    let mut submits = use_signal(|| 0_u32);
    let form = use_form(move || empty_form(spec()));
    rsx! {
        Harness {
            title: "enum-submit".to_string(),
            note: "A data-carrying enum: choose a variant, fill its field, submit. What the client validated and what a server would build from the wire should agree.".to_string(),
            submitted: submitted().unwrap_or_default(),
            {
                form.render(using_fns! {
                    save: move |model: Plot| async move {
                        *submits.write() += 1;
                        submitted.set(Some(format!("{model:#?}")));
                        let wire = form.to_wire();
                        accepted.set(match Submission::accept(spec(), wire.values()) {
                            Ok(sub) => format!("{:#?}", sub.into_model()),
                            Err(errors) => format!("rejected: {errors:?}"),
                        });
                    },
                })
            }
            section { class: "accepted",
                "Accepted by Submission::accept:"
                br {}
                pre { id: "accepted", "{accepted}" }
            }
            p { "Submits: " output { id: "submits", "{submits}" } }
        }
    }
}

// ── textarea ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Bio {
    about: String,
}

/// A `<textarea>`, which is also where a multi-line value round-trips — the
/// newline semantics `fields/string_field.rs` tests in Rust, seen from the browser.
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

/// A SEEDED textarea: the server renders it, the browser hydrates it, and it must
/// show its text, which a textarea takes from its content, not a `value`
/// attribute. Overwrite writes a new value to the store after the user has
/// typed, which content alone cannot show (a typed-in textarea's content moves
/// only its default value): what `value` is still for. Not reset, which the
/// browser also performs natively, from the content, so it would pass anyway.
#[component]
fn TextareaSeeded() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let seeded = Bio {
        about: "one\ntwo & <three>".into(),
    };
    let form = use_form(move || {
        form_for(
            &seeded,
            form! {
                Bio {
                    about => { widget: textarea },
                    buttons: {
                        overwrite: { type: action },
                        save: { type: submit },
                    }
                }
            },
        )
    });
    rsx! {
        Harness {
            title: "textarea-seeded".to_string(),
            note: "A textarea seeded with two lines: it shows them, and Overwrite replaces whatever is typed.".to_string(),
            submitted: submitted().unwrap_or_default(),
            {
                form.render(using_fns! {
                    overwrite: move || async move {
                        form.write(&path!(Bio.about), "overwritten")
                            .expect("about is a field of this form");
                    },
                    save: move |model: Bio| async move {
                        submitted.set(Some(format!("{model:#?}")));
                    },
                })
            }
        }
    }
}

// ── buttons ──────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Person {
    name: String,
}

/// Reset and submit, over a SEEDED form — reset has nothing to restore otherwise.
/// Built with `form_for` rather than `empty_form`, as are `textarea-seeded`
/// and `assignment`.
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

// ── bare ─────────────────────────────────────────────────────────────────

/// `Classes::bare()`: every class slot empty, so formoxus emits no `class`
/// attribute at all. Validation OFF, so an empty submit reaches formoxus and
/// the re-render that shows its error is one the browser performs, which the
/// SSR suite's `bare()` test cannot reach.
#[component]
fn Bare() -> Element {
    harness(
        "bare",
        "One required field under Classes::bare(), browser validation OFF. No element of the form should carry a class, before or after a submit.",
        form! {
            OneField {
                classes: Classes::bare(),
                browser_validation: off,
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── reorder ──────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Playlist {
    songs: Vec<String>,
}

/// A list: items are added with Add, moved with their ↑/↓ buttons, and the
/// submitted model shows the order they ended in. Blank, so a test builds
/// the items it moves.
#[component]
fn Reorder() -> Element {
    harness(
        "reorder",
        "A list. Add items, move them with their arrow buttons, submit: the model keeps the new order.",
        form! {
            Playlist {
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── struct-items ──────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Member {
    name: String,
    email: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Team {
    members: Vec<Member>,
}

/// A list whose items are structs. An item Add makes takes focus
/// at its FIRST field, which a leaf item (`/t/reorder`) cannot show.
#[component]
fn StructItems() -> Element {
    harness(
        "struct-items",
        "A list of two-field items. Add an item: focus lands in its Name field.",
        form! {
            Team {
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── assignment ───────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    groups: Vec<QuestionGroup>,
}

/// apcsp's assignment shape (`ORDERED_LISTS_PLAN.md` box 7), with built-in
/// widgets: a list of enum groups, each holding a list of question ids.
#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum QuestionGroup {
    All { questions: Vec<String> },
    Random { how_many: u8, set: Vec<String> },
}

/// A list inside an enum inside a list, seeded, so a test can move and add
/// at both levels and see each click land on its own list.
#[component]
fn AssignmentForm() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let seeded = Assignment {
        groups: vec![
            QuestionGroup::All {
                questions: vec!["q1".into(), "q2".into(), "q3".into()],
            },
            QuestionGroup::Random {
                how_many: 2,
                set: vec!["q4".into(), "q5".into()],
            },
        ],
    };
    let form = use_form(move || {
        form_for(
            &seeded,
            form! {
                Assignment {
                    buttons: { save: { type: submit } }
                }
            },
        )
    });
    rsx! {
        Harness {
            title: "assignment".to_string(),
            note: "Groups of questions: an All group (q1–q3) and a Random group (q4, q5). Each level has its own ↑/↓ and Add.".to_string(),
            submitted: submitted().unwrap_or_default(),
            {
                form.render(using_fns! {
                    save: move |model: Assignment| async move {
                        submitted.set(Some(format!("{model:#?}")));
                    },
                })
            }
        }
    }
}

// ── adder ────────────────────────────────────────────────────────────────

/// What `/t/adder` offers, one button each.
const SONGS: [&str; 3] = ["Alpha", "Bravo", "Charlie"];

/// A custom adder (`adder:`): one button per song, each adding an item that
/// arrives already holding it, in place of the blank item Add would make.
#[component]
fn SongPicker(props: formoxus::widgets::AddItemProps) -> Element {
    rsx! {
        for song in SONGS {
            button { r#type: "button", onclick: pick(props.on_edit, props.path.clone(), song), "Add {song}" }
        }
    }
}

/// The click handler for one song's button, out here because an rsx `for`
/// body cannot hold the `let` its clone would need.
fn pick(
    on_edit: Callback<formoxus::widgets::Edit>,
    path: formoxus::path::NodePath,
    song: &'static str,
) -> impl FnMut(MouseEvent) {
    move |_| {
        on_edit.call(formoxus::widgets::Edit::AddItem {
            path: path.clone(),
            before: None,
            value: Some(ItemValue::new(song.to_string())),
        });
    }
}

/// A list whose Add is replaced by `SongPicker`. Each click adds a filled item,
/// and the submitted model shows the songs in the order they were picked.
#[component]
fn Adder() -> Element {
    harness(
        "adder",
        "A list with a custom adder. Click a song's button: an item arrives holding it. Submit: the model lists them in the order picked.",
        form! {
            Playlist {
                songs => { adder: SongPicker },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── tags ─────────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tagged {
    tags: BTreeSet<String>,
}

/// A set (`SETS_PLAN.md` boxes 7 and 8): items are added through the draft,
/// "Add to Tags", by Add or Enter, and edited through it by Edit, then Save
/// or Cancel (or Escape).
#[component]
fn Tags() -> Element {
    harness(
        "tags",
        "A set of tags. Type one into Add to Tags and press Add or Enter; Edit puts one back into the box, Save replaces it, Cancel or Escape leaves it. Submit: the model lists them sorted.",
        form! {
            Tagged {
                buttons: { save: { type: submit } }
            }
        },
    )
}

/// The same set, its items constrained by a `pattern`, with browser
/// validation ON. Text left in the draft is not in the set, so it must not
/// block the submit (box 14); Add still checks it.
#[component]
fn TagsPattern() -> Element {
    harness(
        "tags-pattern",
        "A set of lowercase tags, pattern [a-z]+, browser validation ON. Leave \"Rust\" in Add to Tags and submit: it goes through. Press Add: it is refused.",
        form! {
            Tagged {
                browser_validation: on,
                tags[] => { pattern: "[a-z]+" },
                buttons: { save: { type: submit } }
            }
        },
    )
}

// ── live validation (`ERRORS_PLAN.md` 8d) ────────────────────────────────
//
// Each with browser validation OFF, so a submit reaches formoxus and what the
// tests see is formoxus's own error list, never a browser tooltip.

#[derive(Facet, Clone, Debug, PartialEq)]
struct LiveSignup {
    name: String,
    city: String,
}

/// Two fields, so there is somewhere to Tab to. Live validation is on, by
/// default.
#[component]
fn LiveText() -> Element {
    harness(
        "live-text",
        "Name needs at least 3 characters. Typing shows nothing; leaving the field shows its error; once shown, it goes on the keystroke that fixes it. Tabbing through without typing says nothing.",
        form! {
            LiveSignup {
                browser_validation: off,
                name => { min_length: 3 },
                buttons: { save: { type: submit } }
            }
        },
    )
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct LiveLast {
    name: String,
}

/// One field, directly above Submit, inside a wrapper that counts every
/// `submit` reaching it. Trap 1: clicking Submit with a bad value in the field
/// blurs it on mousedown, so its error appears and pushes Submit down before
/// the mouseup. `#submits` says whether the click still landed.
#[component]
fn LiveClick() -> Element {
    let mut submits = use_signal(|| 0_u32);
    let form = use_form(|| {
        empty_form(form! {
            LiveLast {
                browser_validation: off,
                name => { min_length: 3 },
                buttons: { save: { type: submit } }
            }
        })
    });
    rsx! {
        Harness {
            title: "live-click",
            note: "Type one letter into Name, then click Submit once. Its error appears as the field loses focus, above the button.",
            submitted: String::new(),
            div { onsubmit: move |_| submits += 1,
                {form.render(using_fns! { save: move |_: LiveLast| async move {} })}
            }
            p { "Submits: " span { id: "submits", "{submits}" } }
        }
    }
}

/// A server's verdict on a seeded value, as a rejected submit would leave it.
#[component]
fn LiveServerError() -> Element {
    let form = use_form(|| {
        form_for(
            &LiveSignup {
                name: "Ada".to_string(),
                city: String::new(),
            },
            form! { LiveSignup { browser_validation: off } },
        )
    });
    use_hook(|| {
        form.push_field_error(&path!(LiveSignup.name), "That name is taken.")
            .expect("name is a field of this form");
    });
    rsx! {
        Harness {
            title: "live-server-error",
            note: "Name carries a server's error. Clicking in and out of it, without typing, must leave the error alone: nothing local can say the name is free.",
            submitted: String::new(),
            {form.render(using_fns! {})}
        }
    }
}

/// A chooser has no typing, so its change is checked at once, with no blur.
#[component]
fn LiveSelect() -> Element {
    harness(
        "live-select",
        "We do not ship to Alaska. Choosing it says so at once.",
        form! {
            Pick {
                browser_validation: off,
                state => {
                    choices: STATES,
                    validator: |s| if s == "AK" { vec!["We do not ship to Alaska.".into()] } else { Vec::new() },
                },
                buttons: { save: { type: submit } }
            }
        },
    )
}

/// The same two fields with `live_validation: off`: nothing is checked until
/// a submit.
#[component]
fn LiveOff() -> Element {
    harness(
        "live-off",
        "Live validation OFF. Leaving a bad Name shows nothing; Submit shows its error.",
        form! {
            LiveSignup {
                browser_validation: off,
                live_validation: off,
                name => { min_length: 3 },
                buttons: { save: { type: submit } }
            }
        },
    )
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct LiveCustomModel {
    handled: String,
    input_only: String,
}

/// A dev's text widget through its `FieldHandle`, with both handlers a text
/// box wants.
#[derive(Clone, Debug, Default, PartialEq)]
struct HandledText;

impl formoxus::widgets::Widget for HandledText {
    const NAME: &'static str = "handled_text";
    const EXTENDS: formoxus::attrs::AttrSet = formoxus::attrs::AttrSet::EMPTY;
    const PARTS: &'static [formoxus::attrs::Tag] = &[];

    fn render(&self, props: formoxus::widgets::WidgetProps) -> Element {
        rsx! { DevText { field: props, with_blur: true } }
    }
}

/// The same widget missing `onblur`, the mistake the handle's docs warn of.
/// It is never checked on leaving, but it still validates on submit, and once
/// it shows an error each keystroke checks it.
#[derive(Clone, Debug, Default, PartialEq)]
struct InputOnlyText;

impl formoxus::widgets::Widget for InputOnlyText {
    const NAME: &'static str = "input_only_text";
    const EXTENDS: formoxus::attrs::AttrSet = formoxus::attrs::AttrSet::EMPTY;
    const PARTS: &'static [formoxus::attrs::Tag] = &[];

    fn render(&self, props: formoxus::widgets::WidgetProps) -> Element {
        rsx! { DevText { field: props, with_blur: false } }
    }
}

/// Both dev widgets' body, in a component of its own, as a widget reading the
/// store must be.
#[component]
fn DevText(field: formoxus::widgets::WidgetProps, with_blur: bool) -> Element {
    let handle = field.field_handle();
    let label = field.label.clone().unwrap_or_default();
    let invalid = field.validity() == formoxus::widgets::Validity::Invalid;
    rsx! {
        div {
            label { r#for: "{field.id}", "{label}" }
            if with_blur {
                input {
                    id: "{field.id}",
                    value: handle.current(),
                    aria_invalid: invalid.then_some("true"),
                    oninput: handle.on_input(),
                    onblur: handle.on_blur(),
                }
            } else {
                input {
                    id: "{field.id}",
                    value: handle.current(),
                    aria_invalid: invalid.then_some("true"),
                    oninput: handle.on_input(),
                }
            }
            formoxus::widgets::FieldErrors {
                field_id: field.id.clone(),
                paths: vec![field.path.clone()],
                errors: field.errors,
                classes: field.classes.clone(),
            }
        }
    }
}

#[component]
fn LiveCustom() -> Element {
    harness(
        "live-custom",
        "Two dev widgets, each needing 3 characters. Handled has onblur, so leaving it checks it. Input only has none: leaving it says nothing, but Submit does, and then each keystroke rechecks it.",
        form! {
            LiveCustomModel {
                browser_validation: off,
                handled => {
                    widget: HandledText,
                    validator: |v| if v.len() < 3 { vec!["Too short.".into()] } else { Vec::new() },
                },
                input_only => {
                    widget: InputOnlyText,
                    validator: |v| if v.len() < 3 { vec!["Too short.".into()] } else { Vec::new() },
                },
                buttons: { save: { type: submit } }
            }
        },
    )
}
