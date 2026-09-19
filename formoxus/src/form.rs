//! The two layers of a form.
//!
//! [`FormState<T>`] is the plain data: structure, typed values, errors, and the
//! variant answers that live nowhere else. No Dioxus runtime, no reactivity —
//! testable on its own. Lives in `state`, alongside its constructors
//! ([`form_for`]/[`empty_form`]).
//!
//! [`Form<T>`] is what a page holds: a `Signal` of that state, the live value
//! store beside it, and the callback a structural edit travels on. Built by
//! [`use_form`], defined here.
//!
//! [`FormSpec<T>`] is the author's declaration — title, per-field overrides,
//! validator, buttons — that `form!` builds and `FormState::apply_specs`
//! lays onto the tree. Lives in `spec`.

use crate::buttons::{ButtonFn, ButtonSpec, ButtonType, Fns};
use crate::error::{FieldError, FormAccessError, FormError};
use crate::label_case::LabelCase;
use crate::members::Edit;
use crate::{RenderCtx, ValuesByPath};
use dioxus::prelude::*;
use facet::Facet;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, future::Future, pin::Pin, rc::Rc};

mod spec;
mod state;

pub use spec::{FieldSpec, FormSpec};
pub use state::{FormState, empty_form, form_for};

pub type FieldErrors = Vec<(String, Vec<FieldError>)>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FormErrors {
    pub form: Vec<FormError>,
    pub fields: FieldErrors,
}

// ── Button/provider slots ────────────────────────────────────────────────

/// A button handler that receives the form's validated `Model` — only called
/// once [`FormState::validate`] succeeds. `Rc`, not `Box`: the generated
/// `onclick`/`onsubmit` closures run on every click, and each run needs to
/// move an owned copy into a fresh `async move` block, so the handler itself
/// must be cheaply `Clone`. Single-threaded (WASM), so `Rc` over `Arc`.
pub type Handler<M> = Rc<dyn Fn(M) -> Pin<Box<dyn Future<Output = ()>>>>;

/// A button handler that runs unconditionally — no validation attempt, no
/// access to the model. For buttons like Cancel that must work even while the
/// form is invalid.
pub type UncheckedHandler = Rc<dyn Fn() -> Pin<Box<dyn Future<Output = ()>>>>;

/// A re-invokable async fetch a widget calls when it needs external data —
/// e.g. a picker's list of choices — supplied at the render call site rather
/// than baked into the form's declaration.
///
/// **A newtype, not a type alias — this is load-bearing, not stylistic.**
/// `UncheckedHandler` and a bare `Rc<dyn Fn() -> Pin<Box<dyn Future<Output =
/// C>>>>` are both zero-argument closures; the only difference is what the
/// future resolves to. As a type ALIAS, `Provider<()>` would be the exact same type as
/// `UncheckedHandler`, and the blanket impls `IntoSlot<UncheckedHandler> for
/// F` / `IntoSlot<Provider<C>> for F` below would then be the *same impl* at
/// `C = ()` — a hard `E0119` conflicting-implementation error, caught at the
/// two `impl` blocks themselves, not at any call site that might use `C = ()`.
/// Verified directly: writing both impls against the type-alias version fails
/// to compile at all. The newtype makes `Provider<C>` a distinct nominal type
/// regardless of `C`, so the two impls never unify.
pub struct Provider<C>(Rc<dyn Fn() -> Pin<Box<dyn Future<Output = C>>>>);

// Hand-written, not derived: `#[derive(Clone)]` on a generic newtype adds a
// spurious `C: Clone` bound — the same trap `Form`'s hand-written `Clone`
// documents — even though `Rc` is `Clone` regardless of what it wraps.
impl<C> Clone for Provider<C> {
    fn clone(&self) -> Self {
        Provider(self.0.clone())
    }
}

impl<C> Provider<C> {
    /// Call the provider. Returns the future directly rather than being
    /// `async fn`, so a caller still writes `provide.call().await` — the same
    /// shape as calling the wrapped `Rc<dyn Fn() -> ...>` directly, which a
    /// plain struct can't support: implementing the `Fn` traits by hand isn't
    /// available on stable.
    pub fn call(&self) -> Pin<Box<dyn Future<Output = C>>> {
        (self.0)()
    }
}

/// Wrap a plain async closure as a [`Handler`].
pub fn handler<M, F, Fut>(f: F) -> Handler<M>
where
    F: Fn(M) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
{
    Rc::new(move |m| Box::pin(f(m)))
}

/// Wrap a plain async closure as an [`UncheckedHandler`].
pub fn unchecked_handler<F, Fut>(f: F) -> UncheckedHandler
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
{
    Rc::new(move || Box::pin(f()))
}

/// Wrap a plain async closure as a [`Provider`].
pub fn provider<C, F, Fut>(f: F) -> Provider<C>
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = C> + 'static,
{
    Provider(Rc::new(move || Box::pin(f())))
}

/// Converts a bare closure into whichever slot type a generated slot struct's
/// field asks for, so the macro building that struct never has to know, per
/// field, whether it's a validated handler, an unchecked one, or a provider.
/// The macro emits `IntoSlot::into_slot(expr)` for every slot, and the
/// FIELD'S OWN TYPE — known from the struct definition the macro also
/// generated — selects which blanket impl below applies. The author writes a
/// bare closure and never names a wrapper.
///
/// `impl From<F> for Handler<M>` can't do this instead: `Handler<M>` is
/// `Rc<dyn Fn…>`, foreign on both sides of `From`, so coherence rejects it. A
/// local trait sidesteps that.
pub trait IntoSlot<T> {
    fn into_slot(self) -> T;
}

impl<F, M, Fut> IntoSlot<Handler<M>> for F
where
    F: Fn(M) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
{
    fn into_slot(self) -> Handler<M> {
        handler(self)
    }
}

impl<F, Fut> IntoSlot<UncheckedHandler> for F
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
{
    fn into_slot(self) -> UncheckedHandler {
        unchecked_handler(self)
    }
}

impl<F, C, Fut> IntoSlot<Provider<C>> for F
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = C> + 'static,
{
    fn into_slot(self) -> Provider<C> {
        provider(self)
    }
}

/// The value store on its own, seeded from a state's leaves.
///
/// The low-level half of [`use_form`], which is what a page normally wants.
pub fn use_form_values<T>(form: &FormState<T>) -> ValuesByPath
where
    T: Clone + Debug + PartialEq + Facet<'static>,
{
    let leaves = form.leaves();
    use_store(move || leaves.into_iter().collect())
}

/// A live form: the state, the values beside it, and where a structural edit
/// goes.
///
/// `Copy`, so it drops into event handlers without ceremony.
pub struct Form<T: Clone + Debug + PartialEq + Facet<'static> + 'static> {
    /// Structure, typed values, errors — and *answers*. A fieldless enum
    /// variant contributes nothing to `leaves()`, so a variant choice lives
    /// nowhere but here. That's why it's `FormState` and not `FormSchema`, and
    /// why it's a `Signal`: a structural edit rebuilds it and the page has to
    /// re-render.
    state: Signal<FormState<T>>,
    /// Live raw values by qualified path. Changes on every keystroke, and
    /// survives a schema rebuild because paths are stable under one.
    values: ValuesByPath,
    /// Where a structural edit goes. Closes over `state`, which is what lets the
    /// widget layer stay ignorant of `T` — [`RenderCtx`] can't be generic
    /// without making every member type generic too.
    on_edit: Callback<Edit>,
}

// Hand-written, not derived: `#[derive(Copy)]` would add a spurious `T: Copy`
// bound — the same trap `crate::fields` documents for `Default` — even though
// `Signal`, `Store` and `Callback` are each `Copy` regardless of what they wrap.
impl<T: Clone + Debug + PartialEq + Facet<'static> + 'static> Clone for Form<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Clone + Debug + PartialEq + Facet<'static> + 'static> Copy for Form<T> {}

impl<T: Clone + Debug + PartialEq + Facet<'static> + 'static> Form<T> {
    /// The live value store, keyed by qualified path.
    /// `peek`, not `read`: the casing is fixed for the life of the form, so
    /// subscribing a render scope to the whole state for it would re-render on
    /// every unrelated structural edit.
    fn label_case(&self) -> LabelCase {
        self.state.peek().label_case()
    }

    pub fn values(&self) -> ValuesByPath {
        self.values
    }

    /// The state as it currently stands. Reading this subscribes the caller to
    /// *structural* changes.
    pub fn state(&self) -> ReadSignal<FormState<T>> {
        self.state.into()
    }

    /// The whole form — title, fields, errors, button row — with the handlers
    /// for this render.
    ///
    /// Handlers come in here rather than living in the `FormSpec` because they
    /// need what only the call site has: the `Form` itself (for `push_error`),
    /// current props, a `Navigator`. A form with no buttons passes
    /// `Fns::new()`.
    ///
    /// The shell lives here rather than on [`FormState`] because every button
    /// needs this handle to validate — `FormState` has the button *specs* but
    /// no way to run one.
    pub fn render(&self, fns: Fns<T>) -> Element {
        let ctx = RenderCtx::root(self.values, self.on_edit, self.label_case());
        let state = self.state.read();
        let buttons = state.buttons().to_vec();
        let problems = fns.reconcile(&buttons);

        // The submit button gets no `onclick`: native `type="submit"` already
        // routes both a click on it AND Enter-in-a-field through the form's
        // `onsubmit`, so one handler covers both. Wiring a click as well would
        // run it twice.
        let on_submit = buttons
            .iter()
            .find(|b| b.ty == ButtonType::Submit)
            .and_then(|b| fns.get(&b.name).cloned());
        let handle = *self;

        let rendered_buttons = self.render_buttons(&buttons, &fns);

        rsx! {
            div {
                class: "form",
                { state.render_title() }
                form {
                    onsubmit: move |e: FormEvent| {
                        // Without this the browser navigates and the handler's
                        // future is dropped mid-flight.
                        e.prevent_default();
                        let on_submit = on_submit.clone();
                        async move {
                            if let Some(f) = on_submit {
                                run_button(handle, f).await;
                            }
                        }
                    },
                    { state.render_fields(&ctx) }
                    { state.render_errors() }
                    { render_problems(&problems) }
                    { rendered_buttons }
                }
            }
        }
    }

    /// The button row, in declaration order. Nothing at all when the form
    /// declares none, so a view that hand-writes its own buttons is unaffected.
    fn render_buttons(&self, buttons: &[ButtonSpec], fns: &Fns<T>) -> Element {
        if buttons.is_empty() {
            return rsx! {};
        }
        let handle = *self;
        let rendered = buttons
            .iter()
            .map(|b| {
                let f = fns.get(&b.name).cloned();
                let is_submit = b.ty == ButtonType::Submit;
                rsx! {
                    button {
                        key: "{b.name}",
                        r#type: "{b.ty.html_type()}",
                        class: "{b.ty.default_class()}",
                        // A button with nothing behind it is inert rather than
                        // absent: omitting it would hide the mistake, and a
                        // live `submit` with no handler would reload the page.
                        disabled: f.is_none(),
                        onclick: move |_| {
                            let f = f.clone();
                            async move {
                                if let Some(f) = f
                                    && !is_submit
                                {
                                    run_button(handle, f).await;
                                }
                            }
                        },
                        "{b.label()}"
                    }
                }
            })
            .collect::<Vec<_>>();
        rsx! {
            // A grouping element so the button row is addressable as one
            // thing: `.formoxus-buttons` is the hook for laying it out, and
            // for opting its buttons out of any full-width rule a consumer's
            // stylesheet applies to form controls.
            div { class: "formoxus-buttons", { rendered.into_iter() } }
        }
    }

    pub fn render_fragment(&self) -> Element {
        self.state.read().render_fragment(&RenderCtx::root(
            self.values,
            self.on_edit,
            self.label_case(),
        ))
    }

    pub fn render_title(&self) -> Element {
        self.state.read().render_title()
    }

    pub fn render_fields(&self) -> Element {
        self.state.read().render_fields(&RenderCtx::root(
            self.values,
            self.on_edit,
            self.label_case(),
        ))
    }

    pub fn render_errors(&self) -> Element {
        self.state.read().render_errors()
    }

    /// Add a form-level error — one that belongs to the form as a whole rather
    /// than to any field.
    ///
    /// The case this exists for is an answer that only the server has: "invalid
    /// credentials", "that username is taken". A field validator cannot produce
    /// it, because nothing local is wrong.
    ///
    /// **Cleared by the next [`validate`](Self::validate).** That is the intended
    /// lifetime, not a caveat: an error from the last round trip should not
    /// outlive the next submit. It also means a pushed error never blocks
    /// `validate`, since the clear happens first.
    ///
    /// Writing through a copy of the `Signal` for the same reason `validate`
    /// does: it keeps `&self` here, which is what lets `Form` stay `Copy` and
    /// drop into an event handler.
    pub fn push_error(&self, error: FormError) {
        let mut state = self.state;
        state.write().errors.push(error);
    }

    /// Attach an error to one field by path — the per-field counterpart to
    /// [`push_error`](Self::push_error).
    ///
    /// For a server verdict that DOES know which field is wrong: "that
    /// username is taken" names the field, unlike "invalid credentials",
    /// which can't (nothing local says whether it was the username or the
    /// password). `Err` means `path` isn't a field this form has — a caller
    /// bug (a typo, or a path that assumes a shape the model doesn't have),
    /// not something a user's input can trigger.
    ///
    /// **Cleared by the next [`validate`](Self::validate)**, same lifetime as
    /// `push_error` and for the same reason — inherited for free, not
    /// special-cased here: `FormField::validate` already clears and rebuilds
    /// every leaf's own `errors` on each call, so a stale server-pushed error
    /// cannot outlive the next submit.
    pub fn push_field_error(&self, path: &str, message: &str) -> Result<(), FormAccessError> {
        let mut state = self.state;
        state.write().push_field_error(path, message)
    }

    /// Push the live values into the state, then build the model.
    ///
    /// Lives here because this is the only place that knows both halves — a
    /// caller never has to remember that `apply` comes first.
    pub fn validate(&self) -> Option<T> {
        // `Signal` is a `Copy` handle to shared reactive state, so copy it for a
        // mutable binding and write through the copy. Keeps `&self` here, which
        // is what lets `Form` stay `Copy` and drop into event handlers.
        let mut state = self.state;
        let mut state = state.write();
        state.apply(&self.values.read().clone());
        state.validate()
    }
}

/// Run one button's handler.
///
/// **The `ButtonFn` variant decides how it is called, not the spec's
/// `invocation`** — it is the only thing that *can* decide, since a validated
/// handler takes a model and an unchecked one takes nothing, and no amount of
/// declared intent conjures the right arity at runtime. A spec that disagrees
/// with the closure it was given is reported by [`Fns::reconcile`] instead, so
/// the disagreement is visible rather than silently resolved.
async fn run_button<T>(form: Form<T>, f: ButtonFn<T>)
where
    T: Clone + Debug + PartialEq + Facet<'static> + 'static,
{
    match f {
        ButtonFn::Validated(h) => {
            // No model, no call: the errors `validate` just wrote are already
            // on the fields, so the page explains itself.
            if let Some(model) = form.validate() {
                h(model).await;
            }
        }
        ButtonFn::Unchecked(h) => h().await,
    }
}

/// Handler/spec mismatches, rendered beside the form's own errors and marked
/// apart from them — these are a programming mistake, not something the person
/// filling in the form did.
fn render_problems(problems: &[String]) -> Element {
    if problems.is_empty() {
        return rsx! {};
    }
    let problems = problems.to_vec();
    rsx! {
        ul {
            class: "form-errors formoxus-button-problems",
            for p in problems {
                li { class: "form-error", "{p}" }
            }
        }
    }
}

/// Make a [`FormState`] live.
///
/// ```ignore
/// use_form(|| empty_form(question_form()))       // create
/// use_form(|| form_for(&question, question_form()))  // edit
/// ```
///
/// **A thunk, not a value.** Building a `FormState` means walking `T`'s facet
/// shape and allocating a `Vec<Box<dyn FormMember>>`, and `use_signal`'s
/// initializer runs exactly once — so a value argument would be constructed on
/// every render of the calling component and thrown away every time but the
/// first. Deferring it costs one `||` and means the walk happens once.
///
/// One hook rather than a create/edit pair, because the branch happens *inside*
/// the closure, which isn't a hook call, so the rules of hooks don't care:
///
/// ```ignore
/// let form = use_form(|| match existing.as_ref() {
///     Some(model) => form_for(model, question_form()),
///     None => empty_form(question_form()),
/// });
/// ```
///
/// **The state is built once and never tracked.** The closure running once looks
/// like the form follows its input, and it does not. For a model arriving from a
/// `use_server_future`, put the form in its own component that takes the model
/// as a prop — it then can't mount before the data exists — and give it a `key:`
/// so a *different* model remounts it.
pub fn use_form<T: Clone + Debug + PartialEq + Facet<'static> + 'static>(
    state: impl FnOnce() -> FormState<T>,
) -> Form<T> {
    let state = use_signal(state);
    // `peek`, not `read`: seeding the store must not subscribe this component to
    // the state, or every structural edit would re-run the initializer's scope
    // for nothing.
    let values = use_store(move || state.peek().leaves().into_iter().collect());
    let on_edit = use_callback(move |edit: Edit| {
        let mut state = state;
        if let Err(e) = state.write().edit(&edit) {
            // An unknown variant name has already been recorded against the
            // `VariantSet`, so it renders beside the select. Anything else is a
            // bug or a forged event — log rather than panic, since on wasm a
            // panic aborts instead of reaching an `ErrorBoundary`.
            warn!("rejected form edit: {e}");
        }
    });
    Form {
        state,
        values,
        on_edit,
    }
}
