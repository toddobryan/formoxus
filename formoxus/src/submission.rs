//! A form that arrived from a client, rebuilt and validated on the server.

use std::{collections::HashMap, fmt::Debug};

use facet::Facet;

use crate::error::FormError;
use crate::form::{FormErrors, FormSpec, FormState, empty_form};
use crate::path::Path;
use crate::wire::WireForm;

/// A submitted form that passed its own validation, holding both the model the
/// caller wanted and the tree a server verdict can still be attached to.
///
/// **Why both halves.** Rebuilding a form server-side is four mechanical steps
/// — `empty_form`, `apply`, `validate`, and `collect_errors` on the way out —
/// and every `#[post]` that takes a form repeats them identically. Packaging
/// them as a plain `fn(spec, values) -> Result<T, FormErrors>` doesn't work,
/// though: a handler that finds something wrong *after* validation ("that
/// password is incorrect", "that username is taken") needs the `FormState` to
/// push it into, and a function returning only `T` has already dropped it. So
/// this keeps the pair, and hands out `&T` alone.
///
/// **Why `reject_*` consume `self`.** Once a server verdict is recorded the
/// submission is finished, and consuming makes that structural rather than a
/// warning in a comment: `validate` clears every field's errors before it
/// re-runs, so calling it after a push would silently erase the very thing the
/// caller just recorded. Take the value away and there is nothing left to call
/// it on.
///
/// Nothing here is server-only — `FormState` is plain data with no Dioxus
/// runtime behind it, which is what makes the whole approach possible. (A
/// `FormState` still cannot CROSS a server fn boundary, since it holds
/// `Box<dyn FormMember>`; building one on either side is a different thing.)
// `Debug` for the same reason the rest of this path requires it: a failed
// `expect`/matcher assertion has to be able to print what it got.
#[derive(Debug)]
pub struct Submission<T: Clone + Debug + Facet<'static>> {
    state: FormState<T>,
    model: T,
}

impl<T: Clone + Debug + PartialEq + Facet<'static>> Submission<T> {
    /// Rebuild the form from the raw `(path, value)` pairs a client sent, and
    /// run the form's own validation over it.
    ///
    /// The spec is the same one the client rendered from, so every per-field
    /// check AND the cross-field validator run again here — a request that
    /// skipped the browser entirely gets identical treatment, with no check
    /// restated by hand at the call site.
    ///
    /// `Err` carries everything the form objected to. Note it can arrive with
    /// an EMPTY `fields`: a cross-field validator's verdict has no path and
    /// lands in [`FormErrors::form`], so "did it pass?" is this `Result`, never
    /// `fields.is_empty()`.
    pub fn accept(spec: FormSpec<T>, values: &HashMap<String, String>) -> Result<Self, FormErrors> {
        let mut state = empty_form(spec);
        state.apply(values);
        match state.validate() {
            Some(model) => Ok(Self { state, model }),
            None => Err(state.collect_errors()),
        }
    }

    /// The validated model. Borrowed rather than moved out, because a caller
    /// that reads a field and then rejects still needs the submission intact.
    pub fn model(&self) -> &T {
        &self.model
    }

    /// Take the validated model, giving up the ability to reject.
    ///
    /// For a handler that is done checking and wants to hand the model onward
    /// by value. Rejecting afterwards is then a compile error rather than a
    /// judgement call, which is the same trade `reject_field` makes.
    pub fn into_model(self) -> T {
        self.model
    }

    /// Record a server-discovered verdict about one field and finish.
    ///
    /// For what only the server can know — nothing local is wrong with
    /// "incorrect password", so no validator could have caught it. The message
    /// lands at the path the widget that must display it already renders at.
    ///
    /// **Panics if `path` isn't a field of this form.** That is a caller bug (a
    /// typo, or a path assuming a shape the model doesn't have), not anything a
    /// user's input can provoke — and unlike the client, where a panic aborts
    /// instead of reaching an `ErrorBoundary`, a server panic unwinds into a
    /// 500. Failing loudly beats returning a `Result` that every call site has
    /// to `?` for a bug that should never ship, and beats dropping the message
    /// on the floor.
    /// **Takes a [`Path`], not a `&str`.** A typo is a compile error at the
    /// call site, so the panic below is now reachable only by a path
    /// `path!` cannot spell — a specific row, whose index only the runtime
    /// knows. Those go through [`FormState::push_field_error`] with a string.
    ///
    /// **Still panics on a path this form does not have.** A `Path<T>` proves
    /// the field exists on `T`; it does not prove this form renders it, since a
    /// form may cover part of a model, and an unchosen variant's fields are not
    /// in the tree. That remains a caller bug rather than anything a user's
    /// input can provoke, and on a server a panic unwinds into a 500 — which
    /// beats dropping the message on the floor.
    ///
    /// [`FormState::push_field_error`]: crate::FormState::push_field_error
    pub fn reject_field(mut self, path: Path<T>, message: &str) -> WireForm<T> {
        self.state
            .push_field_error(path.as_str(), message)
            .unwrap_or_else(|e| panic!("cannot reject `{path}`: {e}"));
        self.into_wire_with_errors()
    }

    /// Record a server-discovered verdict about the form as a whole and finish.
    ///
    /// The counterpart to [`reject_field`](Self::reject_field) for a verdict
    /// that names no field — "those credentials don't match" can't say whether
    /// it was the username or the password.
    pub fn reject(mut self, message: &str) -> WireForm<T> {
        self.state.errors.push(FormError(message.to_string()));
        self.into_wire_with_errors()
    }

    /// Accepted as submitted: the form's own leaves, no errors.
    ///
    /// For a handler that is finished and wants the client to absorb the
    /// canonical values. To send back values the server *changed*, normalize
    /// the model instead and use [`WireForm::from_model`] — that regenerates
    /// every leaf from the value, so the two cannot drift.
    pub fn into_wire(self) -> WireForm<T> {
        WireForm::new(self.state.as_hash_map(), FormErrors::default())
    }

    /// Values plus whatever has been objected to — what both `reject_*` return.
    fn into_wire_with_errors(self) -> WireForm<T> {
        let errors = self.state.collect_errors();
        WireForm::new(self.state.as_hash_map(), errors)
    }
}
