//! Sets: values with no order of their own and no duplicates, a `BTreeSet`
//! (or a `Vec` marked `set`). Each item is shown, not edited in place: it is
//! added, edited and replaced through the draft below the items.

use std::collections::BTreeSet;

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

// ── Typed in ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    title: String,
    tags: BTreeSet<String>,
}

fn post_spec() -> FormSpec<Post> {
    form! {
        Post {
            title: "A blog post",
            // Case-insensitive, so `Rust` sorts beside `rust` rather than
            // before every lowercase tag. Two such tags are still two: the set
            // compares by the model's own `==`.
            tags => {
                sort_by: unicode_case_insensitive,
                help_text: "Press Enter to add one. Edit puts a tag back in the box.",
            },
            // The gallery's chips preset (`main.css`), one class on the set.
            classes_plus: { set: + ["fx-tags"] },
            buttons: { save: { type: submit } }
        }
    }
}

/// Tags the user makes up: type one and press Add or Enter. A tag already
/// there is refused with a message; Edit puts one back into the box, and
/// Replace swaps it.
#[component]
pub fn FreeTags() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| {
        form_for(
            &Post {
                title: "Hello, formoxus".to_string(),
                tags: ["dioxus", "forms", "rust"].map(String::from).into(),
            },
            post_spec(),
        )
    });
    rsx! {
        {form.render(using_fns! {
            save: move |model: Post| async move {
                submitted.set(Some(format!("{model:?}")));
            },
        })}
        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}

// ── Chosen from a list ───────────────────────────────────────────────────

/// A unit enum is a closed list of legal values, so the draft is a select
/// over them.
#[derive(Facet, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Meeting {
    name: String,
    days: BTreeSet<Weekday>,
}

fn meeting_spec() -> FormSpec<Meeting> {
    form! {
        Meeting {
            title: "A weekly meeting",
            // Week order with no `sort_by`: a set of an enum without data
            // keeps its variants' declaration order.
            days => { label: "Meets on" },
            buttons: { save: { type: submit } }
        }
    }
}

/// Days picked from the week: choose one in the select and press Add. Each
/// day can be picked once, and they are kept in week order whatever order
/// they were added in.
#[component]
pub fn ChosenDays() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| empty_form(meeting_spec()));
    rsx! {
        {form.render(using_fns! {
            save: move |model: Meeting| async move {
                submitted.set(Some(format!("{model:?}")));
            },
        })}
        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}

// ── Ticked or selected: a set widget ─────────────────────────────────────

fn ticked_spec() -> FormSpec<Meeting> {
    form! {
        Meeting {
            title: "A weekly meeting, ticked",
            // One box per day, the enum's variants being the choices. At
            // least one and at most three: at three the rest lock.
            days => {
                label: "Meets on",
                widget: checkbox_multiple,
                min_items: 1,
                max_items: 3,
            },
            buttons: { save: { type: submit } }
        }
    }
}

/// The same days as a row of checkboxes: one per day, ticked for each day in
/// the set. A set widget renders the whole set, so there is no Add.
#[component]
pub fn TickedDays() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| empty_form(ticked_spec()));
    rsx! {
        {form.render(using_fns! {
            save: move |model: Meeting| async move {
                submitted.set(Some(format!("{model:?}")));
            },
        })}
        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tour {
    cities: BTreeSet<String>,
}

const CITIES: &[(&str, &str)] = &[
    ("ams", "Amsterdam"),
    ("bcn", "Barcelona"),
    ("lis", "Lisbon"),
    ("osl", "Oslo"),
    ("rom", "Rome"),
];

fn tour_spec() -> FormSpec<Tour> {
    form! {
        Tour {
            title: "A tour",
            // The choices go beside the widget that shows them. What is
            // stored is the code; what is shown is the name.
            cities => {
                widget: select_multiple,
                choices: CITIES,
                help_text: "Ctrl- or shift-click to pick several.",
            },
            buttons: { save: { type: submit } }
        }
    }
}

/// Cities picked in a multi-select: the set holds the codes of every
/// selected option.
#[component]
pub fn PickedCities() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| {
        form_for(
            &Tour {
                cities: ["lis", "rom"].map(String::from).into(),
            },
            tour_spec(),
        )
    });
    rsx! {
        {form.render(using_fns! {
            save: move |model: Tour| async move {
                submitted.set(Some(format!("{model:?}")));
            },
        })}
        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}
