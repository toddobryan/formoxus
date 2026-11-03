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
            tags => { sort_by: unicode_case_insensitive },
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
            // Week order, `Weekday`'s own `Ord`: an enum has no default order
            // for a set to sort by, so `form!` asks for one.
            days => { label: "Meets on", sort_by: Weekday::cmp },
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
