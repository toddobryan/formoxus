//! The widget boundary for the reflection path.
//!
//! One component per leaf, and that is the load-bearing part. A component is
//! the unit of reactivity in Dioxus: a store read inside one subscribes *that*
//! scope. `FormMember::render` is a plain function with no scope of its own, so
//! reading a value there would subscribe whoever called it — and a single
//! keystroke would re-render the entire form. Spawning a component per leaf is
//! what keeps a write to one path local to one input.
//!
//! This mirrors the derive path, where `TextInput::render` doesn't inline its
//! markup either — it spawns `InputWidget`, for exactly this reason.

use dioxus::prelude::*;

use crate::error::FieldError;
use crate::reflect::ValuesByPath;
use crate::widgets::FieldErrors;

/// A single-line text input bound to one path in the value map.
///
/// `values` + `path` rather than a pre-lensed child store, because a path that
/// the schema has but the map doesn't is a normal state, not an error: a
/// variant chosen after mount reveals leaves that were never populated. A missing
/// key reads as `""`, which is the same "empty IS absence" rule `apply_leaves`
/// already follows when a path is absent from submitted values.
#[component]
pub fn ScalarInput(
    path: String,
    label: Option<String>,
    errors: Vec<FieldError>,
    values: ValuesByPath,
) -> Element {
    // `get_unchecked`, not `get`: `get` calls `contains_key`, which tracks the
    // map *shallowly* — this input would then re-render whenever any key is
    // added anywhere. `get_unchecked` builds the child selector without
    // reading, so the only subscription is the `try_read` below, on this key
    // alone. It never panics here because we never `read()` it directly.
    let slot = values.get_unchecked(path.clone());
    let current = slot.try_read().map(|v| v.clone()).unwrap_or_default();

    let label_text = label;
    let write_path = path.clone();
    let mut values = values;

    rsx! {
        label { class: "form-field",
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
            }
            input {
                r#type: "text",
                name: "{path}",
                value: "{current}",
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    // Write *through the child store* when the key exists: that
                    // marks only this key dirty. `insert` would call
                    // `mark_dirty_shallow` and re-render every other input, so
                    // it's the fallback for a never-populated path only — one
                    // coarse re-render on the first keystroke into a freshly
                    // revealed field, fine-grained from then on.
                    // `peek`, not `contains_key`: the store's own
                    // `contains_key` tracks shallowly, and an event handler has
                    // no business adding subscriptions. Bound to a `let` so the
                    // read guard is definitely released before the write below.
                    let populated = values.peek().contains_key(&write_path);
                    if populated {
                        values.get_unchecked(write_path.clone()).set(raw);
                    } else {
                        values.insert(write_path.clone(), raw);
                    }
                },
            }
            FieldErrors { errors }
        }
    }
}
