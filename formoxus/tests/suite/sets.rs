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
    FormSpec::default().with_sort_by("tags", SetOrder::custom(by_length, "by_length"))
}

fn shelf(names: &[&str]) -> Shelf {
    Shelf {
        tags: names.iter().map(|name| Tag(name.to_string())).collect(),
    }
}

fn values(pairs: &[(&str, &str)]) -> ValuesByPath {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
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
        .filter(|(path, _)| path.starts_with("tags.#"))
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

/// An added item takes the next key, `#3`, and then its place in the order:
/// a key names an item, not a position, so sorting moves items and leaves
/// every key with the item it named.
#[gtest]
fn a_set_with_sort_by_puts_an_added_item_in_order() {
    let mut form = form_for(&shelf(&["dioxus", "go", "rust"]), shelf_spec());
    expect_that!(
        form.edit(&Edit::AddItem {
            path: "tags".to_string(),
            before: None,
            value: Some(ItemValue::new(Tag("css".to_string()))),
        }),
        ok(some(eq("tags.#3")))
    );
    expect_that!(
        tag_values(&form),
        elements_are![eq("go"), eq("css"), eq("rust"), eq("dioxus")]
    );
    expect_that!(item_keys(&form, "tags"), eq("#0,#3,#1,#2"));
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
        FormSpec::default().with_sort_by("pins", SetOrder::custom(by_rank, "by_rank")),
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
        .filter(|(path, _)| path.starts_with("tags.#"))
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
        .filter(|(path, _)| path.starts_with("tags.#"))
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

/// A unit enum has no default order (4a): its item is a variant node, which
/// holds no value to compare. With `sort_by` it sorts at build and on the
/// server's rebuild, and still round-trips.
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
            FormSpec::<Draft>::default().treat_as_set("tags")
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
        .filter(|(path, _)| path.starts_with("tags.#"))
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
    let mut form = form_for(&draft, FormSpec::default().treat_as_set("tags"));
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
            .treat_as_set("tags")
            .with_sort_by("tags", SetOrder::custom(by_length, "by_length")),
    );
    let values: Vec<String> = form
        .collect_values()
        .into_iter()
        .filter(|(path, _)| path.starts_with("tags.#"))
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
    let form = use_form(move || form_for(&draft, FormSpec::default().treat_as_set("tags")));
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
        expect_that!(html.matches("fx-button-remove-row").count(), eq(2));
        expect_that!(html, not(contains_substring("<ol")));
        expect_that!(html, not(contains_substring("fx-move-buttons")));
    }
}

fn draft_with(names: &[&str]) -> FormState<Draft> {
    form_for(
        &Draft { tags: tags(names) },
        FormSpec::default().treat_as_set("tags"),
    )
}

#[gtest]
fn a_list_treated_as_a_set_refuses_a_move_and_a_blank_item() {
    let mut form = draft_with(&["dioxus", "rust"]);
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
    expect_that!(
        form.edit(&add_tag(None)).map_err(|e| e.0),
        err(contains_substring(
            "tags is a set; you cannot add an empty item to a set"
        ))
    );
}

/// An added item takes the next key and its place in the order, and the
/// model's `Vec` follows the order, not the order of arrival.
#[gtest]
fn a_list_treated_as_a_set_puts_an_added_item_in_order() {
    let mut form = draft_with(&["dioxus", "rust"]);
    expect_that!(
        form.edit(&add_tag(Some(ItemValue::new("css".to_string())))),
        ok(some(eq("tags.#2")))
    );
    expect_that!(item_keys(&form, "tags"), eq("#2,#0,#1"));
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
    let mut form = empty_form::<Draft>(FormSpec::default().treat_as_set("tags"));
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
    let mut form = empty_form::<Draft>(FormSpec::default().treat_as_set("tags"));
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
    let _ = empty_form::<TagList>(FormSpec::default().treat_as_set("tags"));
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
        ok(some(eq("tags.#2")))
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
        path: "tags".to_string(),
        before: None,
        value: Some(ItemValue::new(Tag(name.to_string()))),
    };
    expect_that!(form.edit(&add("go")), ok(none()));
    expect_that!(form.edit(&add("js")), ok(some(eq("tags.#2"))));
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
            path: "tags".to_string(),
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
    let add = html.find("fx-button-add-row").expect("the set has Add");
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
            path: "tags".to_string(),
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
            path: "levels".to_string(),
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
            FormSpec::default().with_adder("tags", ItemAdder(TagAdder)),
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
            path: "tags".to_string(),
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
            path: "tags".to_string(),
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
                "tags",
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
    let _ = FormSpec::<Post>::default()
        .with_feedback("title", SetFeedback::new(|_: SetEvent<'_, String>| None));
}

#[gtest]
#[should_panic(
    expected = "tags: if you want to treat this list as a set, call `treat_as_set` first"
)]
fn feedback_on_a_list_not_yet_treated_as_a_set_is_refused() {
    let _ = FormSpec::<Draft>::default()
        .with_feedback("tags", SetFeedback::new(|_: SetEvent<'_, String>| None));
}
