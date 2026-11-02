//! `HashSet`/`BTreeSet`, `Def::Set`: a set's items, its markup, and the edits
//! it refuses (`SETS_PLAN.md` 2g).
//!
//! Until the set sorts (box 4), nothing here depends on a `HashSet`'s
//! iteration order: its tests compare whole sets, whose `PartialEq` ignores
//! order. A set's items are still rendered as their leaves' own widgets, so
//! editable, until Edit (box 8) replaces that; the markup tests check the
//! structure around them, not what an item renders as.

use super::{across_the_wire, store_keys, with_store_keys};
use dioxus::prelude::*;
use facet::Facet;
use formoxus::nodes::{Edit, ItemValue};
use formoxus::prelude::*;
use formoxus::widgets::{AddItemProps, ItemAdder};
use googletest::prelude::*;
use std::collections::{BTreeSet, HashSet};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    title: String,
    tags: BTreeSet<String>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Bag {
    tags: HashSet<String>,
}

fn tags<S: FromIterator<String>>(names: &[&str]) -> S {
    names.iter().map(ToString::to_string).collect()
}

fn post() -> Post {
    Post {
        title: "Hello".to_string(),
        tags: tags(&["dioxus", "rust"]),
    }
}

// ── Round trips ──────────────────────────────────────────────────────────

#[gtest]
fn a_btree_set_round_trips() {
    expect_that!(
        form_for(&post(), FormSpec::default()).validate(),
        some(eq(&post()))
    );
    expect_that!(across_the_wire(&post()), ok(eq(&post())));
}

#[gtest]
fn an_empty_set_round_trips() {
    let empty = Post {
        title: "Hello".to_string(),
        tags: BTreeSet::new(),
    };
    expect_that!(
        form_for(&empty, FormSpec::default()).validate(),
        some(eq(&empty))
    );
    expect_that!(across_the_wire(&empty), ok(eq(&empty)));
}

/// Three items, so a `HashSet` that came back in a different order, or keyed
/// differently on the way, still has to come back as the same set.
#[gtest]
fn a_hash_set_round_trips() {
    let bag = Bag {
        tags: tags(&["alpha", "beta", "gamma"]),
    };
    expect_that!(
        form_for(&bag, FormSpec::default()).validate(),
        some(eq(&bag))
    );
    expect_that!(across_the_wire(&bag), ok(eq(&bag)));
}

// ── Markup ───────────────────────────────────────────────────────────────

#[component]
fn PostForm() -> Element {
    let form = use_form(|| form_for(&post(), FormSpec::default()));
    form.render(using_fns! {})
}

#[component]
fn UntaggedPostForm() -> Element {
    let untagged = Post {
        title: "Hello".to_string(),
        tags: BTreeSet::new(),
    };
    let form = use_form(|| form_for(&untagged, FormSpec::default()));
    form.render(using_fns! {})
}

#[gtest]
fn a_set_is_a_fieldset_with_its_legend() {
    let html = super::without_form_id(&super::render_to_html(PostForm));
    expect_that!(
        html,
        contains_substring(r#"<fieldset class="fx-group fx-set" id="FORMID-tags">"#)
    );
    expect_that!(
        html,
        contains_substring(r#"<legend class="fx-legend fx-field-label">Tags</legend>"#)
    );
}

/// A `<ul>`, not a list's `<ol>`: a set's order is canonical, not the user's,
/// so there is no position worth announcing and nothing to move.
#[gtest]
fn a_sets_items_are_a_ul_each_with_remove_and_no_moves() {
    let html = super::render_to_html(PostForm);
    expect_that!(html.matches(r#"<ul class="fx-set-items">"#).count(), eq(1));
    expect_that!(html.matches(r#"<li class="fx-set-item">"#).count(), eq(2));
    expect_that!(html.matches("fx-button-remove-row").count(), eq(2));
    expect_that!(html, not(contains_substring("<ol")));
    expect_that!(html, not(contains_substring("fx-move-buttons")));
}

/// As with a list: a `<ul>` may hold only `<li>`s, so the adder comes after it.
#[gtest]
fn a_sets_adder_follows_its_ul() {
    let html = super::render_to_html(PostForm);
    let ul_ends = html.find("</ul>").expect("the items are a <ul>");
    let add = html
        .find("fx-button-add-row")
        .expect("a set without `adder:` gets Add");
    expect_that!(add, gt(ul_ends));
}

/// No empty `<ul>`, for the reason a list has no empty `<ol>`: some screen
/// readers announce it as "list, 0 items". The adder stays, since it is how
/// the first item arrives.
#[gtest]
fn an_empty_set_renders_no_ul_but_still_its_adder() {
    let html = super::render_to_html(UntaggedPostForm);
    expect_that!(html, not(contains_substring("<ul")));
    expect_that!(html, contains_substring("fx-button-add-row"));
}

// ── Edits ────────────────────────────────────────────────────────────────

fn add_tag(value: Option<ItemValue>) -> Edit {
    Edit::AddItem {
        path: "tags".to_string(),
        before: None,
        value,
    }
}

#[gtest]
fn a_set_refuses_a_move() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&Edit::MoveItem {
            path: "tags".to_string(),
            from: 0,
            to: 1,
        })
        .map_err(|e| e.0),
        err(contains_substring(
            "tags is a set; you cannot reorder its items"
        ))
    );
    expect_that!(form.validate(), some(eq(&post())), "the set is unchanged");
}

/// A blank item is what a list's Add makes, to be typed into. A set's items
/// are display-only (E), so one arriving blank would be an empty value the
/// user can never fill in.
#[gtest]
fn a_set_refuses_a_blank_item() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&add_tag(None)).map_err(|e| e.0),
        err(contains_substring(
            "tags is a set; you cannot add an empty item to a set"
        ))
    );
    expect_that!(form.validate(), some(eq(&post())), "the set is unchanged");
}

#[gtest]
fn a_set_takes_an_item_that_arrives_filled() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new("web".to_string())))),
        ok(some(eq("tags.#2")))
    );
    expect_that!(
        form.validate(),
        some(eq(&Post {
            title: "Hello".to_string(),
            tags: tags(&["dioxus", "rust", "web"]),
        }))
    );
}

#[gtest]
fn a_set_refuses_an_item_of_the_wrong_type() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new(7_u8)))),
        err(displays_as(contains_substring(
            "cannot add a `u8` item to tags: its items are `String`"
        )))
    );
    expect_that!(form.validate(), some(eq(&post())), "the set is unchanged");
}

// ── A removed item leaves the store ──────────────────────────────────────

#[component]
fn RemovedSetItemProbe() -> Element {
    let form = use_form(|| form_for(&post(), FormSpec::default()));
    use_hook(|| {
        form.edit(&Edit::RemoveItem {
            path: "tags".to_string(),
            index: 0,
        })
        .expect("item 0 exists")
    });
    with_store_keys(form)
}

/// The cleanup `use_form` does for a list's row reaches a set's item too: it
/// finds the item through the key entry `Items` writes for both.
#[gtest]
fn a_removed_items_value_leaves_the_store() {
    let html = super::render_to_html(RemovedSetItemProbe);
    expect_that!(store_keys(&html), eq("tags.#1 title"));
}

// ── Behind an `Option`, and inside an enum ───────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct MaybeTagged {
    tags: Option<BTreeSet<String>>,
}

#[gtest]
fn a_set_behind_an_option_round_trips() {
    for model in [
        MaybeTagged {
            tags: Some(tags(&["dioxus", "rust"])),
        },
        MaybeTagged { tags: None },
    ] {
        expect_that!(
            form_for(&model, FormSpec::default()).validate(),
            some(eq(&model))
        );
        expect_that!(across_the_wire(&model), ok(eq(&model)));
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(C)]
#[allow(dead_code)]
enum Labels {
    Untagged,
    Tagged { tags: BTreeSet<String> },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Note {
    labels: Labels,
}

#[gtest]
fn a_set_inside_an_enum_variant_round_trips() {
    let note = Note {
        labels: Labels::Tagged {
            tags: tags(&["dioxus", "rust"]),
        },
    };
    expect_that!(
        form_for(&note, FormSpec::default()).validate(),
        some(eq(&note))
    );
    expect_that!(across_the_wire(&note), ok(eq(&note)));
}

// ── A custom adder ───────────────────────────────────────────────────────

#[component]
fn TagAdder(props: AddItemProps) -> Element {
    rsx! { span { id: "tag-adder", "adds to {props.path}, which has {props.rows}" } }
}

#[component]
fn CustomAdderPostForm() -> Element {
    let form = use_form(|| {
        form_for(
            &post(),
            FormSpec::default().with_adder("tags", ItemAdder(TagAdder)),
        )
    });
    form.render(using_fns! {})
}

#[component]
fn FormAdderPostForm() -> Element {
    let form = use_form(|| form_for(&post(), form! { Post { tags => { adder: TagAdder } } }));
    form.render(using_fns! {})
}

/// `with_adder` takes a set's path as it takes a list's, and so does `form!`'s
/// `adder:`, whose `const` check accepts a set; either way the set renders the
/// dev's component in place of Add.
#[gtest]
fn a_set_takes_a_custom_adder() {
    for html in [
        super::render_to_html(CustomAdderPostForm),
        super::render_to_html(FormAdderPostForm),
    ] {
        expect_that!(
            html,
            contains_substring(r#"<span id="tag-adder">adds to tags, which has 2</span>"#)
        );
        expect_that!(html, not(contains_substring("fx-button-add-row")));
    }
}

// ── Order ────────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Tag(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shelf {
    tags: BTreeSet<Tag>,
}

/// Only a set of scalars has a default order (`SETS_PLAN.md` 4a); a newtype
/// needs `sort_by`. Empty on purpose: the order is chosen from the item
/// SHAPE when the set is built, so a missing one fails at the first render,
/// not when a user's first Add gives the set something to sort.
#[gtest]
#[should_panic(expected = "you must provide a sort_by function")]
fn a_set_of_non_scalars_without_sort_by_fails_at_construction() {
    let _ = empty_form::<Shelf>(FormSpec::default());
}
