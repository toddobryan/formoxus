//! Compile-checked paths into a model.
//!
//! A leaf path (`email`, `venue.city`, `answers[].text`) is the wire format, so
//! it has to be a string at the boundary. [`ModelPath`] is what keeps it from being a
//! string in *your* code: it wraps the same string, but the only way to obtain
//! one is [`path!`](macro@crate::path), which emits a witness borrow of the
//! field beside it.
//!
//! A path may name ONE item, for an error about it: of a `Vec` by index,
//! `path!(Quiz.answers[i])`, or of a set by value, `path!(Post.tags[= "rust"])`.
//! Either is a run-time value, so that path is built when it is written, and
//! the form turns it into the item's key (`answers.#5`, `tags.#2`) when the
//! error is placed.
//!
//! ```ignore
//! submission.reject_field(path!(Signup.email), "already registered")
//! ```
//!
//! Misspell `email` and rustc says `no field 'emial' on type '&Signup'`, with
//! the caret on the segment. The alternative — `reject_field("emial", …)` — is
//! a panic on a server, discovered by a user.

use facet::{Facet, Peek};
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Borrow;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::error::FormAccessError;
use crate::nodes::no_such_path;

/// One leaf path into `T`, proven to exist at compile time.
///
/// **`PhantomData<fn() -> T>`, not `PhantomData<T>`.** A `ModelPath` holds no `T`
/// and never will, and the `fn` form says so: it leaves `ModelPath<T>` `Send`,
/// `Sync` and `Copy` no matter what `T` is, where `PhantomData<T>` would make a
/// path inherit auto-traits from a model it doesn't contain.
///
/// **A `Cow`, so `Clone` but not `Copy`.** A path with an index is built at
/// run time (`answers[2]`) and owns its string; every other path borrows its
/// literal and costs nothing to clone.
pub struct ModelPath<T> {
    raw: Cow<'static, str>,
    model: PhantomData<fn() -> T>,
}

impl<T> ModelPath<T> {
    /// Called only by [`path!`](macro@crate::path).
    ///
    /// **Deliberately hidden, and that is the whole design.** The guarantee is
    /// not that a `ModelPath<T>` wraps a plausible string — it is that every
    /// `ModelPath<T>` came from an invocation rustc type-checked against `T`. A
    /// public constructor would let `ModelPath::new("emial")` back through the hole
    /// the type exists to close.
    #[doc(hidden)]
    pub const fn __new(raw: &'static str) -> Self {
        Self {
            raw: Cow::Borrowed(raw),
            model: PhantomData,
        }
    }

    /// Called only by [`path!`](macro@crate::path), for a path with an index
    /// (`answers[2]`), which it formats where the path is written. Hidden for
    /// the same reason as [`__new`](Self::__new).
    #[doc(hidden)]
    pub fn __new_indexed(raw: String) -> Self {
        Self {
            raw: Cow::Owned(raw),
            model: PhantomData,
        }
    }

    /// The spelling the form places an error by. Every path but one with an
    /// index is a key as it stands (`venue.city`); an index is spelled
    /// `answers[2]`, and the form resolves it to that item's key
    /// (`answers.#5`) when it places an error there. See
    /// [`FormState::push_field_error`].
    ///
    /// [`FormState::push_field_error`]: crate::FormState::push_field_error
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// `path` with each index and each value resolved to the key of the item it
    /// names, or `None` when one names no item.
    ///
    /// An index, `[n]`, is the `n`th item counted from 0 in the order the
    /// container's entry lists its keys: `groups[1].$Random.set[0]` is
    /// `groups.#3.$Random.set.#0` when `groups` is `"#0,#3"` and that group's set
    /// is `"#0,#1"`. A value, `[=text]` (encoded by [`item_value_key`]), is the
    /// item whose own entry is that text: `tags[=rust]` is `tags.#2` when
    /// `tags.#2` is `"rust"`. Each container's entry is read by the path
    /// resolved so far, so an index or value inside an item finds that item's
    /// container.
    ///
    /// `entry` gives an entry by its path, as `collect_values` writes them: a
    /// container's item keys, a leaf's text, a variant's name. [`crate::FormState`]
    /// reads its own, and [`crate::WireForm`] the ones it carries.
    ///
    /// `None` when an index is past the end of its list, a value is no item's,
    /// or either is applied to something with no item keys at all: an entry is
    /// item keys only if each is `#` and a number, so a leaf's text is never read
    /// as one. A path with neither is returned as it is, which is every path but
    /// one a validator wrote with `path!(…[i])` or `path!(…[= value])`.
    pub(crate) fn resolve<'k>(
        &self,
        entry: impl Fn(&NodePath) -> Option<&'k str>,
    ) -> Option<NodePath> {
        let path = self.as_str();
        // With no index or value, a model path is spelled as the live path it
        // names: neither marks an `Option`, and both spell a variant `$Name`.
        if !path.contains('[') {
            return Some(NodePath::new(path));
        }
        let mut resolved = NodePath::root();
        for segment in path.split('.') {
            let (name, mut selectors) =
                segment.split_at(segment.find('[').unwrap_or(segment.len()));
            resolved = resolved.join(name);
            while let Some(after) = selectors.strip_prefix('[') {
                let (selector, rest) = after.split_once(']')?;
                let keys = entry(&resolved)?;
                if !keys
                    .split(',')
                    .all(|key| key.is_empty() || is_item_key(key))
                {
                    return None;
                }
                let mut keys = keys.split(',').filter(|key| !key.is_empty());
                let key = match selector.strip_prefix('=') {
                    Some(encoded) => {
                        let text = decode(encoded)?;
                        keys.find(|key| entry(&resolved.join(key)) == Some(text.as_str()))?
                    }
                    None => keys.nth(selector.parse().ok()?)?,
                };
                resolved = resolved.join(key);
                selectors = rest;
            }
            if !selectors.is_empty() {
                return None;
            }
        }
        Some(resolved)
    }
}

/// Item keys are `#`-prefixed for the same reason variants are `$`-prefixed:
/// neither character can begin a Rust identifier, so a key can never be mistaken
/// for a field name.
pub(crate) const ITEM_SIGIL: char = '#';

/// An item key segment: `3` -> `#3`. A node's name, not a path: an item's name
/// is its key, and [`NodePath::join_item`] is how a path gets one.
///
/// An item's name is its *identity*, not its position, which is what lets an item
/// be inserted in the middle, removed, or reordered without renaming its
/// neighbours. Renaming them would move every leaf beneath them to a different
/// key in the value store, and the values would have to be shuffled to match:
/// the same silent-corruption hazard variant segments
/// ([`NodePath::join_variant`]) exist to prevent, except recurring on every
/// insert and remove rather than only on a variant switch.
///
/// The cost is that an item's *position* is no longer recoverable from its path.
/// Order lives in `ListNode::items` and nowhere else; see
/// [`NodePath::without_variants`].
pub(crate) fn item_segment(key: usize) -> String {
    format!("{ITEM_SIGIL}{key}")
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NodePath(String);

impl NodePath {
    /// Constructor for a path off the wire or in a test
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub(crate) fn root() -> Self {
        Self(String::new())
    }

    /// The wire spelling, as the `name` attribute holds it. For reading only:
    /// a path is built with `join`, never by editing this string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// `("", "title") -> "title"`, `("location", "street") -> "location.street"`.
    pub(crate) fn join(&self, name: &str) -> Self {
        debug_assert!(
            !name.contains('.'),
            "join cannot accept a name containing a dot"
        );
        if self.0.is_empty() {
            Self(name.to_string())
        } else {
            let prefix = &self.0;
            Self(format!("{prefix}.{name}"))
        }
    }

    /// `("answers", 3) -> "answers.#3"`. See [`item_segment`].
    pub(crate) fn join_item(&self, key: usize) -> Self {
        self.join(&item_segment(key))
    }

    /// `("footprint", "Circle") -> "footprint.$Circle"`.
    ///
    /// `$` cannot begin a Rust identifier, so a variant segment can never collide
    /// with a field name. That's what lets two variants of the same enum each have
    /// a `size` without both claiming `footprint.size`; see
    /// [`without_variants`](Self::without_variants).
    pub(crate) fn join_variant(&self, name: &str) -> Self {
        self.join(&format!("${name}"))
    }

    /// This path with its variant segments taken out, giving the path through
    /// the *model*: `footprint.$Circle.size` -> `footprint.size`.
    ///
    /// A live path mirrors the model exactly apart from these segments, which
    /// exist only to keep same-named fields in different variants apart. A
    /// segment beginning with `$` is never a field, and a variant segment is
    /// never the last segment of a leaf's path: it is only ever a prefix.
    ///
    /// An item key (`answers.#3.text`) is left alone: it *is* a real element of
    /// the model, just identified by identity rather than position. Which element
    /// it is cannot be read off the path (only `ListNode::items` knows that), so
    /// the result names the item without placing it.
    pub fn without_variants(&self) -> String {
        self.0
            .split('.')
            .filter(|segment| !segment.starts_with('$'))
            .collect::<Vec<_>>()
            .join(".")
    }

    /// The spec key that names the node at self.
    ///
    /// Items are keyed by identity (`venues.#3`), but a spec names every item at
    /// once (`venues[]`), so each `.#n` segment folds into a `[]` on the segment
    /// before it: `venues.#3.city` is `venues[].city`, and `answers.#0` is
    /// `answers[]`. Variant segments (`$Circle`) are spelled the same in both.
    pub(crate) fn spec_key(&self) -> String {
        let path = &self.0;
        let mut out = String::with_capacity(path.len());
        for segment in path.split('.') {
            if segment.starts_with(ITEM_SIGIL) {
                out.push_str("[]");
            } else {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(segment);
            }
        }
        out
    }

    /// Is self exactly `base` or does it start with `base`?
    ///
    /// The guard every container's `choose_variant` uses before recursing. Testing
    /// containment up front means at most ONE child is ever asked, so a child's real
    /// failure ("no such variant") propagates verbatim instead of being swallowed as
    /// "no such path" by a parent that was still shopping around.
    pub(crate) fn starts_with(&self, base: &NodePath) -> bool {
        base.0.is_empty()
            || self
                .0
                .strip_prefix(&base.0)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
    }

    /// This path with its `from` prefix swapped for `to`, comparing whole
    /// segments as `starts_with` does: `tags.#3.name` with `tags.#3` replaced
    /// by `tags.#9` (a set's draft, which is named by a key of its own) is
    /// `tags.#9.name`. `None` if the path does not
    /// start with `from`.
    pub(crate) fn replace_prefix(&self, from: &NodePath, to: &NodePath) -> Option<NodePath> {
        if !self.starts_with(from) {
            return None;
        }
        let rest = self.0[from.0.len()..].trim_start_matches('.');
        Some(match (to.0.is_empty(), rest.is_empty()) {
            (_, true) => to.clone(),
            (true, false) => Self(rest.to_string()),
            (false, false) => Self(format!("{}.{rest}", to.0)),
        })
    }

    /// Guard that this path is `base` or somewhere inside it, as
    /// [`starts_with`](Self::starts_with) says; the `Err` names this path.
    pub(crate) fn ensure_starts_with(&self, base: &NodePath) -> Result<(), FormAccessError> {
        if self.starts_with(base) {
            Ok(())
        } else {
            Err(no_such_path(self))
        }
    }
}

impl Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Test-only, so a test can write `eq("venues.#3.city")` against a `NodePath`.
/// Never compiled into a dev's build: a path compared to a bare string is what
/// `NodePath` exists to stop. See `__test-util` in `Cargo.toml`.
#[cfg(any(test, feature = "__test-util"))]
#[doc(hidden)]
impl PartialEq<str> for NodePath {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

/// As for `PartialEq<str>`, for a `&str` literal, which is what googletest's
/// `eq("…")` compares against.
#[cfg(any(test, feature = "__test-util"))]
#[doc(hidden)]
impl PartialEq<&str> for NodePath {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl Borrow<str> for NodePath {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// What [`path!`](macro@crate::path)'s witness reads a `[i]` segment's items
/// through (`Indexable::items`, never called at run time): only a `Vec` has
/// it, so `[i]` on anything else fails to compile, with this trait's message.
///
/// Only a `Vec`, because an index has to mean the same item to the validator
/// that computed it and to the form that places its error. A `Vec` is built in
/// the order its items are shown. A `HashSet` iterates in no fixed order, and
/// a `BTreeSet` by `Ord`, which a set's `sort_by` can disagree with.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`[i]` names one item of a `Vec`, and `{Self}` is not one",
    label = "not a `Vec`",
    note = "a set iterates in an order that need not be the order its items are shown in, \
            so an index into one could name the wrong item; put the error on the set itself"
)]
pub trait Indexable {
    #[doc(hidden)]
    type Item;
    #[doc(hidden)]
    fn items(&self) -> std::slice::Iter<'_, Self::Item>;
}

impl<T> Indexable for Vec<T> {
    type Item = T;
    fn items(&self) -> std::slice::Iter<'_, T> {
        self.iter()
    }
}

/// What [`path!`](macro@crate::path)'s witness checks a `[= value]` segment
/// with (`ItemOf::is_item`, never called at run time): only a set has it, and
/// only a reference to something its items borrow as (`&str` for a set of
/// `String`s) is a value of one.
///
/// Only a set, because only a set's items are unique by value, so a value
/// names one item; a `Vec` can hold it twice, and is named by index, `[i]`.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`[= value]` names one item of a set by its value, and `{Self}` is not a set",
    label = "not a set",
    note = "a `Vec` can hold a value twice, so its items are named by index: `[i]`"
)]
pub trait ItemOf {
    #[doc(hidden)]
    type Item;
    #[doc(hidden)]
    fn is_item<Q: ?Sized>(&self, _value: &Q)
    where
        Self::Item: Borrow<Q>,
    {
    }
}

impl<T> ItemOf for BTreeSet<T> {
    type Item = T;
}

impl<T, S> ItemOf for HashSet<T, S> {
    type Item = T;
}

// Every impl below is hand-written for one reason: `#[derive]` would add a
// `T: Clone`/`T: Debug`/… bound that is never satisfied by anything real here.
// A `ModelPath<T>` contains no `T`, so none of these depend on `T` at all — the same
// trap `crate::form::Form` documents for its own `Clone`.
impl<T> Clone for ModelPath<T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            model: PhantomData,
        }
    }
}

impl<T> Debug for ModelPath<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ModelPath({:?})", self.raw)
    }
}

impl<T> Display for ModelPath<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl<T> PartialEq for ModelPath<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for ModelPath<T> {}

impl<T> Hash for ModelPath<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T> AsRef<str> for ModelPath<T> {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

/// What [`path!`](macro@crate::path) writes for `[= value]`: the value's text
/// as its item's entry holds it, encoded so it cannot end the segment or the
/// path (`.`, `[`, `]`) or be misread (`%`). The text is the one a set's
/// item node writes for that value: a newtype's inner scalar, a unit enum's
/// variant name, any other leaf's display, the way `build::node_for_shape`
/// reads them. A set holds nothing but those.
#[doc(hidden)]
pub fn item_value_key<Q: for<'f> Facet<'f> + ?Sized>(value: &Q) -> String {
    let text = entry_text(Peek::new(value));
    let mut encoded = String::with_capacity(text.len() + 3);
    encoded.push_str("[=");
    for c in text.chars() {
        match c {
            '%' => encoded.push_str("%25"),
            '.' => encoded.push_str("%2E"),
            '[' => encoded.push_str("%5B"),
            ']' => encoded.push_str("%5D"),
            c => encoded.push(c),
        }
    }
    encoded.push(']');
    encoded
}

/// The entry an item node holding `value` writes for itself.
fn entry_text(value: Peek<'_, '_>) -> String {
    let shape = value.shape();
    if shape.scalar_type().is_none() {
        if crate::build::newtype_inner(shape).is_some()
            && let Some(inner) = value
                .into_struct()
                .ok()
                .and_then(|newtype| newtype.field(0).ok())
        {
            return inner.to_string();
        }
        if let Some(variant) = value
            .into_enum()
            .ok()
            .and_then(|e| e.variant_name_active().ok())
        {
            return variant.to_string();
        }
    }
    value.to_string()
}

/// The reverse of [`item_value_key`]'s escaping; `None` for a `%` not
/// followed by one of the four codes it writes.
fn decode(encoded: &str) -> Option<String> {
    let mut text = String::with_capacity(encoded.len());
    let mut rest = encoded;
    while let Some(at) = rest.find('%') {
        text.push_str(&rest[..at]);
        let code = rest.get(at + 1..at + 3)?;
        text.push(match code {
            "25" => '%',
            "2E" => '.',
            "5B" => '[',
            "5D" => ']',
            _ => return None,
        });
        rest = &rest[at + 3..];
    }
    text.push_str(rest);
    Some(text)
}

/// `#` and digits: an item's key, as `collect_values` names one.
fn is_item_key(segment: &str) -> bool {
    segment
        .strip_prefix('#')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    #[derive(Debug)]
    struct Model;
    /// Deliberately NOT `Clone`/`Eq`/`Hash`, to prove none of `ModelPath`'s impls
    /// leak a bound onto the model.
    #[derive(Debug)]
    struct Bare;

    #[gtest]
    fn a_path_is_clone_and_eq_for_a_model_that_is_neither() {
        let a: ModelPath<Bare> = ModelPath::__new("venue.city");
        let b = a.clone();
        expect_that!(a, eq(&b));
        expect_that!(a.as_str(), eq("venue.city"));
    }

    #[gtest]
    fn display_is_the_wire_spelling_and_debug_is_not() {
        let p: ModelPath<Model> = ModelPath::__new("answers[].text");
        expect_that!(format!("{p}"), eq("answers[].text"));
        expect_that!(format!("{p:?}"), eq("ModelPath(\"answers[].text\")"));
    }

    #[gtest]
    fn two_paths_with_the_same_spelling_are_the_same_path() {
        let a: ModelPath<Model> = ModelPath::__new("email");
        let b: ModelPath<Model> = ModelPath::__new("email");
        let c: ModelPath<Model> = ModelPath::__new("bio");
        expect_that!(a, eq(&b));
        expect_that!(a, not(eq(&c)));
    }

    // ── resolve_indices ──────────────────────────────────────────────────
    use std::collections::HashMap;

    fn keys(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(path, keys)| ((*path).to_string(), (*keys).to_string()))
            .collect()
    }

    fn resolve_indices(path: &str, keys: &HashMap<String, String>) -> Option<String> {
        ModelPath::<()>::__new_indexed(path.to_string())
            .resolve(|list| keys.get(list.as_str()).map(String::as_str))
            .map(|resolved| resolved.to_string())
    }

    #[gtest]
    fn a_path_without_an_index_is_unchanged() {
        expect_that!(
            resolve_indices("venue.city", &keys(&[])),
            some(eq("venue.city"))
        );
    }

    /// The index counts the keys in the order listed, which after a move or a
    /// removal is not the order the keys were made in.
    #[gtest]
    fn an_index_becomes_the_key_at_that_position() {
        let keys = keys(&[("answers", "#4,#0,#2")]);
        expect_that!(resolve_indices("answers[0]", &keys), some(eq("answers.#4")));
        expect_that!(resolve_indices("answers[2]", &keys), some(eq("answers.#2")));
    }

    /// Each list's keys are found by the path resolved so far: the set inside
    /// the SECOND group is the one under that group's key.
    #[gtest]
    fn a_nested_index_reads_the_list_inside_the_resolved_item() {
        let keys = keys(&[
            ("groups", "#0,#3"),
            ("groups.#0.$Random.set", "#0"),
            ("groups.#3.$Random.set", "#0,#7"),
        ]);
        expect_that!(
            resolve_indices("groups[1].$Random.set[1]", &keys),
            some(eq("groups.#3.$Random.set.#7"))
        );
    }

    /// `rows[1][0]`: a list of lists, both indices on one segment.
    #[gtest]
    fn two_indices_on_one_segment_resolve_in_turn() {
        let keys = keys(&[("rows", "#0,#1"), ("rows.#1", "#2,#5")]);
        expect_that!(resolve_indices("rows[1][0]", &keys), some(eq("rows.#1.#2")));
    }

    #[gtest]
    fn an_index_past_the_end_names_nothing() {
        let keys = keys(&[("answers", "#0,#1"), ("empty", "")]);
        expect_that!(resolve_indices("answers[2]", &keys), none());
        expect_that!(resolve_indices("empty[0]", &keys), none());
    }

    /// A leaf's entry is its text, which is never read as item keys.
    #[gtest]
    fn an_index_on_something_with_no_items_names_nothing() {
        expect_that!(resolve_indices("title[0]", &keys(&[])), none());
        let leaf = keys(&[("title", "Unit 1")]);
        expect_that!(resolve_indices("title[0]", &leaf), none());
    }

    /// Only `path!` writes an index, always as digits; anything else in the
    /// brackets is not one.
    #[gtest]
    fn a_malformed_index_names_nothing() {
        let keys = keys(&[("answers", "#0")]);
        expect_that!(resolve_indices("answers[x]", &keys), none());
        expect_that!(resolve_indices("answers[0", &keys), none());
        expect_that!(resolve_indices("answers[0]x", &keys), none());
    }

    // ── values: `[=text]` ────────────────────────────────────────────────

    fn tags() -> HashMap<String, String> {
        keys(&[
            ("tags", "#0,#2,#3"),
            ("tags.#0", "go"),
            ("tags.#2", "rust"),
            ("tags.#3", "v1.2 [beta] 100%"),
        ])
    }

    /// A value is the item whose own entry is that text, whatever its place.
    #[gtest]
    fn a_value_becomes_the_key_of_the_item_holding_it() {
        let key = super::item_value_key("rust");
        expect_that!(key, eq("[=rust]"));
        expect_that!(
            resolve_indices(&format!("tags{key}"), &tags()),
            some(eq("tags.#2"))
        );
    }

    /// The four characters that could end a segment or be misread are
    /// encoded, and decoded back before the match.
    #[gtest]
    fn a_value_with_a_dot_brackets_or_a_percent_still_resolves() {
        let key = super::item_value_key("v1.2 [beta] 100%");
        expect_that!(key, eq("[=v1%2E2 %5Bbeta%5D 100%25]"));
        expect_that!(
            resolve_indices(&format!("tags{key}"), &tags()),
            some(eq("tags.#3"))
        );
    }

    #[gtest]
    fn a_value_no_item_holds_names_nothing() {
        expect_that!(resolve_indices("tags[=java]", &tags()), none());
    }

    /// Only `item_value_key` writes a value, and only these four codes.
    #[gtest]
    fn a_malformed_encoding_names_nothing() {
        expect_that!(resolve_indices("tags[=r%7Ast]", &tags()), none());
        expect_that!(resolve_indices("tags[=rust%2]", &tags()), none());
    }

    /// An index finds the item, and a value then finds an item of the set
    /// inside it.
    #[gtest]
    fn an_index_then_a_value_resolve_in_turn() {
        let keys = keys(&[
            ("groups", "#0,#3"),
            ("groups.#3.$Random.set", "#0,#4"),
            ("groups.#3.$Random.set.#0", "q1"),
            ("groups.#3.$Random.set.#4", "q2"),
        ]);
        expect_that!(
            resolve_indices("groups[1].$Random.set[=q2]", &keys),
            some(eq("groups.#3.$Random.set.#4"))
        );
    }

    /// A value's text is what its item node writes: a newtype's inner value,
    /// a unit enum's variant name, a number's display.
    #[gtest]
    fn a_values_text_is_its_item_entry() {
        #[derive(facet::Facet)]
        struct Tag(String);
        #[derive(facet::Facet)]
        #[repr(u8)]
        #[allow(dead_code)]
        enum Day {
            Monday,
            Friday,
        }
        expect_that!(
            super::item_value_key(&Tag("rust".to_string())),
            eq("[=rust]")
        );
        expect_that!(super::item_value_key(&Day::Friday), eq("[=Friday]"));
        expect_that!(super::item_value_key(&7_u8), eq("[=7]"));
        expect_that!(super::item_value_key(&1.5_f64), eq("[=1%2E5]"));
    }

    // ── NodePath ─────────────────────────────────────────────────────────

    fn np(path: &str) -> NodePath {
        NodePath::new(path)
    }

    /// **The root is the empty path, and a join onto it adds no dot:** a
    /// top-level field's path is its bare name.
    #[gtest]
    fn a_join_onto_the_root_is_the_bare_name() {
        expect_that!(NodePath::root().as_str(), eq(""));
        expect_that!(NodePath::root().join("title"), eq(&np("title")));
    }

    #[gtest]
    fn a_join_adds_one_segment_after_a_dot() {
        expect_that!(np("location").join("street"), eq(&np("location.street")));
        expect_that!(np("venues.#3").join("city"), eq(&np("venues.#3.city")));
    }

    /// **An item's segment is `#` and its key, a variant's `$` and its name,**
    /// each written in one place.
    #[gtest]
    fn an_item_and_a_variant_carry_their_sigils() {
        expect_that!(np("answers").join_item(3), eq(&np("answers.#3")));
        expect_that!(
            np("footprint").join_variant("Circle"),
            eq(&np("footprint.$Circle"))
        );
        expect_that!(item_segment(12), eq("#12"));
    }

    #[gtest]
    fn display_is_the_wire_spelling() {
        expect_that!(format!("{}", np("venues.#3.city")), eq("venues.#3.city"));
    }

    // ── starts_with ──────────────────────────────────────────────────────

    #[gtest]
    fn a_path_starts_with_itself_and_each_ancestor() {
        let leaf = np("venues.#3.city");
        expect_that!(leaf.starts_with(&np("venues.#3.city")), eq(true));
        expect_that!(leaf.starts_with(&np("venues.#3")), eq(true));
        expect_that!(leaf.starts_with(&np("venues")), eq(true));
    }

    #[gtest]
    fn an_ancestor_does_not_start_with_its_descendant() {
        expect_that!(np("venues").starts_with(&np("venues.#3")), eq(false));
        expect_that!(np("title").starts_with(&np("subtitle")), eq(false));
    }

    /// **Only whole segments match,** as in `std::path::Path::starts_with`. A
    /// field whose name begins with another's is not inside it, and nor is an
    /// item whose key begins with another's: were `#12` inside `#1`, an edit or
    /// error for one would reach the other, and clearing `#1` would wipe `#12`.
    #[gtest]
    fn a_mere_string_prefix_is_not_a_prefix() {
        expect_that!(np("address2").starts_with(&np("address")), eq(false));
        expect_that!(np("address2.city").starts_with(&np("address")), eq(false));
        expect_that!(np("answers.#12").starts_with(&np("answers.#1")), eq(false));
        expect_that!(
            np("answers.#12.text").starts_with(&np("answers.#1")),
            eq(false)
        );
    }

    /// **Every path starts with the root, the root included,** as every
    /// `std::path::Path` starts with the empty path (decision J, trap 3: `owns`,
    /// before it, said no to all but the root itself).
    #[gtest]
    fn every_path_starts_with_the_root() {
        let root = NodePath::root();
        expect_that!(root.starts_with(&root), eq(true));
        expect_that!(np("title").starts_with(&root), eq(true));
        expect_that!(np("venues.#3.city").starts_with(&root), eq(true));
        expect_that!(np("shape.$Circle.radius").starts_with(&root), eq(true));
    }

    #[gtest]
    fn the_root_starts_with_no_field() {
        expect_that!(NodePath::root().starts_with(&np("title")), eq(false));
    }

    /// **The `Err` names the path that was outside,** not the container it was
    /// tested against: that is the path the caller asked about.
    #[gtest]
    fn ensure_starts_with_names_the_path_outside() {
        expect_that!(
            np("venues.#3.city").ensure_starts_with(&np("venues")),
            ok(anything())
        );
        expect_that!(
            np("address2").ensure_starts_with(&np("address")),
            err(displays_as(eq("no such path: address2")))
        );
    }

    // ── replace_prefix ───────────────────────────────────────────────────

    /// **A path under `from` moves under `to` with the rest of it kept,** which
    /// is how a set item's values move into the draft when it is edited.
    #[gtest]
    fn replace_prefix_keeps_the_rest_of_the_path() {
        let (item, draft) = (np("tags.#3"), np("tags.#9"));
        expect_that!(
            np("tags.#3.name").replace_prefix(&item, &draft),
            some(eq(&np("tags.#9.name")))
        );
        expect_that!(
            np("tags.#3.$Named.label").replace_prefix(&item, &draft),
            some(eq(&np("tags.#9.$Named.label")))
        );
    }

    #[gtest]
    fn replace_prefix_of_from_itself_is_to() {
        expect_that!(
            np("tags.#3").replace_prefix(&np("tags.#3"), &np("tags.#9")),
            some(eq(&np("tags.#9")))
        );
    }

    /// **A path outside `from` is refused, not cut short,** including one
    /// `from` is only a string prefix of: slicing `tags.#30.name` by the length
    /// of `tags.#3` would give `0.name`.
    #[gtest]
    fn replace_prefix_refuses_a_path_outside_from() {
        let (item, draft) = (np("tags.#3"), np("tags.#9"));
        expect_that!(np("tags.#30.name").replace_prefix(&item, &draft), none());
        expect_that!(np("other.name").replace_prefix(&item, &draft), none());
    }

    #[gtest]
    fn replace_prefix_at_the_root_adds_or_drops_no_stray_dot() {
        let root = NodePath::root();
        expect_that!(
            np("a.b").replace_prefix(&root, &np("x")),
            some(eq(&np("x.a.b")))
        );
        expect_that!(
            np("x.a.b").replace_prefix(&np("x"), &root),
            some(eq(&np("a.b")))
        );
        expect_that!(np("a.b").replace_prefix(&root, &root), some(eq(&np("a.b"))));
    }

    // ── spec_key and without_variants ────────────────────────────────────

    /// **Each item key folds into a `[]` on the segment before it,** since a
    /// spec names every item at once; variant segments are spelled the same.
    #[gtest]
    fn an_item_key_folds_into_brackets_in_the_spec_key() {
        expect_that!(np("title").spec_key(), eq("title"));
        expect_that!(np("answers.#0").spec_key(), eq("answers[]"));
        expect_that!(np("venues.#3.city").spec_key(), eq("venues[].city"));
        expect_that!(np("rows.#1.#2").spec_key(), eq("rows[][]"));
        expect_that!(
            np("shapes.#0.$Circle.radius").spec_key(),
            eq("shapes[].$Circle.radius")
        );
        expect_that!(NodePath::root().spec_key(), eq(""));
    }

    /// **Every variant segment goes, and what is left is the path through the
    /// model.** Ported from `path_without_variants`' suite test.
    #[gtest]
    fn without_variants_drops_every_variant_segment() {
        expect_that!(
            np("footprint.$Circle.size").without_variants(),
            eq("footprint.size")
        );
        expect_that!(
            np("outer.$First.inner.$A.x").without_variants(),
            eq("outer.inner.x")
        );
    }

    /// **An item key stays:** it names a real element of the model, though which
    /// one, a key being an identity and not a position, only the list knows.
    #[gtest]
    fn without_variants_keeps_item_keys_and_plain_paths() {
        expect_that!(
            np("shapes.#0.$Circle.radius").without_variants(),
            eq("shapes.#0.radius")
        );
        expect_that!(
            np("location.street").without_variants(),
            eq("location.street")
        );
        expect_that!(np("title").without_variants(), eq("title"));
    }

    // ── Borrow<str> and serde ────────────────────────────────────────────

    /// **A `&str` finds what a `NodePath` key put in** (trap 2). `Borrow<str>`
    /// holds only while a `NodePath` hashes as its `str` does, which the derived
    /// `Hash` gives exactly as long as the `String` is its only field.
    #[gtest]
    fn a_str_lookup_finds_a_node_path_key() {
        let mut map = HashMap::new();
        map.insert(np("venues.#3.city"), 1);
        expect_that!(map.get("venues.#3.city"), some(eq(&1)));
        expect_that!(map.get("venues.#3"), none());

        let set: HashSet<NodePath> = [np("tags.#2")].into_iter().collect();
        expect_that!(set.contains("tags.#2"), eq(true));
    }

    /// **On the wire a `NodePath` is a plain string,** alone and as a map key,
    /// so `WireForm` and `FormErrors` keep the format they had with `String`s.
    #[gtest]
    fn serde_round_trips_as_a_plain_string() {
        let json = serde_json::to_string(&np("venues.#3.city")).expect("serializes");
        expect_that!(json, eq("\"venues.#3.city\""));
        let back: NodePath = serde_json::from_str(&json).expect("deserializes");
        expect_that!(back, eq(&np("venues.#3.city")));

        let map: HashMap<NodePath, u8> = [(np("a.#1"), 7)].into_iter().collect();
        let json = serde_json::to_string(&map).expect("serializes");
        expect_that!(json, eq("{\"a.#1\":7}"));
        let back: HashMap<NodePath, u8> = serde_json::from_str(&json).expect("deserializes");
        expect_that!(back, eq(&map));
    }
}
