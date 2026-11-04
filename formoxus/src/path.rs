//! Compile-checked paths into a model.
//!
//! A leaf path (`email`, `venue.city`, `answers[].text`) is the wire format, so
//! it has to be a string at the boundary. [`Path`] is what keeps it from being a
//! string in *your* code: it wraps the same string, but the only way to obtain
//! one is [`path!`](macro@crate::path), which emits a witness borrow of the
//! field beside it.
//!
//! A path may name ONE item of a `Vec` by index, `path!(Quiz.answers[i])`, for
//! an error about that item: the index is a run-time value, so that path is
//! built when it is written, and the form turns `answers[2]` into the third
//! item's key (`answers.#5`) when the error is placed.
//!
//! ```ignore
//! submission.reject_field(path!(Signup.email), "already registered")
//! ```
//!
//! Misspell `email` and rustc says `no field 'emial' on type '&Signup'`, with
//! the caret on the segment. The alternative — `reject_field("emial", …)` — is
//! a panic on a server, discovered by a user.

use std::borrow::Cow;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// One leaf path into `T`, proven to exist at compile time.
///
/// **`PhantomData<fn() -> T>`, not `PhantomData<T>`.** A `Path` holds no `T`
/// and never will, and the `fn` form says so: it leaves `Path<T>` `Send`,
/// `Sync` and `Copy` no matter what `T` is, where `PhantomData<T>` would make a
/// path inherit auto-traits from a model it doesn't contain.
///
/// **A `Cow`, so `Clone` but not `Copy`.** A path with an index is built at
/// run time (`answers[2]`) and owns its string; every other path borrows its
/// literal and costs nothing to clone.
pub struct Path<T> {
    raw: Cow<'static, str>,
    model: PhantomData<fn() -> T>,
}

impl<T> Path<T> {
    /// Called only by [`path!`](macro@crate::path).
    ///
    /// **Deliberately hidden, and that is the whole design.** The guarantee is
    /// not that a `Path<T>` wraps a plausible string — it is that every
    /// `Path<T>` came from an invocation rustc type-checked against `T`. A
    /// public constructor would let `Path::new("emial")` back through the hole
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

// Every impl below is hand-written for one reason: `#[derive]` would add a
// `T: Clone`/`T: Debug`/… bound that is never satisfied by anything real here.
// A `Path<T>` contains no `T`, so none of these depend on `T` at all — the same
// trap `crate::form::Form` documents for its own `Clone`.
impl<T> Clone for Path<T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            model: PhantomData,
        }
    }
}

impl<T> Debug for Path<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Path({:?})", self.raw)
    }
}

impl<T> Display for Path<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl<T> PartialEq for Path<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for Path<T> {}

impl<T> Hash for Path<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T> AsRef<str> for Path<T> {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

/// `path` with each `[n]` replaced by the `n`th item's key, `n` counted from
/// 0 in the order `item_keys` lists them: `groups[1].$Random.set[0]` is
/// `groups.#3.$Random.set.#0` when `groups` is `"#0,#3"` and that group's set
/// is `"#0,#1"`. Each list's keys are read by the path resolved so far, so an
/// index inside an item finds that item's list.
///
/// `item_keys` gives a container's structure entry by its path, as
/// `collect_values` writes it: [`crate::FormState`] reads its own, and
/// [`crate::WireForm`] the one it carries.
///
/// `None` when an index is past the end of its list, or names something with
/// no item keys at all: an entry is item keys only if each is `#` and a
/// number, so a leaf's text is never read as one. A path without an index is returned as it is, which
/// is every path but one a validator wrote with `path!(…[i])`.
pub(crate) fn resolve_indices<'k>(
    path: &str,
    item_keys: impl Fn(&str) -> Option<&'k str>,
) -> Option<String> {
    if !path.contains('[') {
        return Some(path.to_string());
    }
    let mut resolved = String::with_capacity(path.len());
    for segment in path.split('.') {
        let (name, mut indices) = segment.split_at(segment.find('[').unwrap_or(segment.len()));
        if !resolved.is_empty() {
            resolved.push('.');
        }
        resolved.push_str(name);
        while let Some(after) = indices.strip_prefix('[') {
            let (digits, rest) = after.split_once(']')?;
            let index: usize = digits.parse().ok()?;
            let keys = item_keys(&resolved)?;
            if !keys
                .split(',')
                .all(|key| key.is_empty() || is_item_key(key))
            {
                return None;
            }
            let key = keys.split(',').filter(|key| !key.is_empty()).nth(index)?;
            resolved.push('.');
            resolved.push_str(key);
            indices = rest;
        }
        if !indices.is_empty() {
            return None;
        }
    }
    Some(resolved)
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
    /// Deliberately NOT `Clone`/`Eq`/`Hash`, to prove none of `Path`'s impls
    /// leak a bound onto the model.
    #[derive(Debug)]
    struct Bare;

    #[gtest]
    fn a_path_is_clone_and_eq_for_a_model_that_is_neither() {
        let a: Path<Bare> = Path::__new("venue.city");
        let b = a.clone();
        expect_that!(a, eq(&b));
        expect_that!(a.as_str(), eq("venue.city"));
    }

    #[gtest]
    fn display_is_the_wire_spelling_and_debug_is_not() {
        let p: Path<Model> = Path::__new("answers[].text");
        expect_that!(format!("{p}"), eq("answers[].text"));
        expect_that!(format!("{p:?}"), eq("Path(\"answers[].text\")"));
    }

    #[gtest]
    fn two_paths_with_the_same_spelling_are_the_same_path() {
        let a: Path<Model> = Path::__new("email");
        let b: Path<Model> = Path::__new("email");
        let c: Path<Model> = Path::__new("bio");
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
}
