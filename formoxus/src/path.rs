//! Compile-checked paths into a model.
//!
//! A leaf path (`email`, `venue.city`, `answers[].text`) is the wire format, so
//! it has to be a string at the boundary. [`Path`] is what keeps it from being a
//! string in *your* code: it wraps the same `&'static str`, but the only way to
//! obtain one is [`path!`](macro@crate::path), which emits a witness borrow of
//! the field beside it.
//!
//! ```ignore
//! submission.reject_field(path!(Signup.email), "already registered")
//! ```
//!
//! Misspell `email` and rustc says `no field 'emial' on type '&Signup'`, with
//! the caret on the segment. The alternative — `reject_field("emial", …)` — is
//! a panic on a server, discovered by a user.

use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// One leaf path into `T`, proven to exist at compile time.
///
/// **`PhantomData<fn() -> T>`, not `PhantomData<T>`.** A `Path` holds no `T`
/// and never will, and the `fn` form says so: it leaves `Path<T>` `Send`,
/// `Sync` and `Copy` no matter what `T` is, where `PhantomData<T>` would make a
/// path inherit auto-traits from a model it doesn't contain.
pub struct Path<T> {
    raw: &'static str,
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
            raw,
            model: PhantomData,
        }
    }

    /// The wire spelling — what [`FormState::push_field_error`] and the value
    /// map are keyed by.
    ///
    /// [`FormState::push_field_error`]: crate::FormState::push_field_error
    pub const fn as_str(&self) -> &'static str {
        self.raw
    }
}

// Every impl below is hand-written for one reason: `#[derive]` would add a
// `T: Clone`/`T: Debug`/… bound that is never satisfied by anything real here.
// A `Path<T>` contains no `T`, so none of these depend on `T` at all — the same
// trap `crate::form::Form` documents for its own `Clone`.
impl<T> Clone for Path<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Path<T> {}

impl<T> Debug for Path<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Path({:?})", self.raw)
    }
}

impl<T> Display for Path<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.raw)
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
        self.raw
    }
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
    fn a_path_is_copy_and_eq_for_a_model_that_is_neither() {
        let a: Path<Bare> = Path::__new("venue.city");
        let b = a;
        expect_that!(a, eq(b));
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
        expect_that!(a, eq(b));
        expect_that!(a, not(eq(c)));
    }
}
