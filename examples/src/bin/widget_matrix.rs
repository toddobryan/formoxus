//! Which `(value kind, widget)` pairs actually render, and which panic.
//!
//! `form!` accepts every widget name in its table against every field. Whether
//! the pair then *renders* is decided at runtime, by a match in `ScalarWidget`
//! whose fallthrough arm panics. So the set of combinations that really work is
//! not written down anywhere — it is implied by that match, and a reader of the
//! macro's table cannot see it.
//!
//! This prints it. Run with:
//!
//! ```text
//! cargo run -p formoxus-examples --bin widget_matrix
//! ```
//!
//! It is a development tool rather than a teaching example: it exists to keep
//! the README's "what isn't done" section honest, and to tell us when work on
//! the unimplemented widgets actually lands.

// `dx serve` builds this whole package for wasm, binaries included, and there
// is no `dioxus-ssr` there — nor any point in a terminal table inside a
// browser. Gating by target rather than by a cargo feature keeps
// `cargo test --workspace` compiling it in CI with nobody passing a flag.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

use dioxus::prelude::*;
use facet::Facet;
use formoxus::widgets::{InputType, WidgetType};
use formoxus::{FormSpec, empty_form, use_form};
use std::panic;

/// One field per scalar value kind a model can currently reach. The widget
/// under test is applied to exactly one of these paths at a time; the others
/// keep their defaults, so anything that panics is attributable to the pair
/// being tested rather than to its neighbours.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Sampler {
    text: String,
    int: i64,
    float: f64,
    flag: bool,
}

const FIELDS: [&str; 4] = ["text", "int", "float", "flag"];

#[component]
fn OneField(path: String, widget: WidgetType) -> Element {
    let form = use_form(move || {
        empty_form(FormSpec::<Sampler>::new().with_custom_widget(&path, widget.clone()))
    });
    form.render_fragment()
}

/// Every widget a consumer can name, in the macro table's own order.
fn every_widget() -> Vec<(&'static str, WidgetType)> {
    use InputType::*;
    let inputs = [
        ("text", Text),
        ("password", Password),
        ("hidden", Hidden),
        ("number", Number),
        ("email", Email),
        ("tel", Telephone),
        ("url", Url),
        ("search", Search),
        ("color", Color),
        ("date", Date),
        ("time", Time),
        ("datetime_local", DatetimeLocal),
        ("month", Month),
        ("week", Week),
    ];
    let mut all: Vec<(&'static str, WidgetType)> = inputs
        .into_iter()
        .map(|(n, t)| (n, WidgetType::Input(t)))
        .collect();
    all.extend([
        ("textarea", WidgetType::Textarea),
        ("select", WidgetType::Select),
        ("select_multiple", WidgetType::SelectMultiple),
        ("checkbox", WidgetType::Checkbox),
        ("checkbox_multiple", WidgetType::CheckboxMultiple),
        ("radio_group", WidgetType::RadioGroup),
        ("file", WidgetType::File),
    ]);
    all
}

/// `true` if the pair actually produced markup.
///
/// **Not `catch_unwind`, which was the obvious first attempt and is wrong.**
/// Dioxus catches a panic inside the component's own scope and renders that
/// subtree as nothing, so the unwind never reaches a caller and every pair
/// looks like it passed. What actually happens is that the field silently
/// vanishes while its siblings render normally — which is the real failure mode
/// a user would hit, and is why this looks for the field's own label in the
/// output instead.
#[cfg(not(target_arch = "wasm32"))]
fn renders(path: &str, widget: WidgetType) -> bool {
    let mut dom = VirtualDom::new_with_props(
        OneField,
        OneFieldProps {
            path: path.to_string(),
            widget,
        },
    );
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    // `name="<path>"` and not the label, which was the first attempt: a `hidden`
    // input deliberately renders bare with no label at all, and a checkbox puts
    // its text in a plain `<label>` rather than the `field-label` span every
    // other widget uses. Both looked like failures. Every widget that renders
    // at all emits a `name`, because that is what makes the value submittable.
    html.contains(&format!(r#"name="{path}""#))
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Each unimplemented pair panics on purpose, and the default hook would
    // print a backtrace over the table for every one of them.
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let widgets = every_widget();
    let width = widgets
        .iter()
        .map(|(n, _)| n.len())
        .max()
        .unwrap_or(0)
        .max(7);

    println!("\nformoxus widget matrix — which (value kind, widget) pairs render\n");
    print!("{:width$}", "widget", width = width);
    for p in FIELDS {
        print!("  {:>7}", p);
    }
    println!();
    println!("{}", "─".repeat(width + FIELDS.len() * 9));

    let (mut ok, mut bad) = (0, 0);
    for (name, widget) in &widgets {
        print!("{:width$}", name, width = width);
        for path in FIELDS {
            if renders(path, widget.clone()) {
                ok += 1;
                print!("  {:>7}", "ok");
            } else {
                bad += 1;
                print!("  {:>7}", "PANIC");
            }
        }
        println!();
    }

    panic::set_hook(previous);
    let total = ok + bad;
    println!(
        "\n{ok}/{total} pairs render; {bad} panic.\n\
         Every one of them is spellable in `form!` today.\n"
    );
}
