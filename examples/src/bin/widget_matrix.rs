//! Which `(value kind, widget)` pairs actually render, and which panic.
//!
//! Whether `form!` accepts a pair is the widget's `ACCEPTS`, checked at
//! compile time. Nothing checks it at runtime since the widget types replaced
//! `ScalarWidget`, which asserted it. This tool builds its forms with
//! `FormSpec` directly, bypassing `form!`, so it shows both: what the runtime
//! actually does with every pair, and where that differs from `form!`.
//!
//! This prints it. Run with:
//!
//! ```text
//! cargo run -p formoxus-examples --bin widget_matrix
//! ```
//!
//! It is a development tool rather than a teaching example: it exists to keep
//! the README's "what isn't done" section honest. A widget `form!` reserves a
//! name for but nothing renders yet (`WidgetName::NOT_YET` in `formoxus-spec`)
//! has no type, so it has no row here until it lands.

// `dx serve` builds this whole package for wasm, binaries included, and there
// is no `dioxus-ssr` there — nor any point in a terminal table inside a
// browser. Gating by target rather than by a cargo feature keeps
// `cargo test --workspace` compiling it in CI with nobody passing a flag.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::FieldType;
use formoxus::using_fns;
use formoxus::widgets::{
    AnyWidget, CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput,
    MonthInput, NumberInput, PasswordInput, RadioGroup, SearchInput, Select, TelInput, TextInput,
    Textarea, TimeInput, UrlInput, WeekInput, Widget,
};
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

const FIELDS: [(&str, FieldType); 4] = [
    ("text", FieldType::Text),
    ("int", FieldType::Int),
    ("float", FieldType::Float),
    ("flag", FieldType::Bool),
];

#[component]
fn OneField(path: String, widget: AnyWidget) -> Element {
    let form =
        use_form(move || empty_form(FormSpec::<Sampler>::new().with_widget(&path, widget.clone())));
    form.render(using_fns! {})
}

/// One row of the table: a widget's `form!` name, the widget, and whether
/// `form!` accepts it on a field of each type (no choices, not optional).
type Entry = (&'static str, AnyWidget, fn(FieldType) -> bool);

fn entry<W: Widget + Default>() -> Entry {
    (W::NAME, W::default().into(), |t| W::ACCEPTS.contains(t))
}

/// Every built-in widget type, in `WidgetName::ALL`'s order.
fn every_widget() -> [Entry; 18] {
    [
        entry::<TextInput>(),
        entry::<PasswordInput>(),
        entry::<HiddenInput>(),
        entry::<NumberInput>(),
        entry::<EmailInput>(),
        entry::<TelInput>(),
        entry::<UrlInput>(),
        entry::<SearchInput>(),
        entry::<ColorInput>(),
        entry::<DateInput>(),
        entry::<TimeInput>(),
        entry::<DatetimeLocalInput>(),
        entry::<MonthInput>(),
        entry::<WeekInput>(),
        entry::<CheckboxInput>(),
        entry::<Textarea>(),
        entry::<Select>(),
        entry::<RadioGroup>(),
    ]
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
fn renders(path: &str, widget: AnyWidget) -> bool {
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
    // input deliberately renders bare with no label at all, so it looked like a
    // failure. (A checkbox used to as well, putting its text in a plain `<label>`
    // rather than the `fx-field-label` span — fixed 2026-09-29.) Every widget that renders
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
        .map(|(n, _, _)| n.len())
        .max()
        .unwrap_or(0)
        .max(7);

    println!("\nformoxus widget matrix — which (value kind, widget) pairs render\n");
    print!("{:width$}", "widget", width = width);
    for (p, _) in FIELDS {
        print!("  {p:>7}");
    }
    println!();
    println!("{}", "─".repeat(width + FIELDS.len() * 9));

    // What `form!` would say about each pair, beside what the runtime does.
    let (mut ok, mut unchecked, mut bad) = (0, 0, 0);
    for (name, widget, accepts) in &widgets {
        print!("{name:width$}");
        for (path, field_type) in FIELDS {
            let cell = match (renders(path, widget.clone()), accepts(field_type)) {
                (true, true) => {
                    ok += 1;
                    "ok"
                }
                (true, false) => {
                    unchecked += 1;
                    "ok*"
                }
                (false, _) => {
                    bad += 1;
                    "PANIC"
                }
            };
            print!("  {cell:>7}");
        }
        println!();
    }

    panic::set_hook(previous);
    let total = ok + unchecked + bad;
    println!(
        "\n{ok}/{total} pairs render and `form!` accepts them; {bad} panic.\n\
         ok* = {unchecked} more render, but `form!` rejects them at compile time,\n\
         so only a hand-built `FormSpec` reaches one.\n"
    );
}
