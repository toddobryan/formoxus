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
use std::borrow::Borrow;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

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
pub(crate) fn resolve_indices<'k>(
    path: &str,
    entry: impl Fn(&str) -> Option<&'k str>,
) -> Option<String> {
    if !path.contains('[') {
        return Some(path.to_string());
    }
    let mut resolved = String::with_capacity(path.len());
    for segment in path.split('.') {
        let (name, mut selectors) = segment.split_at(segment.find('[').unwrap_or(segment.len()));
        if !resolved.is_empty() {
            resolved.push('.');
        }
        resolved.push_str(name);
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
                    keys.find(|key| entry(&format!("{resolved}.{key}")) == Some(text.as_str()))?
                }
                None => keys.nth(selector.parse().ok()?)?,
            };
            resolved.push('.');
            resolved.push_str(key);
            selectors = rest;
        }
        if !selectors.is_empty() {
            return None;
        }
    }
    Some(resolved)
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
        super::resolve_indices(path, |list| keys.get(list).map(String::as_str))
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
}
