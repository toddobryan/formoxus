//! `Vec`/`Def::List` in edit mode.

use super::across_the_wire;
use super::models::{Location, Shape};
use super::{Harness, new_since};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::nodes::Edit;
use formoxus::nodes::ValuesByPath;
use formoxus::prelude::*;
use googletest::prelude::*;
use indexmap::IndexMap;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    title: String,
    answers: Vec<String>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venues {
    places: Vec<Location>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Grid {
    rows: Vec<Vec<String>>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Drawings {
    shapes: Vec<Shape>,
}

fn quiz() -> Quiz {
    Quiz {
        title: "Unit 1".to_string(),
        answers: vec!["alpha".to_string(), "beta".to_string()],
    }
}

fn venues() -> Venues {
    Venues {
        places: vec![
            Location {
                street: "123 Main St".to_string(),
                city: "Springfield".to_string(),
                zip: "12345".to_string(),
            },
            Location {
                street: "9 Elm".to_string(),
                city: "Shelbyville".to_string(),
                zip: "99999".to_string(),
            },
        ],
    }
}

#[gtest]
fn scalar_rows_round_trip() {
    let mut form = form_for(&quiz(), FormSpec::default());
    expect_that!(form.validate(), some(eq(&quiz())));
    expect_that!(across_the_wire(&quiz()), ok(eq(&quiz())));
}

#[gtest]
fn an_empty_list_round_trips() {
    // `init_list` with no `begin_list_item` at all — the degenerate case
    // that would quietly pass even if populating were broken, which is why it
    // can't be the only list test.
    let empty = Quiz {
        title: "Unit 1".to_string(),
        answers: Vec::new(),
    };
    let mut form = form_for(&empty, FormSpec::default());
    expect_that!(form.validate(), some(eq(&empty)));
    expect_that!(across_the_wire(&empty), ok(eq(&empty)));
}

#[gtest]
fn struct_rows_round_trip() {
    // The case that proves the `write_value_into` split: a row is a
    // `FieldSet`, so this nests `begin_list_item` → `begin_field` per struct
    // field. Confusing the two halves fails exactly here.
    let mut form = form_for(&venues(), FormSpec::default());
    expect_that!(form.validate(), some(eq(&venues())));
    expect_that!(across_the_wire(&venues()), ok(eq(&venues())));
}

#[gtest]
fn rows_are_named_by_key() {
    let form = form_for(&quiz(), FormSpec::default());
    expect_that!(
        // Into a `Vec`, not compared as a map: `IndexMap`'s `==` ignores order,
        // and the order is part of what this asserts.
        form.collect_values().into_iter().collect::<Vec<_>>(),
        eq(&vec![
            ("title".to_string(), "Unit 1".to_string()),
            // The list's own entry: its row keys, in row order.
            ("answers".to_string(), "#0,#1".to_string()),
            ("answers.#0".to_string(), "alpha".to_string()),
            ("answers.#1".to_string(), "beta".to_string()),
        ])
    );
}

#[gtest]
fn struct_rows_qualify_through_their_key() {
    let form = form_for(&venues(), FormSpec::default());
    let paths: Vec<String> = form.collect_values().into_iter().map(|(p, _)| p).collect();
    expect_that!(
        paths,
        elements_are![
            eq("places"),
            eq("places.#0.street"),
            eq("places.#0.city"),
            eq("places.#0.zip"),
            eq("places.#1.street"),
            eq("places.#1.city"),
            eq("places.#1.zip"),
        ]
    );
}

#[gtest]
fn nested_lists_nest_their_keys() {
    let grid = Grid {
        rows: vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string()],
        ],
    };
    let form = form_for(&grid, FormSpec::default());
    expect_that!(
        // Every list says which rows it has, the inner ones included.
        form.collect_values().into_iter().collect::<Vec<_>>(),
        eq(&vec![
            ("rows".to_string(), "#0,#1".to_string()),
            ("rows.#0".to_string(), "#0,#1".to_string()),
            ("rows.#0.#0".to_string(), "a".to_string()),
            ("rows.#0.#1".to_string(), "b".to_string()),
            ("rows.#1".to_string(), "#0".to_string()),
            ("rows.#1.#0".to_string(), "c".to_string()),
        ])
    );

    let mut form = form;
    expect_that!(form.validate(), some(eq(&grid)));
    expect_that!(across_the_wire(&grid), ok(eq(&grid)));
}

#[gtest]
fn enum_rows_are_pinned_by_the_value() {
    // Populating pins each row's variant independently — row 0 and row 1 are
    // different variants of the same enum, and neither needed a choice from
    // the caller because the value itself answered.
    let drawings = Drawings {
        shapes: vec![
            Shape::Circle { radius: 1.5 },
            Shape::Rectangle {
                width: 2.0,
                height: 3.0,
            },
        ],
    };
    let form = form_for(&drawings, FormSpec::default());
    let paths: Vec<String> = form.collect_values().into_iter().map(|(p, _)| p).collect();
    expect_that!(
        paths,
        elements_are![
            eq("shapes"),
            eq("shapes.#0"),
            eq("shapes.#0.$Circle.radius"),
            eq("shapes.#1"),
            eq("shapes.#1.$Rectangle.width"),
            eq("shapes.#1.$Rectangle.height"),
        ]
    );

    let mut form = form;
    expect_that!(form.validate(), some(eq(&drawings)));
    expect_that!(across_the_wire(&drawings), ok(eq(&drawings)));
}

#[gtest]
fn editing_one_row_leaves_the_others_alone() {
    let mut form = form_for(&venues(), FormSpec::default());
    form.distribute_values(&IndexMap::from([(
        "places.#1.city".to_string(),
        "Ogdenville".to_string(),
    )]));

    let mut expected = venues();
    expected.places[1].city = "Ogdenville".to_string();
    expect_that!(form.validate(), some(eq(&expected)));
    expect_that!(across_the_wire(&expected), ok(eq(&expected)));
}

#[gtest]
fn leaves_then_apply_is_an_identity_round_trip() {
    // The widget loop, for lists, reloaded into an `empty_form` the way the
    // scalar version of this test does. An empty form has no rows, so this
    // only works because the list's own entry (`places` = `#0,#1`) rebuilds
    // them: before #13 it reloaded into a form of the same shape instead.
    let form = form_for(&venues(), FormSpec::default());
    let collected: ValuesByPath = form.collect_values();

    let mut reloaded = empty_form::<Venues>(FormSpec::default());
    reloaded.distribute_values(&collected);
    expect_that!(reloaded.validate(), some(eq(&venues())));
    expect_that!(across_the_wire(&venues()), ok(eq(&venues())));
}

#[gtest]
fn create_mode_yields_no_rows_yet() {
    // Characterization, not an endorsement: `list_node` has no length to
    // work from without a value, so it builds an empty `ListSet` and
    // `validate` produces `vec![]` with no complaint. This test exists to
    // make that silence visible, and SHOULD start failing at step 4.
    let mut form = empty_form::<Quiz>(FormSpec::default());
    form.distribute_values(&IndexMap::from([(
        "title".to_string(),
        "Unit 1".to_string(),
    )]));
    expect_that!(
        form.validate(),
        some(eq(&Quiz {
            title: "Unit 1".to_string(),
            answers: Vec::new(),
        }))
    );
}

// ── Adding and removing rows ─────────────────────────────────────────────
//
// A row's name is a KEY, not a position. Everything below is a consequence of
// that one decision, and every test here would pass with index-named rows
// EXCEPT the two that check the neighbours' paths after an insert or a remove —
// which is exactly where index names silently move data between rows.

fn paths_of<T: Clone + std::fmt::Debug + PartialEq + Facet<'static>>(
    form: &FormState<T>,
) -> Vec<String> {
    form.collect_values().into_iter().map(|(p, _)| p).collect()
}

#[gtest]
fn adding_a_row_appends_by_default() {
    let mut form = form_for(&quiz(), FormSpec::default());
    form.edit(&Edit::AddRow {
        path: "answers".to_string(),
        before: None,
    })
    .expect("answers is a list");

    expect_that!(
        paths_of(&form),
        elements_are![
            eq("title"),
            eq("answers"),
            eq("answers.#0"),
            eq("answers.#1"),
            eq("answers.#2")
        ]
    );
}

#[gtest]
fn a_new_row_is_blank_and_buildable() {
    // The row is built from the ELEMENT shape with no value to peek at, so it
    // has to come out as a working node rather than a placeholder: fill it and
    // the model builds.
    let mut form = form_for(&quiz(), FormSpec::default());
    form.edit(&Edit::AddRow {
        path: "answers".to_string(),
        before: None,
    })
    .expect("answers is a list");
    form.distribute_form_values(&[("answers.#2".to_string(), "gamma".to_string())]);

    expect_that!(
        form.validate(),
        some(eq(&Quiz {
            title: "Unit 1".to_string(),
            answers: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
        }))
    );
}

#[gtest]
fn inserting_at_the_front_does_not_move_the_rows_below_it() {
    // THE test for keys. With index-named rows, inserting at 0 would renumber
    // every row after it — which means every leaf beneath them lands on a
    // different key in the value store, and the store (which deliberately
    // survives structural edits) would hand row 1 the text the user typed into
    // row 0. Keys make the insert a pure addition: nobody else is touched.
    let mut form = form_for(&quiz(), FormSpec::default());
    form.edit(&Edit::AddRow {
        path: "answers".to_string(),
        before: Some(0),
    })
    .expect("answers is a list");

    expect_that!(
        // Into a `Vec`, not compared as a map: `IndexMap`'s `==` ignores order,
        // and the order is part of what this asserts.
        form.collect_values().into_iter().collect::<Vec<_>>(),
        eq(&vec![
            ("title".to_string(), "Unit 1".to_string()),
            // The list's own entry carries the ORDER, which the keys alone
            // cannot: #2 is the newest key but the first row.
            ("answers".to_string(), "#2,#0,#1".to_string()),
            // The new row comes FIRST in order...
            ("answers.#2".to_string(), String::new()),
            // ...while the two that were already here keep both their keys and
            // their values. Order and identity are different things now.
            ("answers.#0".to_string(), "alpha".to_string()),
            ("answers.#1".to_string(), "beta".to_string()),
        ])
    );
}

#[gtest]
fn the_list_builds_in_row_order_not_key_order() {
    // The other half of the split: `write_value_into` walks `rows` and never
    // reads a row's name, so a row inserted at the front lands at the front of
    // the model even though its key is the highest.
    let mut form = form_for(&quiz(), FormSpec::default());
    form.edit(&Edit::AddRow {
        path: "answers".to_string(),
        before: Some(0),
    })
    .expect("answers is a list");
    form.distribute_form_values(&[("answers.#2".to_string(), "aardvark".to_string())]);

    expect_that!(
        form.validate(),
        some(eq(&Quiz {
            title: "Unit 1".to_string(),
            answers: vec![
                "aardvark".to_string(),
                "alpha".to_string(),
                "beta".to_string()
            ],
        }))
    );
}

#[gtest]
fn removing_a_row_leaves_the_survivors_keys_untouched() {
    // The mirror of the insert test. Index names would shift row 2 down into
    // row 1's paths; keys mean the survivor is still `#1` and still owns the
    // values already sitting under `places.#1.*` in the store.
    let mut form = form_for(&venues(), FormSpec::default());
    form.edit(&Edit::RemoveRow {
        path: "places".to_string(),
        index: 0,
    })
    .expect("places is a list");

    expect_that!(
        paths_of(&form),
        elements_are![
            eq("places"),
            eq("places.#1.street"),
            eq("places.#1.city"),
            eq("places.#1.zip")
        ]
    );
    expect_that!(
        form.validate(),
        some(eq(&Venues {
            places: vec![venues().places[1].clone()]
        }))
    );
}

#[gtest]
fn a_key_is_never_reused() {
    // Remove the last row and add one: the new row must NOT inherit the dead
    // row's key, or the value store's leftovers from the removed row would
    // silently populate the new one.
    let mut form = form_for(&quiz(), FormSpec::default());
    form.edit(&Edit::RemoveRow {
        path: "answers".to_string(),
        index: 1,
    })
    .expect("places is a list");
    form.edit(&Edit::AddRow {
        path: "answers".to_string(),
        before: None,
    })
    .expect("answers is a list");

    expect_that!(
        paths_of(&form),
        elements_are![
            eq("title"),
            eq("answers"),
            eq("answers.#0"),
            eq("answers.#2")
        ],
        "the removed row was #1, so #1 must not come back"
    );
    expect_that!(form.collect_values().get("answers"), some(eq("#0,#2")));
}

#[gtest]
fn a_row_edit_reaches_a_row_that_was_added_after_construction() {
    // A row built by `AddRow` has to be indistinguishable from one built at
    // construction — including being reachable by the containment walk, which
    // means its prefix has to match what `list_node` would have produced.
    let mut form = form_for(&Drawings { shapes: Vec::new() }, FormSpec::default());
    form.edit(&Edit::AddRow {
        path: "shapes".to_string(),
        before: None,
    })
    .expect("shapes is a list");
    form.choose_variant("shapes.#0", Some("Circle"))
        .expect("the new row is an enum, and Circle is one of its variants");

    expect_that!(
        paths_of(&form),
        elements_are![
            eq("shapes"),
            eq("shapes.#0"),
            eq("shapes.#0.$Circle.radius")
        ]
    );
}

#[gtest]
fn an_out_of_range_edit_is_an_error_rather_than_a_panic() {
    // `Vec::insert`/`Vec::remove` both panic out of range, and on wasm a panic
    // aborts instead of reaching an `ErrorBoundary` — so these are checked.
    let mut form = form_for(&quiz(), FormSpec::default());
    expect_that!(
        form.edit(&Edit::AddRow {
            path: "answers".to_string(),
            before: Some(9)
        }),
        err(anything())
    );
    expect_that!(
        form.edit(&Edit::RemoveRow {
            path: "answers".to_string(),
            index: 9
        }),
        err(anything())
    );
    expect_that!(
        paths_of(&form),
        elements_are![
            eq("title"),
            eq("answers"),
            eq("answers.#0"),
            eq("answers.#1")
        ],
        "a rejected edit must leave the list exactly as it was"
    );
}

#[gtest]
fn an_add_row_aimed_at_a_list_is_no_longer_reported_as_no_such_path() {
    // Regression: before `ListSet::edit` grew an "is it me?" branch, an edit
    // addressed to the list itself fell through the row loop and came back as
    // `no such path: answers` — a lie, since the list is precisely what owns it.
    let mut form = form_for(&quiz(), FormSpec::default());
    expect_that!(
        form.edit(&Edit::AddRow {
            path: "answers".to_string(),
            before: None
        }),
        ok(anything())
    );
}

#[component]
fn QuizForm() -> Element {
    let form = use_form(|| form_for(&quiz(), FormSpec::default()));
    form.render(using_fns! {})
}

#[gtest]
fn each_row_renders_inside_its_own_wrapper() {
    // The wrapper exists so a row has an element to carry dioxus's `key:` — and
    // it is where a per-row remove widget will go. The key itself is NOT
    // visible here: dioxus keys aren't DOM attributes, they only show up in
    // diffing behaviour, which needs a widget that can dispatch `AddRow`
    // before it can be driven. This guards the structure the key rides on.
    let html = super::render_to_html(QuizForm);
    expect_that!(html.matches(r#"<div class="fx-form-row">"#).count(), eq(2));
    expect_that!(html, contains_substring(r#"name="answers.#0""#));
    expect_that!(html, contains_substring(r#"name="answers.#1""#));
}

// ── The row widgets ─────────────────────────────────────────────────────
//
// These build the list up THROUGH THE UI rather than mounting a populated one,
// because that is the only way to know which button is which: listener
// registration order is the order dioxus creates dynamic nodes, not document
// order, so "the first click listener" is not "the Add button." Built this way,
// every widget is identified by the click that produced it.
//
// None of these prove the row `key:` earns its keep — appending diffs correctly
// by position too, so removing the key fails nothing here. The key's payoff is
// mid-list insertion, which has no widget yet; the data half of that claim is
// `inserting_at_the_front_does_not_move_the_rows_below_it`, and the DOM half
// arrives with the insert-between-rows widget.

#[component]
fn BlankQuizForm() -> Element {
    let form = use_form(|| empty_form::<Quiz>(FormSpec::default()));
    form.render(using_fns! {})
}

#[gtest]
fn clicking_add_on_an_empty_list_creates_a_row() {
    // This also retires an open question rather than answering it: a blank form
    // yields zero rows because a row count isn't in the shape, and that used to
    // read as a gap. With an Add button it is simply the right answer.
    let mut app = Harness::mount(BlankQuizForm);
    let add = app.only_listener("click");
    expect_that!(
        app.html(),
        not(contains_substring("answers.#")),
        "no rows yet"
    );

    app.click(add);

    expect_that!(app.html(), contains_substring(r#"name="answers.#0""#));
}

#[gtest]
fn removing_a_row_through_the_dom_leaves_its_neighbour_untouched() {
    // The end-to-end version of `removing_a_row_leaves_the_survivors_keys_
    // untouched`: same claim, but every step goes through a real event, so it
    // covers the buttons, the `Edit`s they build, `use_form`'s callback, and the
    // value store surviving the rebuild.
    let mut app = Harness::mount(BlankQuizForm);
    let add = app.only_listener("click");

    // Row one. The widget it reveals is the one that wasn't there before.
    let clicks = app.listeners("click");
    let inputs = app.listeners("input");
    app.click(add);
    let remove_first = new_since(&clicks, app.listeners("click"))[0];
    app.fire(
        "input",
        new_since(&inputs, app.listeners("input"))[0],
        "alpha",
    );

    // Row two.
    let clicks = app.listeners("click");
    let inputs = app.listeners("input");
    app.click(add);
    app.fire(
        "input",
        new_since(&inputs, app.listeners("input"))[0],
        "beta",
    );

    expect_that!(
        app.listeners("click"),
        contains(eq(&remove_first)),
        "appending must not tear down the row that was already there"
    );
    expect_that!(
        new_since(&clicks, app.listeners("click")).len(),
        eq(1),
        "one new Remove"
    );

    app.click(remove_first);

    let html = app.html();
    expect_that!(html, not(contains_substring("alpha")), "row one is gone");
    expect_that!(
        html,
        contains_substring(r#"name="answers.#1" value="beta""#),
        "row two keeps its key AND the text typed into it"
    );
}

/// What `to_wire` would send for `answers` right now, and what a server makes
/// of the whole payload, so a `Harness` can read both after clicks.
#[component]
fn BlankQuizProbe() -> Element {
    let form = use_form(|| empty_form::<Quiz>(FormSpec::default()));
    let wire = form.to_wire();
    let rows = wire.values().get("answers").cloned().unwrap_or_default();
    let accepted = match Submission::accept(FormSpec::<Quiz>::default(), wire.values()) {
        Ok(sub) => sub.into_model().answers.join("|"),
        Err(_) => "rejected".to_string(),
    };
    rsx! {
        {form.render(using_fns! {})}
        output { id: "rows", "{rows}" }
        output { id: "accepted", "{accepted}" }
    }
}

/// The browser half of #13 for lists: rows added, typed into and removed
/// through real clicks reach the wire, and a server rebuilds exactly them.
/// The rows exist only in the state, never in the store, so this is the path
/// that would send none if `to_wire` read structure from the store.
#[gtest]
fn rows_added_and_removed_in_the_browser_reach_the_server() {
    let mut app = Harness::mount(BlankQuizProbe);
    // `title` is required, so the server rejects until it has one.
    let title = app.only_listener("input");
    app.fire("input", title, "Unit 1");
    let add = app.only_listener("click");

    let clicks = app.listeners("click");
    let inputs = app.listeners("input");
    app.click(add);
    let remove_first = new_since(&clicks, app.listeners("click"))[0];
    app.fire(
        "input",
        new_since(&inputs, app.listeners("input"))[0],
        "alpha",
    );

    let inputs = app.listeners("input");
    app.click(add);
    app.fire(
        "input",
        new_since(&inputs, app.listeners("input"))[0],
        "beta",
    );
    expect_that!(
        app.html(),
        all![
            contains_substring(r#"<output id="rows">#0,#1</output>"#),
            contains_substring(r#"<output id="accepted">alpha|beta</output>"#)
        ]
    );

    app.click(remove_first);

    expect_that!(
        app.html(),
        all![
            contains_substring(r#"<output id="rows">#1</output>"#),
            contains_substring(r#"<output id="accepted">beta</output>"#)
        ],
        "the removed row is gone from the wire, and its survivor keeps its key"
    );
}

// ── The list's own errors ────────────────────────────────────────────────
//
// A list's own error (a server's verdict at the list's path, a rejected
// row-key entry, and one day a row count, #14) renders under its legend,
// linked by `aria-describedby`. A bad CELL is its row's error, not the list's.

fn errors_at(path: &str, message: &str) -> FormErrors {
    FormErrors {
        form: Vec::new(),
        fields: [(
            path.to_string(),
            vec![ValidationMessage(message.to_string())],
        )]
        .into_iter()
        .collect(),
    }
}

#[component]
fn QuizWithAListError() -> Element {
    let form = use_form(|| form_for(&quiz(), FormSpec::default()));
    form.absorb(WireForm::from_errors(errors_at(
        "answers",
        "Give at least three answers.",
    )))
    .expect("answers is a path of this form");
    form.render(using_fns! {})
}

#[gtest]
fn a_lists_own_error_renders_under_its_legend_and_describes_it() {
    let html = super::without_form_id(&super::render_to_html(QuizWithAListError));
    expect_that!(
        html,
        contains_substring(
            r#"<fieldset class="fx-group fx-list-set fx-invalid" id="FORMID-answers" aria-describedby="FORMID-answers-errors">"#
        )
    );
    expect_that!(
        html,
        contains_substring(r#"<ul class="fx-field-errors" id="FORMID-answers-errors">"#)
    );
    // In reading order: the list's name, then its message, then its rows.
    let legend = html.find("Answers</legend>").expect("the legend renders");
    let message = html
        .find("Give at least three answers.")
        .expect("the message renders");
    let first_row = html.find(r#"name="answers.#0""#).expect("the rows render");
    expect_that!(legend < message && message < first_row, eq(true));
}

#[component]
fn QuizWithACellError() -> Element {
    let form = use_form(|| form_for(&quiz(), FormSpec::default()));
    form.absorb(WireForm::from_errors(errors_at("answers.#0", "Too short.")))
        .expect("answers.#0 is a path of this form");
    form.render(using_fns! {})
}

#[gtest]
fn a_bad_cell_does_not_mark_the_list_invalid() {
    let html = super::without_form_id(&super::render_to_html(QuizWithACellError));
    expect_that!(html, contains_substring("Too short."));
    expect_that!(
        html,
        contains_substring(r#"<fieldset class="fx-group fx-list-set" id="FORMID-answers">"#),
        "no fx-invalid and no aria-describedby: the error is the cell's"
    );
    expect_that!(html, not(contains_substring("FORMID-answers-errors")));
}

#[gtest]
fn a_list_renders_its_own_label() {
    // Until the list got a `fieldset`, no container but `FieldSet` rendered its
    // label, so `answers` appeared on the page as an unexplained stack of inputs.
    let html = super::render_to_html(QuizForm);
    expect_that!(
        html,
        contains_substring(r#"<legend class="fx-legend fx-field-label">Answers</legend>"#)
    );
}

#[gtest]
fn the_row_widgets_never_submit_the_form() {
    // Inside a `<form>` a bare `<button>` is `type="submit"`. Both widgets
    // would then submit the form instead of editing the list — and since the
    // demo page's Check button is a real submit, that failure would look like
    // "adding a row validates the form."
    let html = super::render_to_html(QuizForm);
    expect_that!(
        html.matches(r#"<button type="button""#).count(),
        eq(3),
        "two Remove, one Add"
    );
}
