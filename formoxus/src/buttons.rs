//! What a form's buttons are, and what happens when one is clicked.
//!
//! The two halves are deliberately apart. A [`ButtonSpec`] is **static
//! knowledge** — which buttons exist, in what order, how each renders, whether
//! it validates first — and travels inside the [`FormSpec`](crate::FormSpec)
//! that `form!` builds. A [`ButtonFn`] is the behaviour, supplied at the
//! `render` call site instead, because a handler needs things (the form handle,
//! current props, a `Navigator`) that do not exist where the spec is declared.
//! `BUTTONS_PLAN.md` §2 records why that split is forced rather than chosen.

use std::collections::BTreeMap;

use crate::form::{Handler, UncheckedHandler};
use crate::label_case::{LabelCase, ToCase};

/// One button, as declared in `form!`'s `buttons:` block.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonSpec {
    /// The author's name for it, which is also the key a handler is supplied
    /// under. Not rendered — that's [`text`](Self::text).
    pub name: String,
    pub ty: ButtonType,
    /// Explicit label; `None` falls back to the name.
    pub text: Option<String>,
    /// `None` means "whatever the type implies" — a third state, distinct from
    /// either variant, and the reason this is an `Option`.
    pub invocation: Option<Invocation>,
}

impl ButtonSpec {
    pub fn new(name: &str, ty: ButtonType) -> Self {
        Self {
            name: name.to_string(),
            ty,
            text: None,
            invocation: None,
        }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    pub fn with_invocation(mut self, invocation: Invocation) -> Self {
        self.invocation = Some(invocation);
        self
    }

    /// What the button reads. Explicit `text` wins; otherwise the name in
    /// title case, so `sign_in` reads "Sign In" rather than "sign_in" — the
    /// same fallback a field label gets.
    pub fn label(&self) -> String {
        self.text
            .clone()
            .unwrap_or_else(|| self.name.to_case(LabelCase::Title))
    }

    /// The `invocation` if one was given, else what the type implies.
    pub fn invocation(&self) -> Invocation {
        self.invocation
            .unwrap_or_else(|| self.ty.default_invocation())
    }
}

/// Mirrors the five names `form!` accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonType {
    Submit,
    Reset,
    Cancel,
    Destructive,
    Button,
}

impl ButtonType {
    /// The `type` attribute. Only `submit` and `reset` have native behaviour
    /// worth keeping; everything else is an ordinary `button` so that clicking
    /// it does not submit the form out from under its own handler.
    pub fn html_type(&self) -> &'static str {
        match self {
            ButtonType::Submit => "submit",
            ButtonType::Reset => "reset",
            ButtonType::Cancel | ButtonType::Destructive | ButtonType::Button => "button",
        }
    }

    pub fn default_class(&self) -> &'static str {
        match self {
            ButtonType::Submit => "primary",
            ButtonType::Reset => "outline danger",
            ButtonType::Cancel => "outline secondary",
            ButtonType::Destructive => "danger",
            ButtonType::Button => "secondary",
        }
    }

    /// `Submit` and `Button` act ON the model (a save, a preview), so they want
    /// it validated. `Cancel`, `Reset` and `Destructive` are escape hatches
    /// that have to work on an invalid form — validating first would trap the
    /// user in it.
    pub fn default_invocation(&self) -> Invocation {
        match self {
            ButtonType::Submit | ButtonType::Button => Invocation::IfModelValidates,
            ButtonType::Cancel | ButtonType::Reset | ButtonType::Destructive => {
                Invocation::Unconditional
            }
        }
    }
}

/// Whether the model must pass validation before the handler is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invocation {
    /// Validate first; the handler receives the model and never runs if it
    /// fails.
    IfModelValidates,
    /// Run regardless, with no model.
    Unconditional,
}

/// What a button does, supplied at the render call site.
///
/// Which variant a closure becomes is decided by `using_fns!` from the
/// closure's **arity**, read syntactically: `|m| …` takes the model and is
/// [`Validated`](Self::Validated), `|| …` takes nothing and is
/// [`Unchecked`](Self::Unchecked). A trait could not make that choice — one
/// target type with impls for both `Fn(M)` and `Fn()` is a hard `E0119`, since
/// coherence cannot rule out a type implementing both.
pub enum ButtonFn<T> {
    Validated(Handler<T>),
    Unchecked(UncheckedHandler),
}

// Hand-written, not derived: `#[derive(Clone)]` would add a spurious `T: Clone`
// bound, the same trap `Provider`'s and `Form`'s hand-written impls document.
impl<T> Clone for ButtonFn<T> {
    fn clone(&self) -> Self {
        match self {
            ButtonFn::Validated(h) => ButtonFn::Validated(h.clone()),
            ButtonFn::Unchecked(h) => ButtonFn::Unchecked(h.clone()),
        }
    }
}

/// The handlers for one render, keyed by button name — what `using_fns!`
/// builds.
///
/// A map rather than a struct because the reflection path has no per-form type
/// to put fields on: `form!` is an expression macro producing a runtime
/// `FormSpec` value, so at the `render` call site there is nothing to name.
/// That is also why the completeness check is at render rather than at compile
/// time; [`Fns::reconcile`] is where it happens.
pub struct Fns<T> {
    fns: BTreeMap<String, ButtonFn<T>>,
}

impl<T> Clone for Fns<T> {
    fn clone(&self) -> Self {
        Fns {
            fns: self.fns.clone(),
        }
    }
}

impl<T> Default for Fns<T> {
    fn default() -> Self {
        Fns::new()
    }
}

impl<T> Fns<T> {
    pub fn new() -> Self {
        Fns {
            fns: BTreeMap::new(),
        }
    }

    /// Add one. Used by `using_fns!`, which emits one call per entry.
    pub fn with(mut self, name: &str, f: ButtonFn<T>) -> Self {
        self.fns.insert(name.to_string(), f);
        self
    }

    pub fn get(&self, name: &str) -> Option<&ButtonFn<T>> {
        self.fns.get(name)
    }

    pub fn is_empty(&self) -> bool {
        self.fns.is_empty()
    }

    /// Check these handlers against the buttons a spec declares, returning one
    /// message per mismatch.
    ///
    /// Both directions matter and they fail differently. A **missing** handler
    /// is a dead button the user can click; an **unknown** name is a handler
    /// that will never run, which is usually a typo and otherwise a rename that
    /// only got done on one side. Neither can be caught at compile time here,
    /// so this is the check that replaces `missing field 'cancel'`.
    pub fn reconcile(&self, buttons: &[ButtonSpec]) -> Vec<String> {
        let mut problems = Vec::new();
        for b in buttons {
            match self.fns.get(&b.name) {
                None => problems.push(format!("button `{}` has no fn supplied", b.name)),
                // The spec says whether the model has to validate first; the
                // closure's arity says whether it can receive one. Neither can
                // see the other — the spec is a runtime value and `using_fns!`
                // reads arity syntactically — so disagreement is caught here
                // and named in the terms the author writes.
                Some(f) => match (b.invocation(), f) {
                    (Invocation::IfModelValidates, ButtonFn::Unchecked(_)) => {
                        problems.push(format!(
                            "button `{}` validates its model, so its fn must take it — \
                             write `|m| …`, or declare `invocation: unconditional`",
                            b.name
                        ));
                    }
                    (Invocation::Unconditional, ButtonFn::Validated(_)) => {
                        problems.push(format!(
                            "button `{}` runs unconditionally, so its fn takes no model — \
                             write `|| …`, or declare `invocation: if_model_validates`",
                            b.name
                        ));
                    }
                    _ => {}
                },
            }
        }
        for name in self.fns.keys() {
            if !buttons.iter().any(|b| &b.name == name) {
                problems.push(format!("`{name}` is not a button this form declares"));
            }
        }
        // Two submit buttons share one `onsubmit`, so the second would never
        // fire and Enter-in-a-field would silently pick the first.
        let submits: Vec<&str> = buttons
            .iter()
            .filter(|b| b.ty == ButtonType::Submit)
            .map(|b| b.name.as_str())
            .collect();
        if submits.len() > 1 {
            problems.push(format!(
                "a form can have one submit button; this one declares {}",
                submits.join(", ")
            ));
        }
        problems
    }
}
