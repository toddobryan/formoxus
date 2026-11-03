//! Lists: values in an order the user decides, duplicates allowed, a `Vec`.
//! Each item is edited where it is, Add appends a blank one, and ↑/↓ move
//! it.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

// ── Typed in ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Recipe {
    name: String,
    steps: Vec<String>,
}

fn recipe_spec() -> FormSpec<Recipe> {
    form! {
        Recipe {
            title: "A recipe",
            steps[] => { widget: textarea },
            buttons: { save: { type: submit } }
        }
    }
}

/// Steps the user writes, in the order they happen: Add a step, then move it
/// into place with ↑/↓. The model's `Vec` is in the order on screen.
#[component]
pub fn FreeSteps() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| {
        form_for(
            &Recipe {
                name: "Toast".to_string(),
                steps: ["Slice the bread.", "Toast it.", "Butter it."]
                    .map(String::from)
                    .into(),
            },
            recipe_spec(),
        )
    });
    rsx! {
        {form.render(using_fns! {
            save: move |model: Recipe| async move {
                submitted.set(Some(format!("{model:?}")));
            },
        })}
        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}

// ── Chosen from a list ───────────────────────────────────────────────────

/// Airport codes are what the model stores; the user sees the city.
const CITIES: &[(&str, &str)] = &[
    ("BOS", "Boston"),
    ("ORD", "Chicago"),
    ("DEN", "Denver"),
    ("JFK", "New York"),
    ("SEA", "Seattle"),
];

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    traveler: String,
    stops: Vec<String>,
}

fn trip_spec() -> FormSpec<Trip> {
    form! {
        Trip {
            title: "A trip",
            // Every stop is one of the cities; formoxus refuses anything else
            // on submit, however it arrives.
            stops[] => { widget: select, choices: CITIES },
            buttons: { save: { type: submit } }
        }
    }
}

/// Stops picked from a list, in travel order. Unlike a set, a list can hold
/// the same value twice: a round trip starts and ends in the same city.
#[component]
pub fn ChosenStops() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| {
        form_for(
            &Trip {
                traveler: "Ada".to_string(),
                stops: ["BOS", "ORD", "BOS"].map(String::from).into(),
            },
            trip_spec(),
        )
    });
    rsx! {
        {form.render(using_fns! {
            save: move |model: Trip| async move {
                submitted.set(Some(format!("{model:?}")));
            },
        })}
        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}
