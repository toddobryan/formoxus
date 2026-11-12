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
use formoxus::nodes::{Edit, ItemValue, SetFeedback, SetOrder, ValuesByPath};
use formoxus::path::NodePath;
use formoxus::prelude::*;
use formoxus::widgets::{AddItemProps, ItemAdder};
use googletest::prelude::*;
use std::cmp::Ordering;
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
    expect_that!(html.matches("fx-button-remove-item").count(), eq(2));
    expect_that!(html, not(contains_substring("<ol")));
    expect_that!(html, not(contains_substring("fx-move-buttons")));
}

/// As with a list: a `<ul>` may hold only `<li>`s, so the adder comes after it.
#[gtest]
fn a_sets_adder_follows_its_ul() {
    let html = super::render_to_html(PostForm);
    let ul_ends = html.find("</ul>").expect("the items are a <ul>");
    let add = html
        .find("fx-button-add-item")
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
    expect_that!(html, contains_substring("fx-button-add-item"));
}

// ── Edits ────────────────────────────────────────────────────────────────

fn add_tag(value: Option<ItemValue>) -> Edit {
    Edit::AddItem {
        path: NodePath::new("tags"),
        before: None,
        value,
    }
}

#[gtest]
fn a_set_refuses_a_move() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&Edit::MoveItem {
            path: NodePath::new("tags"),
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

/// A list's Add makes a blank item to type into; a set's commits its draft
/// (box 7), and an empty draft adds nothing, so a set never gets an item the
/// user cannot fill in.
#[gtest]
fn a_set_adds_nothing_from_an_empty_draft() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(form.edit(&add_tag(None)), ok(none()));
    expect_that!(form.validate(), some(eq(&post())), "the set is unchanged");
}

#[gtest]
fn a_set_takes_an_item_that_arrives_filled() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new("web".to_string())))),
        // `#2` is the draft's (box 7).
        ok(some(eq("tags.#3")))
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
            path: NodePath::new("tags"),
            index: 0,
        })
        .expect("item 0 exists")
    });
    with_store_keys(form)
}

/// The cleanup `use_form` does for a list's item reaches a set's item too: it
/// finds the item through the key entry `Items` writes for both. `tags.#2` is
/// the draft's leaf, which the store holds like any other (box 7).
#[gtest]
fn a_removed_items_value_leaves_the_store() {
    let html = super::render_to_html(RemovedSetItemProbe);
    expect_that!(store_keys(&html), eq("tags.#1 tags.#2 title"));
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
    rsx! { span { id: "tag-adder", "adds to {props.path}, which has {props.item_count}" } }
}

#[component]
fn CustomAdderPostForm() -> Element {
    let form = use_form(|| {
        form_for(
            &post(),
            FormSpec::default().with_adder(&path!(Post.tags), ItemAdder(TagAdder)),
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
        expect_that!(html, not(contains_substring("fx-button-add-item")));
    }
}

// ── A set widget, through a hand-built spec ──────────────────────────────
//
// `WIDGETS_PLAN.md` box 6, step 3h, written before `form!` could put a
// `widget:` on a set (box 8): these reach `with_set_widget` directly, which a
// hand-built spec still does, with a widget that prints what it was handed.

/// Prints its props as text, so a test reads exactly what `SetNode` built.
#[derive(Clone, Debug, Default, PartialEq)]
struct PropsAsText;

impl formoxus::widgets::Widget for PropsAsText {
    const NAME: &'static str = "props_as_text";
    const EXTENDS: formoxus::attrs::AttrSet = formoxus::attrs::AttrSet::EMPTY;
    const PARTS: &'static [formoxus::attrs::Tag] = &[];
}

impl formoxus::widgets::SetWidget for PropsAsText {
    fn render(&self, props: formoxus::widgets::SetWidgetProps) -> Element {
        let choices = props
            .choices
            .iter()
            .map(|c| format!("{}={}", c.value, c.display))
            .collect::<Vec<_>>()
            .join(",");
        let chosen = props.chosen.join(",");
        let label = props.label.clone().unwrap_or_default();
        let max_items = props
            .max_items
            .map_or(String::new(), |mi| format!(" max_items {mi}"));
        rsx! {
            div { id: "{props.id}", "data-path": "{props.path}",
                "choices [{choices}] chosen [{chosen}] required {props.required}{max_items} label [{label}]"
            }
        }
    }
}

fn triage_with_props_as_text() -> FormSpec<Triage> {
    FormSpec::default().with_set_widget(&path!(Triage.levels), PropsAsText)
}

fn low_and_high() -> Triage {
    Triage {
        levels: [Priority::Low, Priority::High].into(),
    }
}

#[component]
fn SetWidgetForm() -> Element {
    let form = use_form(|| form_for(&low_and_high(), triage_with_props_as_text()));
    form.render(using_fns! {})
}

/// The widget renders the whole set (decision H): no fieldset, items, draft or
/// Add of `SetNode`'s own. Its `choices` are the draft's, every variant;
/// `chosen` the entry of each item now in the set, in item order.
#[gtest]
fn a_set_widget_is_handed_the_choices_and_what_is_chosen() {
    let html = super::without_form_id(&super::render_to_html(SetWidgetForm));
    expect_that!(
        html,
        contains_substring(
            r#"<div id="FORMID-levels" data-path="levels">choices [Low=Low,Medium=Medium,High=High] chosen [Low,High] required false label [Levels]</div>"#
        )
    );
    expect_that!(html, not(contains_substring("fx-set")));
    expect_that!(html, not(contains_substring("fx-button-add-item")));
}

#[component]
fn SetWidgetAfterEditsForm() -> Element {
    let form = use_form(|| form_for(&low_and_high(), triage_with_props_as_text()));
    use_hook(|| {
        form.edit(&Edit::RemoveItem {
            path: NodePath::new("levels"),
            index: 0,
        })
        .expect("item 0 exists");
        form.edit(&Edit::AddItem {
            path: NodePath::new("levels"),
            before: None,
            value: Some(ItemValue::raw("Medium")),
        })
        .expect("Medium is a variant");
    });
    form.render(using_fns! {})
}

/// The edits a real set widget will send, a `RemoveItem` by index and an
/// `AddItem` by entry, reach `SetNode::edit` as the adder's do, and the next
/// render's `chosen` follows them. In the set's order, not the order of the
/// edits: Medium is added last but sorts before High.
#[gtest]
fn a_set_widgets_chosen_follows_its_edits() {
    let html = super::render_to_html(SetWidgetAfterEditsForm);
    expect_that!(html, contains_substring("chosen [Medium,High]"));
}

/// The widget changes how the set renders, not what it holds.
#[gtest]
fn a_set_with_a_widget_still_round_trips() {
    let triage = low_and_high();
    expect_that!(
        form_for(&triage, triage_with_props_as_text()).validate(),
        some(eq(&triage))
    );
}

/// The build's backstop for a hand-built spec: a leaf widget on a set compiles
/// (`with_widget` takes any path), so the build refuses it, naming the builder
/// that was meant.
#[gtest]
#[should_panic(
    expected = "levels is a set, which takes a SET widget, not one that renders a \
                           single value — give it one with `with_set_widget`"
)]
fn a_leaf_widget_on_a_set_names_with_set_widget() {
    let _ = form_for(
        &low_and_high(),
        FormSpec::default().with_widget(&path!(Triage.levels), formoxus::widgets::Textarea),
    );
}

#[gtest]
#[should_panic(expected = "applies only to a set")]
fn with_set_widget_refuses_a_path_that_is_not_a_set() {
    let _ = FormSpec::<Post>::default().with_set_widget(&path!(Post.title), PropsAsText);
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
#[should_panic(expected = "tags: a set of `Tag` has no default order; give it a sort_by")]
fn a_set_of_non_scalars_without_sort_by_fails_at_construction() {
    let _ = empty_form::<Shelf>(FormSpec::default());
}

/// Shortest first, unlike `Tag`'s own `Ord`, so a set that ignored its
/// `sort_by` would come out in a different order.
fn by_length(a: &Tag, b: &Tag) -> Ordering {
    a.0.len().cmp(&b.0.len()).then_with(|| a.0.cmp(&b.0))
}

fn shelf_spec() -> FormSpec<Shelf> {
    FormSpec::default().with_sort_by(&path!(Shelf.tags), SetOrder::custom(by_length, "by_length"))
}

fn shelf(names: &[&str]) -> Shelf {
    Shelf {
        tags: names.iter().map(|name| Tag(name.to_string())).collect(),
    }
}

fn values(pairs: &[(&str, &str)]) -> ValuesByPath {
    pairs
        .iter()
        .map(|(k, v)| (NodePath::new(k.to_string()), v.to_string()))
        .collect()
}

/// The set's item keys, in the order its items sit: `collect_values` writes
/// them at the set's own path in that order, so this is the order on screen.
fn item_keys<T: Clone + std::fmt::Debug + PartialEq + Facet<'static>>(
    form: &FormState<T>,
    set: &str,
) -> String {
    form.collect_values()
        .get(set)
        .cloned()
        .unwrap_or_else(|| panic!("{set} is a set, so it writes its keys"))
}

/// What each item of `tags` holds, in the order the items sit. A `Tag` is a
/// newtype, so a leaf, so each item is exactly one value at `tags.#N`.
fn tag_values(form: &FormState<Shelf>) -> Vec<String> {
    form.collect_values()
        .into_iter()
        .filter(|(path, _)| path.as_str().starts_with("tags.#"))
        .map(|(_, raw)| raw)
        .collect()
}

/// At build, the `BTreeSet` hands its items over in `Tag`'s own order
/// (dioxus, go, rust); the set sorts them by `by_length` BEFORE the keys are
/// assigned, so `#0` is the shortest, not the first the set iterated.
#[gtest]
fn a_set_with_sort_by_is_in_that_order_when_built() {
    let form = form_for(&shelf(&["dioxus", "go", "rust"]), shelf_spec());
    expect_that!(
        tag_values(&form),
        elements_are![eq("go"), eq("rust"), eq("dioxus")]
    );
    expect_that!(item_keys(&form, "tags"), eq("#0,#1,#2"));
}

/// An added item takes the next free key, `#4` (the draft has `#3`), and then
/// its place in the order:
/// a key names an item, not a position, so sorting moves items and leaves
/// every key with the item it named.
#[gtest]
fn a_set_with_sort_by_puts_an_added_item_in_order() {
    let mut form = form_for(&shelf(&["dioxus", "go", "rust"]), shelf_spec());
    expect_that!(
        form.edit(&Edit::AddItem {
            path: NodePath::new("tags"),
            before: None,
            value: Some(ItemValue::new(Tag("css".to_string()))),
        }),
        ok(some(eq("tags.#4")))
    );
    expect_that!(
        tag_values(&form),
        elements_are![eq("go"), eq("css"), eq("rust"), eq("dioxus")]
    );
    expect_that!(item_keys(&form, "tags"), eq("#0,#4,#1,#2"));
}

/// The server's case: a submit rebuilds the items in the order the wire
/// names them, and if it fails, renders them again. The set sorts once the
/// leaves are filled, not when the items are made, since a new item is
/// empty until `distribute_values` reaches its leaf.
#[gtest]
fn a_set_with_sort_by_sorts_the_items_a_submit_rebuilds() {
    let mut form = empty_form::<Shelf>(shelf_spec());
    form.distribute_values(&values(&[
        ("tags", "#0,#1,#2"),
        ("tags.#0", "dioxus"),
        ("tags.#1", "rust"),
        ("tags.#2", "go"),
    ]));
    expect_that!(
        tag_values(&form),
        elements_are![eq("go"), eq("rust"), eq("dioxus")]
    );
    expect_that!(item_keys(&form, "tags"), eq("#2,#1,#0"));
    expect_that!(form.validate(), some(eq(&shelf(&["dioxus", "go", "rust"]))));
}

#[derive(Facet, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Pin {
    label: String,
    rank: u8,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Board {
    pins: BTreeSet<Pin>,
}

fn by_rank(a: &Pin, b: &Pin) -> Ordering {
    a.rank.cmp(&b.rank)
}

/// A struct item is a node with no value of its own, so a custom order
/// rebuilds each item's `Pin` to compare it. An item that cannot be rebuilt,
/// a blank `rank` (`Empty`) or an unparseable one (`Invalid`), must not
/// panic: `write_value_into` hits `unreachable!` on either, which is why the
/// order validates a copy first. Both go last, in the order they arrived.
#[gtest]
fn a_set_item_that_cannot_be_built_sorts_last_without_panicking() {
    let mut form = empty_form::<Board>(
        FormSpec::default().with_sort_by(&path!(Board.pins), SetOrder::custom(by_rank, "by_rank")),
    );
    form.distribute_values(&values(&[
        ("pins", "#0,#1,#2,#3"),
        ("pins.#0.label", "blank"),
        ("pins.#0.rank", ""),
        ("pins.#1.label", "second"),
        ("pins.#1.rank", "2"),
        ("pins.#2.label", "invalid"),
        ("pins.#2.rank", "x"),
        ("pins.#3.label", "first"),
        ("pins.#3.rank", "1"),
    ]));
    expect_that!(item_keys(&form, "pins"), eq("#3,#1,#0,#2"));
    expect_that!(form.validate(), none());
}

// ── `sort_by` in `form!` ─────────────────────────────────────────────────

/// A fn, its `T` inferred from the field: the same order as `with_sort_by`
/// gives, at build and on the server's rebuild.
#[gtest]
fn form_sort_by_takes_a_fn() {
    let spec = || form! { Shelf { tags => { sort_by: by_length } } };
    let form = form_for(&shelf(&["dioxus", "go", "rust"]), spec());
    expect_that!(
        tag_values(&form),
        elements_are![eq("go"), eq("rust"), eq("dioxus")]
    );

    let mut rebuilt = empty_form::<Shelf>(spec());
    rebuilt.distribute_values(&values(&[
        ("tags", "#0,#1,#2"),
        ("tags.#0", "dioxus"),
        ("tags.#1", "rust"),
        ("tags.#2", "go"),
    ]));
    expect_that!(
        tag_values(&rebuilt),
        elements_are![eq("go"), eq("rust"), eq("dioxus")]
    );
}

fn post_tags(form: &FormState<Post>) -> Vec<String> {
    form.collect_values()
        .into_iter()
        .filter(|(path, _)| path.as_str().starts_with("tags.#"))
        .map(|(_, raw)| raw)
        .collect()
}

fn post_tagged(names: &[&str]) -> Post {
    Post {
        title: "Hello".to_string(),
        tags: tags(names),
    }
}

/// A path to a trait method coerces to `fn(&String, &String) -> Ordering`
/// as a fn does. The order is the default one; what this pins is that the
/// path is accepted.
#[gtest]
fn form_sort_by_takes_a_method_path() {
    let form = form_for(
        &post_tagged(&["b", "a", "c"]),
        form! { Post { tags => { sort_by: String::cmp } } },
    );
    expect_that!(post_tags(&form), elements_are![eq("a"), eq("b"), eq("c")]);
}

/// The closure's argument types are not written: the projection pins the
/// item type before the closure is checked, so `a.len()` resolves.
#[gtest]
fn form_sort_by_takes_an_unannotated_closure() {
    let form = form_for(
        &post_tagged(&["ccc", "a", "bb"]),
        form! { Post { tags => { sort_by: |a, b| a.len().cmp(&b.len()) } } },
    );
    expect_that!(
        post_tags(&form),
        elements_are![eq("a"), eq("bb"), eq("ccc")]
    );
}

/// Through the prelude, as a dev names it. By plain byte order "Banana"
/// sorts before "apple"; ignoring case, after.
#[gtest]
fn form_sort_by_takes_a_case_insensitive_order() {
    let form = form_for(
        &post_tagged(&["apple", "Banana"]),
        form! { Post { tags => { sort_by: unicode_case_insensitive } } },
    );
    expect_that!(post_tags(&form), elements_are![eq("apple"), eq("Banana")]);
}

/// The name is `SetOrder`'s, for its `Debug`: what `(order, name)` wrote,
/// or else the order as written.
#[gtest]
fn form_sort_by_names_the_order() {
    let named = form! { Post { tags => { sort_by: (String::cmp, "byte order") } } };
    expect_that!(
        format!("{named:?}"),
        contains_substring(r#"name: "byte order""#)
    );
    let unnamed = form! { Post { tags => { sort_by: String::cmp } } };
    expect_that!(
        format!("{unnamed:?}"),
        contains_substring(r#"name: "String::cmp""#)
    );
}

/// `tags?`: the order is over what the `Option` holds.
#[gtest]
fn form_sort_by_reaches_a_set_behind_an_option() {
    let maybe = MaybeTagged {
        tags: Some(tags(&["ccc", "a", "bb"])),
    };
    let mut form = form_for(
        &maybe,
        form! { MaybeTagged { tags? => { sort_by: |a, b| a.len().cmp(&b.len()) } } },
    );
    let order: Vec<String> = form
        .collect_values()
        .into_iter()
        .filter(|(path, _)| path.as_str().starts_with("tags.#"))
        .map(|(_, raw)| raw)
        .collect();
    expect_that!(order, elements_are![eq("a"), eq("bb"), eq("ccc")]);
    expect_that!(form.validate(), some(eq(&maybe)));
}

#[derive(Facet, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Triage {
    levels: BTreeSet<Priority>,
}

/// Highest first, against `Priority`'s own order. By reference although
/// `Copy`, since `fn(&T, &T) -> Ordering` is what `sort_by` takes.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn most_urgent_first(a: &Priority, b: &Priority) -> Ordering {
    b.cmp(a)
}

/// A unit enum's `sort_by` replaces its default declaration order (decision
/// O, below): it sorts at build and on the server's rebuild, and still
/// round-trips.
#[gtest]
fn a_unit_enum_set_sorts_by_its_sort_by() {
    let spec = || form! { Triage { levels => { sort_by: most_urgent_first } } };
    let triage = Triage {
        levels: [Priority::Low, Priority::High, Priority::Medium].into(),
    };
    let mut form = form_for(&triage, spec());
    let wire = form.collect_values();
    expect_that!(wire.get("levels"), some(eq("#0,#1,#2")));
    expect_that!(wire.get("levels.#0"), some(eq("High")));
    expect_that!(wire.get("levels.#2"), some(eq("Low")));
    expect_that!(form.validate(), some(eq(&triage)));

    let mut rebuilt = empty_form::<Triage>(spec());
    rebuilt.distribute_values(&values(&[
        ("levels", "#0,#1,#2"),
        ("levels.#0", "Low"),
        ("levels.#1", "High"),
        ("levels.#2", "Medium"),
    ]));
    expect_that!(item_keys(&rebuilt, "levels"), eq("#1,#2,#0"));
    expect_that!(rebuilt.validate(), some(eq(&triage)));
}

// ── A unit enum's default order (`WIDGETS_PLAN.md` decision O) ───────────

/// Declared small to large, but ORDERED large to small, so a test can tell
/// declaration order from `Ord`: a `BTreeSet<Size>` iterates Large first.
#[derive(Facet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
enum Size {
    Small,
    Medium,
    Large,
}

impl Ord for Size {
    fn cmp(&self, other: &Self) -> Ordering {
        (*other as u8).cmp(&(*self as u8))
    }
}

impl PartialOrd for Size {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Order {
    sizes: BTreeSet<Size>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Bin {
    sizes: HashSet<Size>,
}

fn all_sizes() -> [Size; 3] {
    [Size::Large, Size::Small, Size::Medium]
}

/// Each item's entry, in item order: the set's order as a reader sees it.
fn entries<T: Clone + std::fmt::Debug + PartialEq + Facet<'static>>(
    form: &FormState<T>,
    set: &str,
) -> Vec<String> {
    let values = form.collect_values();
    item_keys(form, set)
        .split(',')
        .map(|key| {
            values
                .get(format!("{set}.{key}").as_str())
                .cloned()
                .unwrap_or_else(|| panic!("{set}.{key} has an entry"))
        })
        .collect()
}

/// No `sort_by`, and no panic: declaration order, not the enum's `Ord`.
#[gtest]
fn a_unit_enum_set_sorts_in_declaration_order_by_default() {
    let order = Order {
        sizes: all_sizes().into(),
    };
    let form = form_for(&order, FormSpec::default());
    expect_that!(
        entries(&form, "sizes"),
        elements_are![eq("Small"), eq("Medium"), eq("Large")]
    );
}

/// A `HashSet` iterates differently in each process; sorting before keys are
/// assigned is what makes a server's render and a client's hydration agree.
#[gtest]
fn a_unit_enum_hash_set_sorts_in_declaration_order_too() {
    let bin = Bin {
        sizes: all_sizes().into(),
    };
    let form = form_for(&bin, FormSpec::default());
    expect_that!(
        entries(&form, "sizes"),
        elements_are![eq("Small"), eq("Medium"), eq("Large")]
    );
}

/// An item added by entry goes where declaration order puts it, not last.
#[gtest]
fn an_added_unit_enum_item_takes_its_declared_place() {
    let mut form = form_for(
        &Order {
            sizes: [Size::Small, Size::Large].into(),
        },
        FormSpec::default(),
    );
    expect_that!(
        form.edit(&Edit::AddItem {
            path: NodePath::new("sizes"),
            before: None,
            value: Some(ItemValue::raw("Medium")),
        }),
        // `#3`, not `#2`: the draft already holds the next key.
        ok(some(eq("sizes.#3")))
    );
    expect_that!(
        entries(&form, "sizes"),
        elements_are![eq("Small"), eq("Medium"), eq("Large")]
    );
}

/// The draft's way in, choosing a variant and then Add, sorts the same.
#[gtest]
fn a_unit_enum_draft_adds_in_declared_place() {
    let mut form = form_for(
        &Order {
            sizes: [Size::Small, Size::Large].into(),
        },
        FormSpec::default(),
    );
    // The draft takes the next key after the two items'.
    expect_that!(
        form.edit(&Edit::ChooseVariant {
            path: NodePath::new("sizes.#2"),
            variant: Some("Medium".to_string()),
        }),
        ok(none())
    );
    expect_that!(
        form.edit(&Edit::AddItem {
            path: NodePath::new("sizes"),
            before: None,
            value: None,
        }),
        ok(some(eq("sizes.#2")))
    );
    expect_that!(
        entries(&form, "sizes"),
        elements_are![eq("Small"), eq("Medium"), eq("Large")]
    );
}

/// Equal is the same variant: adding one already there adds nothing.
#[gtest]
fn a_unit_enum_set_refuses_a_variant_it_has() {
    let order = Order {
        sizes: [Size::Small, Size::Large].into(),
    };
    let mut form = form_for(&order, FormSpec::default());
    expect_that!(
        form.edit(&Edit::AddItem {
            path: NodePath::new("sizes"),
            before: None,
            value: Some(ItemValue::raw("Large")),
        }),
        ok(none())
    );
    expect_that!(form.validate(), some(eq(&order)));
}

/// `form!`'s compile-time check agrees with the run time: a unit enum set
/// written with no `sort_by` compiles, and `set` on a `Vec` of one does too.
#[gtest]
fn form_takes_a_unit_enum_set_without_sort_by() {
    let order = Order {
        sizes: all_sizes().into(),
    };
    let mut form = form_for(&order, form! { Order { sizes => { label: "Sizes" } } });
    expect_that!(
        entries(&form, "sizes"),
        elements_are![eq("Small"), eq("Medium"), eq("Large")]
    );
    expect_that!(form.validate(), some(eq(&order)));
}

#[gtest]
fn a_unit_enum_set_in_default_order_round_trips() {
    let order = Order {
        sizes: all_sizes().into(),
    };
    expect_that!(
        form_for(&order, FormSpec::default()).validate(),
        some(eq(&order))
    );
    expect_that!(across_the_wire(&order), ok(eq(&order)));
    let bin = Bin {
        sizes: all_sizes().into(),
    };
    expect_that!(across_the_wire(&bin), ok(eq(&bin)));
}

// ── `set` on a `Vec` ─────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Draft {
    tags: Vec<String>,
}

/// `form!`'s `set` is `treat_as_set` on the spec, so the two build the same
/// spec. Rendering and writing a `Vec` as a set are 5b–5e's tests.
#[gtest]
fn form_set_treats_the_list_as_a_set() {
    expect_that!(
        format!("{:?}", form! { Draft { tags => { set } } }),
        eq(&format!(
            "{:?}",
            FormSpec::<Draft>::default().treat_as_set(&path!(Draft.tags))
        ))
    );
}

/// `with_sort_by` on a `Vec` panics unless `treat_as_set` came first;
/// `form!` emits them in that order whichever order the keys are written.
#[gtest]
fn form_set_and_sort_by_on_a_vec_build_a_spec() {
    let spec = form! { Draft { tags => { sort_by: unicode_case_insensitive, set } } };
    expect_that!(
        format!("{spec:?}"),
        contains_substring(r#"name: "unicode_case_insensitive""#)
    );
}

fn draft_tags(form: &FormState<Draft>) -> Vec<String> {
    form.collect_values()
        .into_iter()
        .filter(|(path, _)| path.as_str().starts_with("tags.#"))
        .map(|(_, raw)| raw)
        .collect()
}

/// A `Vec` can hold a duplicate and a set cannot, so the second "rust" is
/// dropped when the form is built (5c), before the keys are assigned: three
/// items, `#0..#2`, sorted. And the model gets back what the screen shows, a
/// sorted `Vec` with no duplicate (5e).
#[gtest]
fn a_list_treated_as_a_set_drops_its_duplicates_when_built() {
    let draft = Draft {
        tags: tags(&["rust", "dioxus", "rust", "css"]),
    };
    let mut form = form_for(&draft, FormSpec::default().treat_as_set(&path!(Draft.tags)));
    expect_that!(
        draft_tags(&form),
        elements_are![eq("css"), eq("dioxus"), eq("rust")]
    );
    expect_that!(item_keys(&form, "tags"), eq("#0,#1,#2"));
    expect_that!(
        form.validate(),
        some(eq(&Draft {
            tags: tags(&["css", "dioxus", "rust"]),
        }))
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct TagList {
    tags: Vec<Tag>,
}

/// Equal is `Tag`'s own `==`, not the order: "ab" and "cd" compare `Equal`
/// by length but are different tags, so both stay, and only the repeated
/// "ab" goes, though "cd" sits between the two.
#[gtest]
fn a_list_treated_as_a_set_drops_by_equality_not_by_its_order() {
    let list = TagList {
        tags: ["ab", "cd", "ab", "xyz"]
            .iter()
            .map(|name| Tag(name.to_string()))
            .collect(),
    };
    let mut form = form_for(
        &list,
        FormSpec::default()
            .treat_as_set(&path!(TagList.tags))
            .with_sort_by(
                &path!(TagList.tags),
                SetOrder::custom(by_length, "by_length"),
            ),
    );
    let values: Vec<String> = form
        .collect_values()
        .into_iter()
        .filter(|(path, _)| path.as_str().starts_with("tags.#"))
        .map(|(_, raw)| raw)
        .collect();
    expect_that!(values, elements_are![eq("ab"), eq("cd"), eq("xyz")]);
    expect_that!(
        form.validate().map(|list| list.tags),
        some(elements_are![
            eq(&Tag("ab".to_string())),
            eq(&Tag("cd".to_string())),
            eq(&Tag("xyz".to_string()))
        ])
    );
}

/// The markup and the refusals are a set's, not a list's: a `<ul>`, Remove
/// on each item and no ↑/↓, through the spec and through `form!` alike.
#[component]
fn DraftAsSetForm() -> Element {
    let draft = Draft {
        tags: tags(&["dioxus", "rust"]),
    };
    let form =
        use_form(move || form_for(&draft, FormSpec::default().treat_as_set(&path!(Draft.tags))));
    form.render(using_fns! {})
}

#[component]
fn DraftAsSetMacroForm() -> Element {
    let draft = Draft {
        tags: tags(&["dioxus", "rust"]),
    };
    let form = use_form(move || form_for(&draft, form! { Draft { tags => { set } } }));
    form.render(using_fns! {})
}

#[gtest]
fn a_list_treated_as_a_set_renders_as_a_set() {
    for html in [
        super::render_to_html(DraftAsSetForm),
        super::render_to_html(DraftAsSetMacroForm),
    ] {
        expect_that!(html.matches(r#"<ul class="fx-set-items">"#).count(), eq(1));
        expect_that!(html.matches(r#"<li class="fx-set-item">"#).count(), eq(2));
        expect_that!(html.matches("fx-button-remove-item").count(), eq(2));
        expect_that!(html, not(contains_substring("<ol")));
        expect_that!(html, not(contains_substring("fx-move-buttons")));
    }
}

fn draft_with(names: &[&str]) -> FormState<Draft> {
    form_for(
        &Draft { tags: tags(names) },
        FormSpec::default().treat_as_set(&path!(Draft.tags)),
    )
}

#[gtest]
fn a_list_treated_as_a_set_refuses_a_move_and_adds_nothing_from_an_empty_draft() {
    let mut form = draft_with(&["dioxus", "rust"]);
    expect_that!(
        form.edit(&Edit::MoveItem {
            path: NodePath::new("tags"),
            from: 0,
            to: 1,
        })
        .map_err(|e| e.0),
        err(contains_substring(
            "tags is a set; you cannot reorder its items"
        ))
    );
    expect_that!(form.edit(&add_tag(None)), ok(none()));
    expect_that!(item_keys(&form, "tags"), eq("#0,#1"));
}

/// An added item takes the next free key (the draft has `#2`) and its place
/// in the order, and the model's `Vec` follows the order, not the order of
/// arrival.
#[gtest]
fn a_list_treated_as_a_set_puts_an_added_item_in_order() {
    let mut form = draft_with(&["dioxus", "rust"]);
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new("css".to_string())))),
        ok(some(eq("tags.#3")))
    );
    expect_that!(item_keys(&form, "tags"), eq("#3,#0,#1"));
    expect_that!(
        form.validate(),
        some(eq(&Draft {
            tags: tags(&["css", "dioxus", "rust"]),
        }))
    );
}

/// The wire can name two items with equal values (a script, say). The second
/// is dropped when the values are distributed (5d), beside the sort, so the
/// nodes and the model hold the same items, and a failed submit would
/// re-render without it. Decision D: dropped quietly, not refused.
#[gtest]
fn a_list_treated_as_a_set_drops_a_duplicate_from_the_wire() {
    let mut form = empty_form::<Draft>(FormSpec::default().treat_as_set(&path!(Draft.tags)));
    form.distribute_values(&values(&[
        ("tags", "#0,#1,#2"),
        ("tags.#0", "rust"),
        ("tags.#1", "css"),
        ("tags.#2", "rust"),
    ]));
    expect_that!(draft_tags(&form), elements_are![eq("css"), eq("rust")]);
    expect_that!(item_keys(&form, "tags"), eq("#1,#0"));
    expect_that!(
        form.validate(),
        some(eq(&Draft {
            tags: tags(&["css", "rust"]),
        }))
    );
}

/// A `BTreeSet`'s write always collapsed a duplicate; before 5d its nodes
/// kept both, so the screen showed an item the model did not have.
#[gtest]
fn a_set_drops_a_duplicate_from_the_wire_from_its_nodes_too() {
    let mut form = empty_form::<Post>(FormSpec::default());
    form.distribute_values(&values(&[
        ("title", "Hello"),
        ("tags", "#0,#1"),
        ("tags.#0", "rust"),
        ("tags.#1", "rust"),
    ]));
    expect_that!(post_tags(&form), elements_are![eq("rust")]);
    expect_that!(form.validate(), some(eq(&post_tagged(&["rust"]))));
}

/// An item with no value equals nothing, not even another item with none:
/// both blanks stay, after the item that has a value, and the submit fails
/// on them as it would anyway.
#[gtest]
fn blank_items_are_not_duplicates_of_each_other() {
    let mut form = empty_form::<Draft>(FormSpec::default().treat_as_set(&path!(Draft.tags)));
    form.distribute_values(&values(&[
        ("tags", "#0,#1,#2"),
        ("tags.#0", ""),
        ("tags.#1", ""),
        ("tags.#2", "rust"),
    ]));
    expect_that!(item_keys(&form, "tags"), eq("#2,#0,#1"));
    expect_that!(form.validate(), none());
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct MaybeDraft {
    tags: Option<Vec<String>>,
}

/// Behind an `Option`, written with its `?` as every optional path is: the
/// payload is the set, built, sorted, deduplicated and written as a `Vec`.
#[gtest]
fn a_list_behind_an_option_can_be_treated_as_a_set() {
    let maybe = MaybeDraft {
        tags: Some(tags(&["rust", "css", "rust"])),
    };
    let mut form = form_for(&maybe, form! { MaybeDraft { tags? => { set } } });
    expect_that!(
        form.validate(),
        some(eq(&MaybeDraft {
            tags: Some(tags(&["css", "rust"])),
        }))
    );
    let mut none = form_for(
        &MaybeDraft { tags: None },
        form! { MaybeDraft { tags? => { set } } },
    );
    expect_that!(none.validate(), some(eq(&MaybeDraft { tags: None })));
}

/// A `Vec` of newtypes treated as a set needs `sort_by`, as a set of them
/// does: `form!` refuses it at compile time (`form_set_flag_without_sort_by`),
/// and a hand-built spec when the form is built.
#[gtest]
#[should_panic(expected = "tags: a set of `Tag` has no default order; give it a sort_by")]
fn a_list_of_newtypes_treated_as_a_set_without_sort_by_fails_at_construction() {
    let _ = empty_form::<TagList>(FormSpec::default().treat_as_set(&path!(TagList.tags)));
}

// ── Duplicates refused at the adder (box 6) ──────────────────────────────

/// An `AddItem` equal to an item is refused for the user, not the caller:
/// `Ok(None)`, nothing added, nothing to log (decision C, box 6 B).
#[gtest]
fn a_set_refuses_a_duplicate_without_an_error() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new("rust".to_string())))),
        ok(none())
    );
    expect_that!(form.validate(), some(eq(&post())), "the set is unchanged");
}

/// Equal is the model's `==`, not the order (H): under a case-insensitive
/// order "Rust" sorts with "rust", but is a different `String`, so it is
/// added, as a `BTreeSet<String>` would hold both.
#[gtest]
fn a_set_takes_an_item_its_order_ties_but_its_type_does_not_equal() {
    let mut form = form_for(
        &post(),
        form! { Post { tags => { sort_by: unicode_case_insensitive } } },
    );
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new("Rust".to_string())))),
        ok(some(eq("tags.#3")))
    );
    expect_that!(
        form.validate(),
        some(eq(&post_tagged(&["dioxus", "Rust", "rust"])))
    );
}

/// The same through a custom order, which compares with `T`'s own `==`:
/// "go" and "js" are the same length, so tie under `by_length`, but only a
/// second "go" is a duplicate.
#[gtest]
fn a_set_with_sort_by_refuses_only_an_equal_item() {
    let mut form = form_for(&shelf(&["go", "rust"]), shelf_spec());
    let add = |name: &str| Edit::AddItem {
        path: NodePath::new("tags"),
        before: None,
        value: Some(ItemValue::new(Tag(name.to_string()))),
    };
    expect_that!(form.edit(&add("go")), ok(none()));
    expect_that!(form.edit(&add("js")), ok(some(eq("tags.#3"))));
    expect_that!(form.validate(), some(eq(&shelf(&["go", "js", "rust"]))));
}

/// A `Vec` treated as a set refuses one too.
#[gtest]
fn a_list_treated_as_a_set_refuses_a_duplicate() {
    let mut form = draft_with(&["dioxus", "rust"]);
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new("dioxus".to_string())))),
        ok(none())
    );
    expect_that!(
        form.validate(),
        some(eq(&Draft {
            tags: tags(&["dioxus", "rust"]),
        }))
    );
}

/// A value of the wrong type is still the caller's mistake, refused for its
/// type, not mistaken for an item that is there or not.
#[gtest]
fn a_wrong_type_is_still_an_error_not_a_duplicate() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new(7_u8)))),
        err(displays_as(contains_substring(
            "cannot add a `u8` item to tags: its items are `String`"
        )))
    );
}

// ── The set's live region ────────────────────────────────────────────────

/// The region's whole `<div>`, as the set renders it after its adder.
fn region(class: &str, text: &str) -> String {
    format!(r#"<div class="{class}" role="status">{text}</div>"#)
}

fn done(text: &str) -> String {
    region("fx-set-message", text)
}

fn refused(text: &str) -> String {
    region("fx-set-message fx-invalid", text)
}

/// Present and empty before any edit: a `status` region is announced
/// reliably only if it is already in the page when its text changes.
#[gtest]
fn a_sets_live_region_is_there_and_empty_before_any_edit() {
    expect_that!(
        super::render_to_html(PostForm),
        contains_substring(region("fx-set-message", ""))
    );
}

#[component]
fn AddProbe() -> Element {
    let form = use_form(|| form_for(&post(), FormSpec::default()));
    use_hook(|| {
        form.edit(&add_tag(Some(ItemValue::new("web".to_string()))))
            .expect("a new value is added")
    });
    form.render(using_fns! {})
}

#[component]
fn RemoveProbe() -> Element {
    let form = use_form(|| form_for(&post(), FormSpec::default()));
    use_hook(|| {
        form.edit(&Edit::RemoveItem {
            path: NodePath::new("tags"),
            index: 1,
        })
        .expect("item 1 exists")
    });
    form.render(using_fns! {})
}

#[component]
fn DuplicateAddProbe() -> Element {
    let form = use_form(|| form_for(&post(), FormSpec::default()));
    use_hook(|| {
        form.edit(&add_tag(Some(ItemValue::new("rust".to_string()))))
            .expect("a duplicate is refused without an error")
    });
    form.render(using_fns! {})
}

/// Each edit says what it did, Django's way: an add and a removal as news, a
/// duplicate as a refusal, in the slot's invalid class.
#[gtest]
fn each_edit_says_what_it_did() {
    expect_that!(
        super::render_to_html(AddProbe),
        contains_substring(done("“web” was added successfully."))
    );
    expect_that!(
        super::render_to_html(RemoveProbe),
        contains_substring(done("“rust” was removed successfully."))
    );
    expect_that!(
        super::render_to_html(DuplicateAddProbe),
        contains_substring(refused("“rust” is already there."))
    );
}

/// After the adder, where the user just acted.
#[gtest]
fn the_region_follows_the_adder() {
    let html = super::render_to_html(DuplicateAddProbe);
    let add = html.find("fx-button-add-item").expect("the set has Add");
    let message = html.find("fx-set-message").expect("the set has its region");
    expect_that!(message, gt(add));
}

#[component]
fn DuplicateThenAddProbe() -> Element {
    let form = use_form(|| form_for(&post(), FormSpec::default()));
    use_hook(|| {
        form.edit(&add_tag(Some(ItemValue::new("rust".to_string()))))
            .expect("refused without an error");
        form.edit(&add_tag(Some(ItemValue::new("web".to_string()))))
            .expect("a new value is added")
    });
    form.render(using_fns! {})
}

/// A message answers the last edit only: the refusal does not linger once
/// the next edit has done something else.
#[gtest]
fn the_next_edit_replaces_the_message() {
    let html = super::render_to_html(DuplicateThenAddProbe);
    expect_that!(html, not(contains_substring("is already there")));
    expect_that!(
        html,
        contains_substring(done("“web” was added successfully."))
    );
}

#[component]
fn DuplicateTagProbe() -> Element {
    let form = use_form(|| form_for(&shelf(&["go", "rust"]), shelf_spec()));
    use_hook(|| {
        form.edit(&Edit::AddItem {
            path: NodePath::new("tags"),
            before: None,
            value: Some(ItemValue::new(Tag("go".to_string()))),
        })
        .expect("refused without an error")
    });
    form.render(using_fns! {})
}

#[component]
fn AddPriorityProbe() -> Element {
    let form = use_form(|| {
        form_for(
            &Triage {
                levels: [Priority::Low].into(),
            },
            form! { Triage { levels => { sort_by: most_urgent_first } } },
        )
    });
    use_hook(|| {
        form.edit(&Edit::AddItem {
            path: NodePath::new("levels"),
            before: None,
            value: Some(ItemValue::new(Priority::High)),
        })
        .expect("a new level is added")
    });
    form.render(using_fns! {})
}

/// An item is named as the user sees it, not by a `Display` its type may not
/// have: a newtype by its inner value, a unit enum by its variant.
#[gtest]
fn an_item_is_named_as_the_user_sees_it() {
    expect_that!(
        super::render_to_html(DuplicateTagProbe),
        contains_substring(refused("“go” is already there."))
    );
    expect_that!(
        super::render_to_html(AddPriorityProbe),
        contains_substring(done("“High” was added successfully."))
    );
}

#[component]
fn DuplicateCustomAdderProbe() -> Element {
    let form = use_form(|| {
        form_for(
            &post(),
            FormSpec::default().with_adder(&path!(Post.tags), ItemAdder(TagAdder)),
        )
    });
    use_hook(|| {
        form.edit(&add_tag(Some(ItemValue::new("rust".to_string()))))
            .expect("refused without an error")
    });
    form.render(using_fns! {})
}

/// The set renders the region, not the adder, so a custom adder has it too.
#[gtest]
fn a_custom_adder_still_has_the_sets_message() {
    let html = super::render_to_html(DuplicateCustomAdderProbe);
    let adder = html.find(r#"id="tag-adder""#).expect("the custom adder");
    let message = html
        .find(&refused("“rust” is already there."))
        .expect("the refusal is shown");
    expect_that!(message, gt(adder));
}

// ── A dev's own words: `feedback:` ───────────────────────────────────────

/// A named `fn` works as a closure does. `_ =>` because `SetEvent` is
/// `#[non_exhaustive]`.
fn shout(event: SetEvent<'_, String>) -> Option<String> {
    match event {
        SetEvent::Added(tag) => Some(format!("ADDED {tag}")),
        SetEvent::Removed(tag) => Some(format!("REMOVED {tag}")),
        SetEvent::Duplicate(tag) => Some(format!("ALREADY {tag}")),
        _ => None,
    }
}

#[component]
fn ShoutingAddProbe() -> Element {
    let form = use_form(|| form_for(&post(), form! { Post { tags => { feedback: shout } } }));
    use_hook(|| {
        form.edit(&add_tag(Some(ItemValue::new("web".to_string()))))
            .expect("added")
    });
    form.render(using_fns! {})
}

#[component]
fn ShoutingRemoveProbe() -> Element {
    let form = use_form(|| form_for(&post(), form! { Post { tags => { feedback: shout } } }));
    use_hook(|| {
        form.edit(&Edit::RemoveItem {
            path: NodePath::new("tags"),
            index: 1,
        })
        .expect("item 1 exists")
    });
    form.render(using_fns! {})
}

#[component]
fn ShoutingDuplicateProbe() -> Element {
    let form = use_form(|| form_for(&post(), form! { Post { tags => { feedback: shout } } }));
    use_hook(|| {
        form.edit(&add_tag(Some(ItemValue::new("rust".to_string()))))
            .expect("refused without an error")
    });
    form.render(using_fns! {})
}

/// The dev's words replace formoxus's for every event, and a duplicate is
/// still a refusal, in the invalid class, whoever wrote its text.
#[gtest]
fn feedback_puts_each_event_in_the_devs_words() {
    expect_that!(
        super::render_to_html(ShoutingAddProbe),
        contains_substring(done("ADDED web"))
    );
    expect_that!(
        super::render_to_html(ShoutingRemoveProbe),
        contains_substring(done("REMOVED rust"))
    );
    expect_that!(
        super::render_to_html(ShoutingDuplicateProbe),
        contains_substring(refused("ALREADY rust"))
    );
}

#[component]
fn ToasterProbe() -> Element {
    // Stands in for an app's toaster: the closure captures a signal and
    // returns `None`, so the set's own region stays empty. A SYNC signal: the
    // hook lives in the spec, which must be `Send` for a server handler, and
    // a plain `use_signal`'s storage is a `RefCell`.
    let toasts = use_signal_sync(Vec::<String>::new);
    let form = use_form(move || {
        form_for(
            &post(),
            form! { Post { tags => { feedback: move |event| {
                // `Fn`, so a copy to write through: a signal is `Copy`.
                let mut toasts = toasts;
                toasts.write().push(format!("{event:?}"));
                None
            } } } },
        )
    });
    use_hook(|| {
        form.edit(&add_tag(Some(ItemValue::new("web".to_string()))))
            .expect("added");
        form.edit(&add_tag(Some(ItemValue::new("web".to_string()))))
            .expect("refused without an error")
    });
    rsx! {
        {form.render(using_fns! {})}
        ul { id: "toasts",
            for toast in toasts.read().iter() {
                li { "{toast}" }
            }
        }
    }
}

/// A closure, not a `fn`, so it can hand the event to the app: here a
/// captured sync signal gets both, and `None` leaves the region empty.
#[gtest]
fn feedback_can_send_the_event_elsewhere_and_show_nothing() {
    let html = super::render_to_html(ToasterProbe);
    expect_that!(
        html,
        contains_substring(
            r#"<ul id="toasts"><li>Added(&#34;web&#34;)</li><li>Duplicate(&#34;web&#34;)</li></ul>"#
        )
    );
    expect_that!(html, contains_substring(region("fx-set-message", "")));
}

#[component]
fn TagFeedbackProbe() -> Element {
    let form = use_form(|| {
        form_for(
            &shelf(&["go", "rust"]),
            form! { Shelf { tags => {
                sort_by: by_length,
                feedback: |event| match event {
                    SetEvent::Added(tag) => Some(format!("tag {} ({} letters)", tag.0, tag.0.len())),
                    _ => None,
                },
            } } },
        )
    });
    use_hook(|| {
        form.edit(&Edit::AddItem {
            path: NodePath::new("tags"),
            before: None,
            value: Some(ItemValue::new(Tag("css".to_string()))),
        })
        .expect("added")
    });
    form.render(using_fns! {})
}

/// The event carries the dev's own type, a whole `Tag`, rebuilt from the
/// item, not the string the set shows.
#[gtest]
fn feedback_gets_the_item_as_its_own_type() {
    expect_that!(
        super::render_to_html(TagFeedbackProbe),
        contains_substring(done("tag css (3 letters)"))
    );
}

#[component]
fn HandBuiltFeedbackProbe() -> Element {
    let form = use_form(|| {
        form_for(
            &post(),
            FormSpec::default().with_feedback(
                &path!(Post.tags),
                SetFeedback::new(|event: SetEvent<'_, String>| match event {
                    SetEvent::Added(tag) => Some(format!("+{tag}")),
                    _ => None,
                }),
            ),
        )
    });
    use_hook(|| {
        form.edit(&add_tag(Some(ItemValue::new("web".to_string()))))
            .expect("added")
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_hand_built_spec_takes_feedback_too() {
    expect_that!(
        super::render_to_html(HandBuiltFeedbackProbe),
        contains_substring(done("+web"))
    );
}

#[gtest]
#[should_panic(expected = "title: `feedback` applies only to a set, not `String`")]
fn feedback_on_a_scalar_is_refused() {
    let _ = FormSpec::<Post>::default().with_feedback(
        &path!(Post.title),
        SetFeedback::new(|_: SetEvent<'_, String>| None),
    );
}

#[gtest]
#[should_panic(
    expected = "tags: if you want to treat this list as a set, call `treat_as_set` first"
)]
fn feedback_on_a_list_not_yet_treated_as_a_set_is_refused() {
    let _ = FormSpec::<Draft>::default().with_feedback(
        &path!(Draft.tags),
        SetFeedback::new(|_: SetEvent<'_, String>| None),
    );
}

// ── The draft (box 7) ────────────────────────────────────────────────────
//
// A set's Add commits its draft, a blank item rendered in the adder. Through
// `use_form` (the `Harness`) for what only it does: the draft's text lives in
// the store, and Add reads it from there. Through `FormState` for the rest,
// with the draft's value distributed in, as `apply_edit` does.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tagged {
    tags: BTreeSet<String>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Sizes {
    sizes: BTreeSet<u8>,
}

/// The form, then what its `to_wire` would send, as `path=raw` pairs.
fn with_wire<T: Clone + std::fmt::Debug + PartialEq + Facet<'static> + 'static>(
    form: &Form<T>,
    extra: Element,
) -> Element {
    let wire = form
        .to_wire()
        .values()
        .iter()
        .map(|(path, raw)| format!("{path}={raw}"))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        {form.render(using_fns! {})}
        {extra}
        output { id: "wire", "{wire}" }
    }
}

fn wire(pairs: &str) -> String {
    format!(r#"<output id="wire">{pairs}</output>"#)
}

#[component]
fn EmptyTaggedForm() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Tagged>::default()));
    with_wire(&form, rsx! {})
}

#[component]
fn EmptySizesForm() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Sizes>::default()));
    with_wire(&form, rsx! {})
}

#[component]
fn ResettableTaggedForm() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Tagged>::default()));
    // Formatted, so the label crosses as a mutation `Harness::labelled` sees.
    let label = "Reset";
    with_wire(
        &form,
        rsx! {
            button { aria_label: "{label}", onclick: move |_| form.reset(), "Reset" }
        },
    )
}

/// The draft's input, on an EMPTY set, where it is the only one: listeners
/// register in the order dioxus creates nodes, not document order, so "the
/// last input" is not the draft once there are items.
fn draft_input(app: &super::Harness) -> dioxus::core::ElementId {
    app.only_listener("input")
}

/// Add, on an empty set, where nothing else is clicked.
fn add_button(app: &super::Harness) -> dioxus::core::ElementId {
    app.only_listener("click")
}

/// What the draft's input shows, by its path.
fn draft_value(path: &str, value: &str) -> String {
    format!(r#"name="{path}" value="{value}""#)
}

/// Named for its set, since items have no labels, and not `required`: an
/// empty draft is nothing being added, and would otherwise block the submit.
#[gtest]
fn the_draft_is_named_for_its_set_and_not_required() {
    let html = super::render_to_html(EmptyTaggedForm);
    expect_that!(html, contains_substring(">Add to Tags</label>"));
    expect_that!(html, contains_substring(draft_value("tags.#0", "")));
    expect_that!(html, not(contains_substring("required")));
}

#[component]
fn LabelledDraftForm() -> Element {
    let form = use_form(|| empty_form(form! { Tagged { tags[] => { label: "Tag" } } }));
    form.render(using_fns! {})
}

/// The items' own label, when the dev gives one, is the draft's too.
#[gtest]
fn the_items_label_names_the_draft() {
    let html = super::render_to_html(LabelledDraftForm);
    expect_that!(html, contains_substring(">Tag</label>"));
    expect_that!(html, not(contains_substring("Add to Tags")));
}

#[gtest]
fn typing_into_the_draft_reaches_neither_the_wire_nor_the_set() {
    let mut app = super::Harness::mount(EmptyTaggedForm);
    app.fire("input", draft_input(&app), "web");
    expect_that!(app.html(), contains_substring(wire("tags=")));
    expect_that!(
        app.html(),
        contains_substring(draft_value("tags.#0", "web"))
    );
}

/// Add makes the draft an item under the key it had, says so, and leaves a
/// new blank draft under the next.
#[gtest]
fn add_makes_the_draft_an_item() {
    let mut app = super::Harness::mount(EmptyTaggedForm);
    app.fire("input", draft_input(&app), "web");
    app.click(add_button(&app));
    let html = app.html();
    expect_that!(html, contains_substring(wire("tags=#0 tags.#0=web")));
    expect_that!(
        html,
        contains_substring(done("“web” was added successfully."))
    );
    expect_that!(html, contains_substring(draft_value("tags.#1", "")));
}

/// Enter in the draft is Add, so a user adds without leaving the keyboard.
#[gtest]
fn enter_in_the_draft_adds_it() {
    let mut app = super::Harness::mount(EmptyTaggedForm);
    app.fire("input", draft_input(&app), "web");
    app.press(app.only_listener("keydown"), Key::Enter);
    expect_that!(app.html(), contains_substring(wire("tags=#0 tags.#0=web")));
}

/// Any other key is typing.
#[gtest]
fn another_key_in_the_draft_adds_nothing() {
    let mut app = super::Harness::mount(EmptyTaggedForm);
    app.fire("input", draft_input(&app), "web");
    app.press(app.only_listener("keydown"), Key::Tab);
    expect_that!(app.html(), contains_substring(wire("tags=")));
}

/// Refused, with the error beside the draft and the text left to fix.
#[gtest]
fn an_invalid_draft_is_refused_and_keeps_its_text() {
    let mut app = super::Harness::mount(EmptySizesForm);
    app.fire("input", draft_input(&app), "300");
    app.click(add_button(&app));
    let html = app.html();
    expect_that!(html, contains_substring(wire("sizes=")));
    expect_that!(html, contains_substring(draft_value("sizes.#0", "300")));
    expect_that!(
        html,
        contains_substring("&#34;300&#34; isn&#39;t a valid u8")
    );
    expect_that!(html, contains_substring(region("fx-set-message", "")));
}

/// The draft's input and Add are found while the set is empty, and used again
/// after an add: the draft renders in the same place, so its elements are
/// the same ones, under the next key, which is what keeps focus in it.
#[gtest]
fn a_duplicate_draft_is_refused_and_keeps_its_text() {
    let mut app = super::Harness::mount(EmptyTaggedForm);
    let (draft, add) = (draft_input(&app), add_button(&app));
    app.fire("input", draft, "rust");
    app.click(add);
    app.fire("input", draft, "rust");
    app.click(add);
    let html = app.html();
    expect_that!(html, contains_substring(wire("tags=#0 tags.#0=rust")));
    expect_that!(
        html,
        contains_substring(refused("“rust” is already there."))
    );
    expect_that!(html, contains_substring(draft_value("tags.#1", "rust")));
}

#[gtest]
fn reset_clears_the_draft() {
    let mut app = super::Harness::mount(ResettableTaggedForm);
    app.fire("input", draft_input(&app), "we");
    app.click(app.labelled("click", "Reset"));
    expect_that!(app.html(), contains_substring(draft_value("tags.#0", "")));
}

/// `post()` has two items, so its draft is `tags.#2`.
fn post_with_draft(text: &str) -> FormState<Post> {
    let mut form = form_for(&post(), FormSpec::default());
    form.distribute_values(&ValuesByPath::from_iter([(
        NodePath::new("tags.#2"),
        text.to_string(),
    )]));
    form
}

#[gtest]
fn a_draft_is_neither_sent_nor_validated() {
    let mut form = post_with_draft("web");
    expect_that!(form.collect_values().get("tags.#2"), none());
    expect_that!(form.validate(), some(eq(&post())));
}

/// An `AddItem` with a value takes a key of its own, never the draft's, so
/// what was typed into the draft is still there to add.
#[gtest]
fn an_add_with_a_value_never_takes_the_drafts_key() {
    let mut form = post_with_draft("we");
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new("css".to_string())))),
        ok(some(eq("tags.#3")))
    );
    expect_that!(form.edit(&add_tag(None)), ok(some(eq("tags.#2"))));
    expect_that!(
        form.validate(),
        some(eq(&post_tagged(&["css", "dioxus", "rust", "we"])))
    );
}

/// A server's draft is built before the wire's keys arrive, so one of them
/// can be its key. It moves to a fresh one: here `#1`, whose value it then
/// takes, rather than staying at `#0` and reading item `#0`'s.
#[gtest]
fn a_drafts_key_moves_when_the_wire_takes_it() {
    let mut form = empty_form(FormSpec::<Tagged>::default());
    form.distribute_values(&ValuesByPath::from_iter(
        [("tags", "#0"), ("tags.#0", "x"), ("tags.#1", "y")]
            .map(|(path, raw)| (NodePath::new(path.to_string()), raw.to_string())),
    ));
    expect_that!(form.edit(&add_tag(None)), ok(some(eq("tags.#1"))));
    expect_that!(
        form.validate(),
        some(eq(&Tagged {
            tags: tags(&["x", "y"]),
        }))
    );
}

/// The items' spec reaches the draft, constraints and all.
#[gtest]
fn the_items_constraints_refuse_a_draft() {
    // 6: "dioxus" is an item already.
    let mut form = form_for(&post(), form! { Post { tags[] => { max_length: 6 } } });
    form.distribute_values(&ValuesByPath::from_iter([(
        NodePath::new("tags.#2"),
        "toolong".to_string(),
    )]));
    expect_that!(form.edit(&add_tag(None)), ok(none()));
    expect_that!(form.validate(), some(eq(&post())));
}

#[component]
fn EmptyTriageForm() -> Element {
    let form =
        use_form(|| empty_form(form! { Triage { levels => { sort_by: most_urgent_first } } }));
    form.render(using_fns! {})
}

/// A unit enum's draft is its own widget, a select over the variants, named
/// by its legend as any enum's select is.
#[gtest]
fn a_unit_enum_sets_draft_is_a_select() {
    let html = super::render_to_html(EmptyTriageForm);
    expect_that!(html, contains_substring("Add to Levels</legend>"));
    expect_that!(
        html,
        contains_substring(r#"<option value="High">High</option>"#)
    );
}

/// Choosing in the draft's select is an edit inside the draft, which the set
/// hands it; Add then makes the choice an item.
#[gtest]
fn a_unit_enum_draft_takes_a_choice_and_adds_it() {
    let mut form = empty_form(form! { Triage { levels => { sort_by: most_urgent_first } } });
    expect_that!(
        form.edit(&Edit::ChooseVariant {
            path: NodePath::new("levels.#0"),
            variant: Some("High".to_string()),
        }),
        ok(none())
    );
    expect_that!(
        form.edit(&Edit::AddItem {
            path: NodePath::new("levels"),
            before: None,
            value: None,
        }),
        ok(some(eq("levels.#0")))
    );
    expect_that!(
        form.validate(),
        some(eq(&Triage {
            levels: [Priority::High].into(),
        }))
    );
}

#[component]
fn EmptyTriageWireForm() -> Element {
    let form =
        use_form(|| empty_form(form! { Triage { levels => { sort_by: most_urgent_first } } }));
    with_wire(&form, rsx! {})
}

/// The same through `use_form`: the choice stays in the state, where no
/// sync from the store can unchoose it before Add reads it.
#[gtest]
fn a_unit_enum_draft_chosen_in_its_select_is_added() {
    let mut app = super::Harness::mount(EmptyTriageWireForm);
    app.fire("change", app.only_listener("change"), "High");
    app.click(add_button(&app));
    let html = app.html();
    expect_that!(html, contains_substring(wire("levels=#0 levels.#0=High")));
    expect_that!(
        html,
        contains_substring(done("“High” was added successfully."))
    );
}

// ── Editing an item through the draft (box 8) ────────────────────────────
//
// Edit marks an item and puts its value into the draft; Replace (Add, while
// editing) replaces the item with what the draft holds; Cancel, Escape or a
// second press of Edit stops, leaving the item as it was.

fn tagged() -> Tagged {
    Tagged {
        tags: tags(&["dioxus", "rust"]),
    }
}

#[component]
fn TaggedWireForm() -> Element {
    let form = use_form(|| form_for(&tagged(), FormSpec::default()));
    with_wire(&form, rsx! {})
}

/// Mounted with `dioxus` as item `#0`, `rust` `#1`, and the draft `#2`.
/// Items are shown, not edited in place, and `Tagged` has no other field, so
/// the draft is the only input.
fn mounted_tags() -> super::Harness {
    super::Harness::mount(TaggedWireForm)
}

/// Replace, while `rust` is being edited: Add, named for the item it would
/// replace.
fn replace(app: &super::Harness) -> dioxus::core::ElementId {
    app.labelled("click", "Replace “rust”")
}

fn edit_rust(app: &mut super::Harness) {
    app.click(app.labelled("click", "Edit “rust”"));
}

const POST_WIRE: &str = "tags=#0,#1 tags.#0=dioxus tags.#1=rust";

/// Shown as its text, with Edit and Remove, not as an input of its own
/// (decision E): nothing moves while the user types.
#[gtest]
fn an_item_is_shown_with_edit_and_remove() {
    let html = super::render_to_html(PostForm);
    expect_that!(html, not(contains_substring(r#"name="tags.#1""#)));
    expect_that!(
        html,
        contains_substring(r#"aria-label="Edit “rust”" aria-pressed="false""#)
    );
    expect_that!(html, contains_substring("fx-button fx-button-edit-item"));
}

#[gtest]
fn edit_puts_the_item_into_the_draft() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    let html = app.html();
    expect_that!(html, contains_substring(draft_value("tags.#2", "rust")));
    expect_that!(
        html,
        contains_substring(r#"aria-label="Edit “rust”" aria-pressed="true""#)
    );
    expect_that!(html, contains_substring(r#"aria-label="Replace “rust”""#));
    expect_that!(html, contains_substring("Cancel</button>"));
    expect_that!(
        html,
        contains_substring(wire(POST_WIRE)),
        "nothing changed yet"
    );
}

/// Replace replaces the item, in one edit: the set never holds both.
#[gtest]
fn replace_swaps_in_the_draft_for_the_item_being_edited() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    app.fire("input", app.only_listener("input"), "go");
    app.click(replace(&app));
    let html = app.html();
    expect_that!(
        html,
        contains_substring(wire("tags=#0,#2 tags.#0=dioxus tags.#2=go"))
    );
    expect_that!(
        html,
        contains_substring(done("“rust” was changed to “go” successfully."))
    );
    expect_that!(html, contains_substring(draft_value("tags.#3", "")));
    expect_that!(html, contains_substring("Add</button>"));
}

/// Replaced by itself unchanged, it is not its own duplicate, and there is
/// nothing to say.
#[gtest]
fn replacing_an_item_with_itself_ends_the_edit_quietly() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    app.click(replace(&app));
    let html = app.html();
    expect_that!(
        html,
        contains_substring(wire("tags=#0,#2 tags.#0=dioxus tags.#2=rust"))
    );
    expect_that!(html, contains_substring(region("fx-set-message", "")));
    expect_that!(html, contains_substring("Add</button>"));
}

/// Equal to ANOTHER item is a duplicate as ever, and the edit goes on.
#[gtest]
fn replacing_with_another_items_value_is_refused() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    app.fire("input", app.only_listener("input"), "dioxus");
    app.click(replace(&app));
    let html = app.html();
    expect_that!(html, contains_substring(wire(POST_WIRE)));
    expect_that!(
        html,
        contains_substring(refused("“dioxus” is already there."))
    );
    expect_that!(html, contains_substring(r#"aria-label="Replace “rust”""#));
}

#[gtest]
fn cancel_leaves_the_item_as_it_was() {
    let mut app = mounted_tags();
    let before = app.listeners("click");
    edit_rust(&mut app);
    app.fire("input", app.only_listener("input"), "go");
    let cancel = super::new_since(&before, app.unlabelled("click"));
    app.click(cancel[0]);
    let html = app.html();
    expect_that!(html, contains_substring(wire(POST_WIRE)));
    expect_that!(html, contains_substring(draft_value("tags.#2", "")));
    expect_that!(
        html,
        contains_substring(r#"aria-label="Edit “rust”" aria-pressed="false""#)
    );
}

#[gtest]
fn escape_in_the_draft_cancels_an_edit() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    app.fire("input", app.only_listener("input"), "go");
    app.press(app.only_listener("keydown"), Key::Escape);
    let html = app.html();
    expect_that!(html, contains_substring(wire(POST_WIRE)));
    expect_that!(html, contains_substring(draft_value("tags.#2", "")));
}

/// Not editing, Escape is not formoxus's: what was typed stays.
#[gtest]
fn escape_without_an_edit_leaves_the_draft_alone() {
    let mut app = mounted_tags();
    app.fire("input", app.only_listener("input"), "we");
    app.press(app.only_listener("keydown"), Key::Escape);
    expect_that!(app.html(), contains_substring(draft_value("tags.#2", "we")));
}

/// Edit is a toggle: pressed again, it stops the edit as Cancel does.
#[gtest]
fn pressing_edit_again_cancels() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    edit_rust(&mut app);
    let html = app.html();
    expect_that!(html, contains_substring(draft_value("tags.#2", "")));
    expect_that!(html, contains_substring("Add</button>"));
}

/// Editing another item moves the edit there; what the draft held goes.
#[gtest]
fn editing_another_item_moves_the_edit() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    app.click(app.labelled("click", "Edit “dioxus”"));
    let html = app.html();
    expect_that!(html, contains_substring(draft_value("tags.#2", "dioxus")));
    expect_that!(
        html,
        contains_substring(r#"aria-label="Edit “rust”" aria-pressed="false""#)
    );
}

/// The item stays in the set until Replace, so a submit while editing sends it
/// and loses nothing; what the draft holds is not sent (box 7).
#[gtest]
fn an_item_being_edited_is_still_in_the_set() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&Edit::EditItem {
            path: NodePath::new("tags"),
            index: Some(1),
        }),
        ok(some(eq("tags.#2")))
    );
    expect_that!(form.validate(), some(eq(&post())));
}

/// Removing the item being edited ends the edit, and the draft keeps its
/// text: Add would put it back, as a new item.
#[gtest]
fn removing_the_item_being_edited_ends_the_edit() {
    let mut form = form_for(&post(), FormSpec::default());
    form.edit(&Edit::EditItem {
        path: NodePath::new("tags"),
        index: Some(1),
    })
    .expect("item 1 exists");
    form.edit(&Edit::RemoveItem {
        path: NodePath::new("tags"),
        index: 1,
    })
    .expect("item 1 exists");
    expect_that!(form.edit(&add_tag(None)), ok(some(eq("tags.#2"))));
    expect_that!(form.validate(), some(eq(&post())));
}

#[gtest]
fn editing_an_item_that_is_not_there_is_refused() {
    let mut form = form_for(&post(), FormSpec::default());
    expect_that!(
        form.edit(&Edit::EditItem {
            path: NodePath::new("tags"),
            index: Some(2),
        }),
        err(displays_as(contains_substring(
            "cannot edit item at index 2 of tags: it has 2 items"
        )))
    );
}

/// A unit enum's item moves into the draft as its chosen variant, a
/// structure value, and back out as an item.
#[gtest]
fn a_unit_enum_item_is_edited_through_its_draft() {
    let mut form = form_for(
        &Triage {
            levels: [Priority::Low].into(),
        },
        form! { Triage { levels => { sort_by: most_urgent_first } } },
    );
    form.edit(&Edit::EditItem {
        path: NodePath::new("levels"),
        index: Some(0),
    })
    .expect("item 0 exists");
    form.edit(&Edit::ChooseVariant {
        path: NodePath::new("levels.#1"),
        variant: Some("High".to_string()),
    })
    .expect("the draft takes a choice");
    expect_that!(
        form.edit(&Edit::AddItem {
            path: NodePath::new("levels"),
            before: None,
            value: None,
        }),
        ok(some(eq("levels.#1")))
    );
    expect_that!(
        form.validate(),
        some(eq(&Triage {
            levels: [Priority::High].into(),
        }))
    );
}

/// Edit fills a draft, so only beside the adder that shows one.
#[gtest]
fn a_custom_adder_gets_no_edit() {
    expect_that!(
        super::render_to_html(CustomAdderPostForm),
        not(contains_substring("fx-button-edit-item"))
    );
}

#[component]
fn ItemErrorForm() -> Element {
    let form = use_form(|| {
        let mut state = form_for(&post(), FormSpec::default());
        state
            .push_field_error(&path!(Post.tags[= "rust"]), "Not that one.")
            .expect("item 1 exists");
        state
    });
    form.render(using_fns! {})
}

/// An item's own error, from a dev's validator or a server, is shown with it,
/// since it has no input of its own to show one.
#[gtest]
fn an_items_own_error_is_shown_with_it() {
    expect_that!(
        super::render_to_html(ItemErrorForm),
        contains_substring(r#"<li class="fx-field-error">Not that one.</li>"#)
    );
}

fn changed(event: SetEvent<'_, String>) -> Option<String> {
    match event {
        SetEvent::Changed { from, to } => Some(format!("{from} became {to}")),
        _ => None,
    }
}

#[component]
fn ChangedFeedbackForm() -> Element {
    let form = use_form(|| {
        form_for(
            &tagged(),
            form! { Tagged { tags => { feedback: changed } } },
        )
    });
    form.render(using_fns! {})
}

/// A dev's `feedback:` hears of a change with both values.
#[gtest]
fn feedback_hears_of_a_change() {
    let mut app = super::Harness::mount(ChangedFeedbackForm);
    edit_rust(&mut app);
    app.fire("input", app.only_listener("input"), "go");
    app.click(replace(&app));
    expect_that!(app.html(), contains_substring(done("rust became go")));
}

#[gtest]
fn a_list_refuses_an_item_edit() {
    let mut form = form_for(&Draft { tags: tags(&["a"]) }, FormSpec::default());
    expect_that!(
        form.edit(&Edit::EditItem {
            path: NodePath::new("tags"),
            index: Some(0),
        }),
        err(displays_as(contains_substring(
            "tags is a list; its items are edited in place"
        )))
    );
}

/// What the user sees of it: the adder is back to Add, with no Cancel.
#[gtest]
fn removing_the_item_being_edited_restores_add() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    app.click(app.labelled("click", "Remove “rust”"));
    let html = app.html();
    expect_that!(html, contains_substring("Add</button>"));
    expect_that!(html, not(contains_substring("Cancel</button>")));
    expect_that!(html, contains_substring(draft_value("tags.#2", "rust")));
}

// ── Display (box 9) ──────────────────────────────────────────────────────

/// A set's Remove is named by its item, as Edit is: the set is sorted, so
/// "Remove item 2" names nothing a user can see. A list keeps its items'
/// positions (`vecs.rs`).
#[gtest]
fn a_sets_remove_is_named_by_its_item() {
    let html = super::render_to_html(PostForm);
    expect_that!(html, contains_substring(r#"aria-label="Remove “rust”""#));
    expect_that!(html, not(contains_substring("Remove item")));
}

#[component]
fn TextareaItemsForm() -> Element {
    let form = use_form(|| form_for(&post(), form! { Post { tags[] => { widget: textarea } } }));
    form.render(using_fns! {})
}

/// The items' own widget is the draft's: shown, items are text, so the
/// widget `tags[]` names is where an item is typed, new or being edited.
#[gtest]
fn the_items_widget_renders_the_draft() {
    let html = super::render_to_html(TextareaItemsForm);
    expect_that!(html, contains_substring(r#"name="tags.#2""#));
    expect_that!(html, contains_substring("<textarea"));
}

/// The draft and its buttons in one `fx-set-adder`, so a preset can lay
/// them out as a row (the gallery's `fx-tags`).
#[gtest]
fn the_draft_and_its_buttons_share_one_wrapper() {
    let html = super::render_to_html(PostForm);
    let start = html
        .find(r#"<div class="fx-set-adder"><div id=""#)
        .expect("the wrapper opens on the draft");
    let add = html.find("Add</button>").expect("the set has Add");
    let message = html.find("fx-set-message").expect("the set has its region");
    expect_that!(add, gt(start));
    expect_that!(
        html[add..message].to_string(),
        starts_with("Add</button></div>")
    );
}

#[component]
fn TagsPresetForm() -> Element {
    let form = use_form(|| {
        form_for(
            &post(),
            form! { Post { classes_plus: { set: + ["fx-tags"] } } },
        )
    });
    form.render(using_fns! {})
}

/// How the gallery's tags preset is turned on (its `main.css`): one class
/// on the set's fieldset, through `classes_plus:`.
#[gtest]
fn the_tags_preset_is_one_class_on_the_set() {
    expect_that!(
        super::render_to_html(TagsPresetForm),
        contains_substring(r#"<fieldset class="fx-group fx-set fx-tags""#)
    );
}

// ── A dev's item view: `item:` (box 9) ───────────────────────────────────

/// A view that shows everything it was handed.
#[component]
fn TagChip(props: formoxus::widgets::SetItemProps) -> Element {
    // Straight from the store, as a dev's view has to read it (issue #22).
    let stored = props
        .values
        .read()
        .get(&props.path)
        .cloned()
        .unwrap_or_default();
    let says = format!(
        "{} at {} #{} editing={} stored={stored}",
        props.text, props.path, props.index, props.editing
    );
    rsx! { span { class: "chip", "{says}" } }
}

#[component]
fn ChipForm() -> Element {
    let form = use_form(|| form_for(&tagged(), form! { Tagged { tags => { item: TagChip } } }));
    form.render(using_fns! {})
}

#[component]
fn HandBuiltChipForm() -> Element {
    let form = use_form(|| {
        form_for(
            &tagged(),
            FormSpec::default()
                .with_item_view(&path!(Tagged.tags), formoxus::widgets::ItemView(TagChip)),
        )
    });
    form.render(using_fns! {})
}

/// The view shows the item, and formoxus still renders the rest of the
/// `<li>`: Edit and Remove, named by the item, follow it.
#[gtest]
fn a_dev_item_view_shows_each_item() {
    for html in [
        super::render_to_html(ChipForm),
        super::render_to_html(HandBuiltChipForm),
    ] {
        expect_that!(
            html,
            contains_substring(
                r#"<li class="fx-set-item"><span class="chip">rust at tags.#1 #1 editing=false stored=rust</span><button"#
            )
        );
        expect_that!(html, contains_substring(r#"aria-label="Edit “rust”""#));
        expect_that!(html, contains_substring(r#"aria-label="Remove “rust”""#));
    }
}

#[component]
fn ChipWireForm() -> Element {
    let form = use_form(|| form_for(&tagged(), form! { Tagged { tags => { item: TagChip } } }));
    with_wire(&form, rsx! {})
}

/// Told when its item is the one being edited.
#[gtest]
fn a_dev_item_view_knows_when_its_item_is_being_edited() {
    let mut app = super::Harness::mount(ChipWireForm);
    edit_rust(&mut app);
    expect_that!(
        app.html(),
        contains_substring("rust at tags.#1 #1 editing=true")
    );
}

#[component]
fn ChipListForm() -> Element {
    let form = use_form(|| {
        form_for(
            &Draft {
                tags: tags(&["b", "a"]),
            },
            form! { Draft { tags => { set, item: TagChip } } },
        )
    });
    form.render(using_fns! {})
}

/// On a `Vec` treated as a set, as on a set: sorted before its keys are
/// given out, so `a` is `#0`.
#[gtest]
fn a_dev_item_view_shows_a_list_treated_as_a_set() {
    expect_that!(
        super::render_to_html(ChipListForm),
        contains_substring(r#"<span class="chip">a at tags.#0 #0"#)
    );
}

#[gtest]
#[should_panic(expected = "title: `item` applies only to a set, not `String`")]
fn an_item_view_on_a_scalar_is_refused() {
    let _ = FormSpec::<Post>::default()
        .with_item_view(&path!(Post.title), formoxus::widgets::ItemView(TagChip));
}

#[gtest]
#[should_panic(
    expected = "tags: if you want to treat this list as a set, call `treat_as_set` first"
)]
fn an_item_view_on_a_list_not_yet_treated_as_a_set_is_refused() {
    let _ = FormSpec::<Draft>::default()
        .with_item_view(&path!(Draft.tags), formoxus::widgets::ItemView(TagChip));
}

/// Named for the item it replaces, not for what the draft holds, so the name
/// stays true while the user types; and it shows the word that begins its
/// name (WCAG 2.5.3), so it never reads "Save" beside a form's own `save`.
#[gtest]
fn replace_is_named_for_the_item_it_replaces() {
    let mut app = mounted_tags();
    edit_rust(&mut app);
    app.fire("input", app.only_listener("input"), "go");
    let html = app.html();
    expect_that!(html, contains_substring(r#"aria-label="Replace “rust”""#));
    expect_that!(html, contains_substring("Replace</button>"));
    expect_that!(html, not(contains_substring("Save")));
}

// ── The draft and the browser's constraint checks (box 14) ───────────────

#[component]
fn PatternDraftForm() -> Element {
    let form = use_form(|| {
        empty_form(form! { Tagged { tags[] => { pattern: "[a-z]+", max_length: 6 } } })
    });
    with_wire(&form, rsx! {})
}

/// The draft's input is inside the `<form>`, so a constraint on it would let
/// the browser block a submit that formoxus allows (text left in the draft
/// is not in the set). So its field's constraints stay off it, all but
/// `maxlength`, which typing cannot break.
#[gtest]
fn the_draft_leaves_its_items_constraints_off_but_keeps_maxlength() {
    let html = super::render_to_html(PatternDraftForm);
    expect_that!(html, not(contains_substring("pattern=")));
    expect_that!(html, contains_substring("maxlength=6"));
}

/// Off the input, not off the item: Add checks the pattern, and the error
/// is the draft's own.
#[gtest]
fn add_still_checks_a_drafts_pattern() {
    let mut app = super::Harness::mount(PatternDraftForm);
    app.fire("input", draft_input(&app), "Rust");
    app.click(add_button(&app));
    let html = app.html();
    expect_that!(html, contains_substring(wire("tags=")));
    expect_that!(html, contains_substring(draft_value("tags.#0", "Rust")));
    expect_that!(html, contains_substring(r#"<li class="fx-field-error">"#));
}

#[component]
fn PatternItemsForm() -> Element {
    let form = use_form(|| {
        form_for(
            &tagged(),
            form! { Tagged { tags[] => { pattern: "[a-z]+" } } },
        )
    });
    form.render(using_fns! {})
}

/// Only the draft: a list's items, edited in place, keep theirs. Here a set's
/// items have no inputs at all, so the draft is the only one.
#[gtest]
fn a_lists_items_keep_their_constraints() {
    expect_that!(
        super::render_to_html(ListPatternForm),
        contains_substring(r#"pattern="[a-z]+""#)
    );
    expect_that!(
        super::render_to_html(PatternItemsForm),
        not(contains_substring("pattern="))
    );
}

#[component]
fn ListPatternForm() -> Element {
    let form = use_form(|| {
        form_for(
            &Draft { tags: tags(&["a"]) },
            form! { Draft { tags[] => { pattern: "[a-z]+" } } },
        )
    });
    form.render(using_fns! {})
}

// ── A set over choices shows their labels ────────────────────────────────

const CITIES: &[(&str, &str)] = &[("BOS", "Boston"), ("ORD", "Chicago")];

#[derive(Facet, Clone, Debug, PartialEq)]
struct Itinerary {
    cities: BTreeSet<String>,
}

fn itinerary(cities: &[&str]) -> Itinerary {
    Itinerary {
        cities: cities.iter().map(ToString::to_string).collect(),
    }
}

fn itinerary_spec() -> FormSpec<Itinerary> {
    form! { Itinerary { cities[] => { widget: select, choices: CITIES } } }
}

#[component]
fn ChosenCities() -> Element {
    let form = use_form(|| form_for(&itinerary(&["BOS", "ORD", "XXX"]), itinerary_spec()));
    form.render(using_fns! {})
}

/// The item reads as its draft's `<select>` offered it, the label, and is
/// named by it; the value is what the model and the wire carry.
#[gtest]
fn an_item_over_choices_shows_its_label() {
    let html = super::render_to_html(ChosenCities);
    expect_that!(
        html,
        all![
            contains_substring(r#"<li class="fx-set-item">Boston<"#),
            contains_substring(r#"<li class="fx-set-item">Chicago<"#),
            contains_substring(r#"aria-label="Edit “Boston”""#),
            contains_substring(r#"aria-label="Remove “Chicago”""#),
            not(contains_substring(r#"<li class="fx-set-item">BOS<"#)),
        ]
    );
}

/// A value that is no longer a choice has no label, so the user sees the
/// value, beside the field's error once validated.
#[gtest]
fn an_item_that_is_no_longer_a_choice_shows_its_value() {
    let html = super::render_to_html(ChosenCities);
    expect_that!(html, contains_substring(r#"<li class="fx-set-item">XXX<"#));
}

#[component]
fn AddCityProbe() -> Element {
    let form = use_form(|| form_for(&itinerary(&["BOS"]), itinerary_spec()));
    use_hook(|| {
        form.edit(&Edit::AddItem {
            path: NodePath::new("cities"),
            before: None,
            value: Some(ItemValue::new("ORD".to_string())),
        })
        .expect("Chicago is a new city")
    });
    form.render(using_fns! {})
}

/// The set's messages name it by its label too.
#[gtest]
fn a_message_names_an_item_by_its_label() {
    expect_that!(
        super::render_to_html(AddCityProbe),
        contains_substring(done("“Chicago” was added successfully."))
    );
}

// ── A set of a unit enum names items as its select does ─────────────────

#[derive(Facet, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Region {
    NewEngland,
    PacificNorthwest,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trips {
    regions: BTreeSet<Region>,
}

#[component]
fn RegionsProbe() -> Element {
    let form = use_form(|| {
        form_for(
            &Trips {
                regions: [Region::NewEngland].into(),
            },
            form! { Trips { regions => { sort_by: Region::cmp } } },
        )
    });
    use_hook(|| {
        form.edit(&Edit::AddItem {
            path: NodePath::new("regions"),
            before: None,
            value: Some(ItemValue::new(Region::PacificNorthwest)),
        })
        .expect("a new region is added")
    });
    form.render(using_fns! {})
}

/// A variant reads as the draft's `<select>` offers it, label-cased, in the
/// item, its buttons and the message, not as its Rust name.
#[gtest]
fn a_unit_enum_item_is_shown_label_cased() {
    let html = super::render_to_html(RegionsProbe);
    expect_that!(
        html,
        all![
            contains_substring(r#"<li class="fx-set-item">New England<"#),
            contains_substring(r#"aria-label="Remove “Pacific Northwest”""#),
            contains_substring(done("“Pacific Northwest” was added successfully.")),
            // The select it was chosen in, for comparison.
            contains_substring(r#"<option value="NewEngland">New England</option>"#),
            not(contains_substring(">NewEngland<")),
        ]
    );
}

#[component]
fn LowerRegionsProbe() -> Element {
    let form = use_form(|| {
        form_for(
            &Trips {
                regions: [Region::NewEngland].into(),
            },
            form! { Trips { label_case: "label case", regions => { sort_by: Region::cmp } } },
        )
    });
    use_hook(|| {
        form.edit(&Edit::AddItem {
            path: NodePath::new("regions"),
            before: None,
            value: Some(ItemValue::new(Region::PacificNorthwest)),
        })
        .expect("a new region is added")
    });
    form.render(using_fns! {})
}

/// The case is the form's, applied when it renders: the message was made at
/// the edit, before the case was known, and still follows it.
#[gtest]
fn the_forms_label_case_reaches_items_and_messages() {
    let html = super::render_to_html(LowerRegionsProbe);
    expect_that!(
        html,
        all![
            contains_substring(r#"<li class="fx-set-item">new england<"#),
            contains_substring(done("“pacific northwest” was added successfully.")),
        ]
    );
}

// ── A dev's attributes reach the draft ───────────────────────────────────

#[component]
fn PlaceholderDraftForm() -> Element {
    let form = use_form(|| empty_form(form! { Tagged { tags[] => { placeholder: "Add a tag" } } }));
    form.render(using_fns! {})
}

/// The draft keeps only `maxlength` of its FIELD's own attributes (box 14),
/// but a dev's, like `placeholder`, are not constraints and all reach it:
/// "Add a tag" is the hint the draft most wants.
#[gtest]
fn a_placeholder_reaches_the_draft() {
    let html = super::render_to_html(PlaceholderDraftForm);
    expect_that!(
        html,
        contains_regex(r#"<input [^>]*name="tags\.#0"[^>]*placeholder="Add a tag""#)
    );
}

// ── The two set widgets, through a hand-built spec ───────────────────────
//
// `WIDGETS_PLAN.md` box 7, step 7i: `CheckboxMultiple` and `SelectMultiple`
// where `PropsAsText` stood above, through the hand-built spec they were
// written against before box 8. `form!`'s own tests are further down.

fn triage_with(widget: impl Into<formoxus::widgets::AnySetWidget>) -> FormSpec<Triage> {
    FormSpec::default().with_set_widget(&path!(Triage.levels), widget)
}

/// Low and High of three, at least one and at most two: the limit is reached,
/// so Medium is the one choice locked.
fn triage_at_its_limit(widget: impl Into<formoxus::widgets::AnySetWidget>) -> FormSpec<Triage> {
    triage_with(widget)
        .with_min_items(&path!(Triage.levels), 1)
        .with_max_items(&path!(Triage.levels), 2)
}

#[component]
fn CheckboxesForm() -> Element {
    let form = use_form(|| {
        form_for(
            &low_and_high(),
            triage_with(formoxus::widgets::CheckboxMultiple),
        )
    });
    form.render(using_fns! {})
}

#[component]
fn CheckboxesAtTheLimitForm() -> Element {
    let form = use_form(|| {
        form_for(
            &low_and_high(),
            triage_at_its_limit(formoxus::widgets::CheckboxMultiple),
        )
    });
    form.render(using_fns! {})
}

/// A fieldset of one box per choice, ticked for each chosen entry, in place of
/// `SetNode`'s own items and Add (decision H).
#[gtest]
fn checkboxes_tick_what_the_set_holds() {
    let html = super::without_form_id(&super::render_to_html(CheckboxesForm));
    expect_that!(
        html,
        contains_substring(concat!(
            r#"<fieldset class="fx-form-field fx-checkbox-multiple">"#,
            r#"<legend class="fx-legend fx-field-label">Levels</legend>"#,
            r#"<div class="fx-choice"><input class="fx-control fx-checkbox" type="checkbox" id="FORMID-levels-0" value="Low" checked=true/>"#,
            r#"<label class="fx-choice-label" for="FORMID-levels-0">Low</label></div>"#,
            r#"<div class="fx-choice"><input class="fx-control fx-checkbox" type="checkbox" id="FORMID-levels-1" value="Medium"/>"#,
            r#"<label class="fx-choice-label" for="FORMID-levels-1">Medium</label></div>"#,
            r#"<div class="fx-choice"><input class="fx-control fx-checkbox" type="checkbox" id="FORMID-levels-2" value="High" checked=true/>"#,
            r#"<label class="fx-choice-label" for="FORMID-levels-2">High</label></div>"#,
            r#"</fieldset>"#,
        ))
    );
    expect_that!(html, not(contains_substring("fx-button-add-item")));
}

/// What the group must NOT carry. No `name` (question Q): the set's path is
/// where the wire puts item keys. No `aria-invalid` or `role`: a bare fieldset
/// is role `group`, where ARIA leaves `aria-invalid` undefined. No `required`
/// on a box, even with `min_items`: on a checkbox it means THAT box must be
/// ticked, so `min_items` shows only as the legend's marker.
#[gtest]
fn checkboxes_carry_no_name_aria_invalid_role_or_required() {
    let html = super::render_to_html(CheckboxesAtTheLimitForm);
    expect_that!(html, not(contains_substring("name=")));
    expect_that!(html, not(contains_substring("aria-invalid")));
    expect_that!(html, not(contains_substring("role=")));
    expect_that!(html, not(contains_substring("required=")));
    expect_that!(
        html,
        contains_regex(
            r#"<legend [^>]*>Levels<span class="fx-required" aria-hidden="true"> \*</span></legend>"#
        )
    );
}

/// At `max_items` (question P) every unticked box is locked and every ticked
/// one stays live, so the user can untick one and pick another, as the Add
/// button stops a list.
#[gtest]
fn checkboxes_lock_what_is_unticked_at_max_items() {
    let html = super::render_to_html(CheckboxesAtTheLimitForm);
    expect_that!(html, contains_regex(r#"value="Medium" disabled=true/>"#));
    expect_that!(html, contains_regex(r#"value="Low" checked=true/>"#));
    expect_that!(html, contains_regex(r#"value="High" checked=true/>"#));
}

/// The change listeners the boxes registered, in choice order.
fn boxes(app: &super::Harness) -> Vec<dioxus::core::ElementId> {
    let found = app.listeners("change");
    assert_eq!(found.len(), 3, "one `change` listener per choice: {app:?}");
    found
}

/// Ticking a box adds its entry and unticking one removes it, through the
/// `AddItem` and `RemoveItem` a real event sends, and the next render ticks
/// what the set now holds.
#[gtest]
fn ticking_and_unticking_a_box_edits_the_set() {
    let mut app = super::Harness::mount(CheckboxesForm);
    let medium = boxes(&app)[1];
    app.fire("change", medium, "true");
    expect_that!(
        app.html(),
        contains_regex(r#"value="Medium" checked=true/>"#)
    );

    let low = boxes(&app)[0];
    app.fire("change", low, "false");
    let html = app.html();
    expect_that!(html, contains_regex(r#"value="Low"/>"#));
    expect_that!(html, contains_regex(r#"value="Medium" checked=true/>"#));
    expect_that!(html, contains_regex(r#"value="High" checked=true/>"#));
}

/// Unticking one box at the limit unlocks the rest.
#[gtest]
fn unticking_at_max_items_unlocks_the_rest() {
    let mut app = super::Harness::mount(CheckboxesAtTheLimitForm);
    let high = boxes(&app)[2];
    app.fire("change", high, "false");
    expect_that!(app.html(), not(contains_substring("disabled")));
}

#[component]
fn SelectForm() -> Element {
    let form = use_form(|| {
        form_for(
            &low_and_high(),
            triage_with(formoxus::widgets::SelectMultiple),
        )
    });
    form.render(using_fns! {})
}

#[component]
fn SelectAtTheLimitForm() -> Element {
    let form = use_form(|| {
        form_for(
            &low_and_high(),
            triage_at_its_limit(formoxus::widgets::SelectMultiple),
        )
    });
    form.render(using_fns! {})
}

/// One labelled control, laid out as `Select` is (question R): a `<label for>`
/// names it, and each chosen entry is a selected option. No `name` (Q).
#[gtest]
fn a_multi_select_selects_what_the_set_holds() {
    let html = super::without_form_id(&super::render_to_html(SelectForm));
    expect_that!(
        html,
        contains_substring(concat!(
            r#"<div class="fx-form-field">"#,
            r#"<label class="fx-field-label" for="FORMID-levels">Levels</label>"#,
            r#"<select class="fx-control fx-select" id="FORMID-levels" multiple=true>"#,
            r#"<option value="Low" selected=true>Low</option>"#,
            r#"<option value="Medium">Medium</option>"#,
            r#"<option value="High" selected=true>High</option>"#,
            r#"</select></div>"#,
        ))
    );
    expect_that!(html, not(contains_substring("name=")));
}

/// Unlike a checkbox, a `<select multiple>` takes `required` to mean "at
/// least one selected", which is `min_items >= 1`; at `max_items` the
/// unselected options are locked, as the boxes are.
#[gtest]
fn a_multi_select_is_required_and_locks_at_its_limits() {
    let html = super::render_to_html(SelectAtTheLimitForm);
    expect_that!(
        html,
        contains_regex(r"<select [^>]*multiple=true required=true>")
    );
    expect_that!(
        html,
        contains_substring(r#"<option value="Medium" disabled=true>"#)
    );
    expect_that!(
        html,
        contains_substring(r#"<option value="Low" selected=true>"#)
    );
}

/// One change can move several options at once. Low and High out, Medium
/// in, as one event: two `RemoveItem`s, highest index first, then an
/// `AddItem`, and the set ends holding only Medium.
#[gtest]
fn one_change_to_a_multi_select_can_swap_several_options() {
    let mut app = super::Harness::mount(SelectForm);
    let select = app.only_listener("change");
    app.select(select, &["Medium"]);
    let html = app.html();
    expect_that!(html, contains_substring(r#"<option value="Low">"#));
    expect_that!(
        html,
        contains_substring(r#"<option value="Medium" selected=true>"#)
    );
    expect_that!(html, contains_substring(r#"<option value="High">"#));
}

// Step 7h: an edit made through a set widget re-checks the set's own count
// at once, as `apply_edit` does for every Add and Remove (`check_item_count`
// in `form.rs`): a tick is a finished answer, a write and a blur together.

#[component]
fn CheckboxesCheckedEmptyForm() -> Element {
    let form = use_form(|| {
        form_for(
            &Triage {
                levels: BTreeSet::new(),
            },
            triage_with(formoxus::widgets::CheckboxMultiple)
                .with_min_items(&path!(Triage.levels), 1),
        )
    });
    use_hook(|| {
        let _ = form.validate();
    });
    form.render(using_fns! {})
}

/// A submit says the set needs a pick; ticking one meets `min_items`, and
/// the error goes then, not at the next submit.
#[gtest]
fn a_ticked_box_clears_the_sets_min_items_error() {
    let mut app = super::Harness::mount(CheckboxesCheckedEmptyForm);
    expect_that!(app.html(), contains_substring("Add at least 1 item."));
    let low = boxes(&app)[0];
    app.fire("change", low, "true");
    let html = app.html();
    expect_that!(html, contains_regex(r#"value="Low" checked=true/>"#));
    expect_that!(html, not(contains_substring("Add at least")));
}

#[component]
fn CheckboxesOneTickedForm() -> Element {
    let form = use_form(|| {
        form_for(
            &Triage {
                levels: [Priority::Low].into(),
            },
            triage_with(formoxus::widgets::CheckboxMultiple)
                .with_min_items(&path!(Triage.levels), 1),
        )
    });
    form.render(using_fns! {})
}

/// Unticking the last box under `min_items: 1` says so at once, as leaving
/// a required text field empty does on blur.
#[gtest]
fn unticking_the_last_box_says_so_at_once() {
    let mut app = super::Harness::mount(CheckboxesOneTickedForm);
    let low = boxes(&app)[0];
    app.fire("change", low, "false");
    let html = app.html();
    expect_that!(html, contains_regex(r#"value="Low"/>"#));
    expect_that!(html, contains_substring("Add at least 1 item."));
}

#[component]
fn SelectOnePickedForm() -> Element {
    let form = use_form(|| {
        form_for(
            &Triage {
                levels: [Priority::Low].into(),
            },
            triage_with(formoxus::widgets::SelectMultiple).with_min_items(&path!(Triage.levels), 1),
        )
    });
    form.render(using_fns! {})
}

/// The same through the select, where the set's result also reaches the
/// control as `aria-invalid`, which a listbox takes.
#[gtest]
fn deselecting_the_last_option_marks_the_select_invalid_at_once() {
    let mut app = super::Harness::mount(SelectOnePickedForm);
    let select = app.only_listener("change");
    app.select(select, &[]);
    let html = app.html();
    expect_that!(html, contains_regex(r#"<select [^>]*aria-invalid="true""#));
    expect_that!(html, contains_substring("Add at least 1 item."));
}

// ── A set of at most one is not a set ────────────────────────────────────
//
// `form!` refuses a literal `max_items: 1` on a set at compile time (the
// `form_set_of_*` goldens); these are the build's backstop, for a hand-built
// spec or a limit that is not a literal.

const ONE: usize = 1;

#[gtest]
#[should_panic(
    expected = "levels: a set of exactly one `Priority` is one value, not a set: make the field \
                a `Priority`, which a `select` or `radio_group` picks"
)]
fn a_set_of_exactly_one_is_refused() {
    let _ = form_for(
        &low_and_high(),
        triage_with(formoxus::widgets::SelectMultiple)
            .with_min_items(&path!(Triage.levels), ONE)
            .with_max_items(&path!(Triage.levels), ONE),
    );
}

#[gtest]
#[should_panic(
    expected = "levels: a set of at most one `Priority` is an optional value, not a set: make \
                the field an `Option<Priority>`, which a `select` or `radio_group` picks"
)]
fn a_set_of_at_most_one_is_refused() {
    let _ = form_for(
        &low_and_high(),
        FormSpec::default().with_max_items(&path!(Triage.levels), ONE),
    );
}

/// Through `form!` with a limit that is not a literal: no compile-time
/// check can read it, so the build refuses it instead.
#[gtest]
#[should_panic(expected = "levels: a set of at most one `Priority` is an optional value")]
fn form_leaves_a_non_literal_max_items_to_the_build() {
    let _ = form_for(
        &low_and_high(),
        form! { Triage { levels => { max_items: ONE } } },
    );
}

/// Two is a real "pick exactly two", so only one is refused.
#[gtest]
fn a_set_of_exactly_two_is_allowed() {
    let pair = low_and_high();
    expect_that!(
        form_for(
            &pair,
            form! { Triage { levels => { min_items: 2, max_items: 2 } } }
        )
        .validate(),
        some(eq(&pair))
    );
}

// ── `form!` names the set widgets (box 8) ────────────────────────────────

/// `form!`'s `widget: checkbox_multiple` builds exactly what the hand-built
/// spec above does.
#[component]
fn CheckboxesThroughFormMacro() -> Element {
    let form = use_form(|| {
        form_for(
            &low_and_high(),
            form! { Triage { levels => { widget: checkbox_multiple } } },
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn form_names_checkbox_multiple() {
    expect_that!(
        super::without_form_id(&super::render_to_html(CheckboxesThroughFormMacro)),
        eq(&super::without_form_id(&super::render_to_html(
            CheckboxesForm
        )))
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    cities: BTreeSet<String>,
}

const EUROPE: &[(&str, &str)] = &[("paris", "Paris"), ("rome", "Rome"), ("oslo", "Oslo")];

fn paris_and_oslo() -> Trip {
    Trip {
        cities: ["paris".to_string(), "oslo".to_string()].into(),
    }
}

/// Decision D: `choices:` beside the widget, on the set's own line, are the
/// items' choices, offered in the order written and shown by their labels.
#[component]
fn CitiesSelect() -> Element {
    let form = use_form(|| {
        form_for(
            &paris_and_oslo(),
            form! { Trip { cities => { widget: select_multiple, choices: EUROPE } } },
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_set_widgets_choices_go_beside_it() {
    expect_that!(
        super::render_to_html(CitiesSelect),
        contains_substring(concat!(
            r#"<option value="paris" selected=true>Paris</option>"#,
            r#"<option value="rome">Rome</option>"#,
            r#"<option value="oslo" selected=true>Oslo</option>"#,
        ))
    );
}

#[gtest]
fn a_set_widget_with_choices_round_trips() {
    let trip = paris_and_oslo();
    expect_that!(
        form_for(
            &trip,
            form! { Trip { cities => { widget: checkbox_multiple, choices: EUROPE } } }
        )
        .validate(),
        some(eq(&trip))
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Route {
    stops: Vec<String>,
}

/// A `Vec` takes a set widget with `set` beside it, and keeps its order of
/// first appearance no more than a set does: the set's sort decides.
#[component]
fn StopsCheckboxes() -> Element {
    let form = use_form(|| {
        form_for(
            &Route {
                stops: vec!["rome".to_string()],
            },
            form! { Route { stops => { set, widget: checkbox_multiple, choices: EUROPE } } },
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_list_marked_set_takes_a_set_widget() {
    let html = super::render_to_html(StopsCheckboxes);
    expect_that!(html, contains_regex(r#"value="rome" checked=true/>"#));
    expect_that!(html, contains_regex(r#"value="paris"/>"#));
}

/// A dev's own `SetWidget`, named by path: the field's shape sends it to
/// `with_set_widget`, and its `choices:` to the items.
#[component]
fn CitiesAsText() -> Element {
    let form = use_form(|| {
        form_for(
            &paris_and_oslo(),
            form! { Trip { cities => { widget: PropsAsText, choices: EUROPE } } },
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn form_sends_a_devs_set_widget_and_its_choices_to_the_set() {
    expect_that!(
        super::render_to_html(CitiesAsText),
        contains_substring(
            "choices [paris=Paris,rome=Rome,oslo=Oslo] chosen [oslo,paris] required false label [Cities]"
        )
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct MaybeTriage {
    levels: Option<BTreeSet<Priority>>,
}

/// An optional set takes a set widget too (decision E), written with its
/// `?` as every `Option` is in a path.
#[component]
fn OptionalCheckboxes() -> Element {
    let form = use_form(|| {
        form_for(
            &MaybeTriage {
                levels: Some([Priority::Medium].into()),
            },
            form! { MaybeTriage { levels? => { widget: checkbox_multiple } } },
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn an_optional_set_takes_a_set_widget() {
    expect_that!(
        super::render_to_html(OptionalCheckboxes),
        contains_regex(r#"value="Medium" checked=true/>"#)
    );
}

// ── Box 9: what is left after 7i and box 8 ───────────────────────────────

/// Trap 8: an optional set with nothing ticked is `None`, not `Some` of an
/// empty set, because an empty collection is not present; a plain set with
/// nothing ticked is empty. Easy to get backwards, so both.
#[gtest]
fn unticking_an_optional_sets_last_box_makes_it_none() {
    let mut form = form_for(
        &MaybeTriage {
            levels: Some([Priority::Medium].into()),
        },
        form! { MaybeTriage { levels? => { widget: checkbox_multiple } } },
    );
    form.edit(&Edit::RemoveItem {
        path: NodePath::new("levels"),
        index: 0,
    })
    .expect("Medium is item 0");
    expect_that!(form.validate(), some(eq(&MaybeTriage { levels: None })));
}

#[gtest]
fn unticking_a_plain_sets_last_box_leaves_it_empty() {
    let mut form = form_for(
        &Triage {
            levels: [Priority::Medium].into(),
        },
        form! { Triage { levels => { widget: checkbox_multiple } } },
    );
    form.edit(&Edit::RemoveItem {
        path: NodePath::new("levels"),
        index: 0,
    })
    .expect("Medium is item 0");
    expect_that!(
        form.validate(),
        some(eq(&Triage {
            levels: BTreeSet::new()
        }))
    );
}

fn trip_with_checkboxes() -> FormSpec<Trip> {
    form! { Trip { cities => { widget: checkbox_multiple, choices: EUROPE, min_items: 1 } } }
}

/// The widget changes how the set renders, never what crosses: the wire is
/// any set's (`cities` = item keys, `cities.#0` = an entry), and the server
/// rebuilds the model from it with the same spec.
#[gtest]
fn a_set_widgets_form_crosses_the_wire() {
    let trip = paris_and_oslo();
    let wire = formoxus::WireForm::from_model(&trip, trip_with_checkboxes());
    expect_that!(
        formoxus::Submission::accept(trip_with_checkboxes(), wire.values())
            .map(formoxus::Submission::into_model),
        ok(eq(&trip))
    );
}

/// And the server holds `min_items` against a forged wire with no items.
#[gtest]
fn the_server_holds_a_set_widgets_min_items() {
    let empty = Trip {
        cities: BTreeSet::new(),
    };
    let wire = formoxus::WireForm::from_model(&empty, FormSpec::default());
    expect_that!(
        formoxus::Submission::accept(trip_with_checkboxes(), wire.values()).is_err(),
        eq(true)
    );
}

/// `classes_plus:` reaches each widget's parts by their `Tag` names, as
/// for any built-in.
#[component]
fn ClassedSetWidgets() -> Element {
    let form = use_form(|| {
        form_for(
            &low_and_high(),
            form! {
                Triage {
                    levels => { widget: checkbox_multiple },
                    classes_plus: {
                        checkbox_multiple.fieldset: + [picks],
                        checkbox_multiple.input: [tick],
                    }
                }
            },
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn classes_plus_reaches_checkbox_multiples_parts() {
    let html = super::render_to_html(ClassedSetWidgets);
    expect_that!(
        html,
        contains_substring(r#"<fieldset class="fx-form-field fx-checkbox-multiple picks">"#)
    );
    expect_that!(
        html,
        contains_substring(r#"<input class="tick" type="checkbox""#)
    );
}

#[component]
fn ClassedMultiSelect() -> Element {
    let form = use_form(|| {
        form_for(
            &paris_and_oslo(),
            form! {
                Trip {
                    cities => { widget: select_multiple, choices: EUROPE },
                    classes_plus: { select_multiple.select: + [many] },
                }
            },
        )
    });
    form.render(using_fns! {})
}

#[gtest]
fn classes_plus_reaches_select_multiples_parts() {
    expect_that!(
        super::render_to_html(ClassedMultiSelect),
        contains_substring(r#"<select class="fx-control fx-select many""#)
    );
}
