use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use dioxus::core::Element;
use dioxus::prelude::{UnsyncStorage, Writable, WritableExt, use_store};
use dioxus::stores::Store;

use crate::error::FormError;

/// A button handler that receives the form's validated `Model` — only called
/// once `FormStoreExt::validate` succeeds. `Rc`, not `Box`: the generated
/// `onclick`/`onsubmit` closures run on every click, and each run needs to move
/// an owned copy into a fresh `async move` block, so the handler itself must be
/// cheaply `Clone`. Single-threaded (WASM), so `Rc` over `Arc`.
pub type Handler<M> = Rc<dyn Fn(M) -> Pin<Box<dyn Future<Output = ()>>>>;

/// A button handler that runs unconditionally — no validation attempt, no
/// access to the model. For buttons like Cancel that must work even while the
/// form is invalid.
pub type UncheckedHandler = Rc<dyn Fn() -> Pin<Box<dyn Future<Output = ()>>>>;

/// Wrap a plain async closure as a [`Handler`], for a `#[form(button(...))]`
/// field on a generated `…Handlers` struct.
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

/// A re-invokable async fetch a widget calls when it actually needs external
/// data — e.g. a picker's list of choices — supplied at the `store.render(...)`
/// call site rather than baked into the declaration, the same use-site-not-
/// declaration-site split [`Handler`] already gives buttons. Unlike a
/// `Handler<Model>`, which is always keyed to the form's model type, a
/// `Provider`'s `C` is whatever shape the *widget* needs (a list of choices,
/// typically), so each `#[form(component = ..., provided)]` field gets its own
/// concrete `Provider<C>` via that widget's [`crate::widgets::ProvidedWidget::Choices`].
/// `Rc`, not `Box`, so it can be called repeatedly — once per open, and once
/// per row when a `#[form(field_set)]` repeating group shares one provider
/// across every row — and so it's cheaply `Clone` for that repeated-row case.
pub type Provider<C> = Rc<dyn Fn() -> Pin<Box<dyn Future<Output = C>>>>;

/// Wrap a plain async closure as a [`Provider`].
pub fn provider<C, F, Fut>(f: F) -> Provider<C>
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = C> + 'static,
{
    Rc::new(move || Box::pin(f()))
}

pub trait FormStoreExt {
    type Model;
    type Handlers;
    type Providers;
    fn validate(&self) -> Option<Self::Model>;
    fn render(&self, handlers: Self::Handlers, providers: Self::Providers) -> Element;
}

impl<S: FormState + 'static> FormStoreExt for Store<S> {
    type Model = S::Model;
    type Handlers = S::Handlers;
    type Providers = S::Providers;

    fn validate(&self) -> Option<S::Model> {
        // `Store` is a Copy handle to shared reactive state; copy it for a mutable
        // binding, then write through the copy (mutates the same underlying store).
        let mut store = *self;
        store.write().validate()
    }

    fn render(&self, handlers: Self::Handlers, providers: Self::Providers) -> Element {
        S::render(*self, handlers, providers)
    }
}

pub trait Form: std::fmt::Debug {
    type State: FormState + Default + Clone + std::fmt::Debug + 'static;
}

pub trait FormState: FromModel<Self::Model> + ValidateForm<Self::Model> + std::fmt::Debug {
    type Model: std::fmt::Debug;
    /// The per-form `…Handlers` struct the derive generates: one field per
    /// `#[form(button(...))]`, each a [`Handler<Self::Model>`] or an
    /// [`UncheckedHandler`] depending on that button's resolved `HandlerKind`.
    type Handlers;
    /// The per-form `…Providers` struct the derive generates: one field per
    /// `#[form(component = ..., provided)]` field (typed [`Provider<C>`] for
    /// that widget's own `Choices`), plus one per embedded `#[form(field_set)]`
    /// field (typed as *that* field set's own `Providers`, nested regardless of
    /// whether it happens to need any). Plain `()` when nothing needs one.
    type Providers;
    fn validate(&mut self) -> Option<Self::Model>;
    fn has_errors(&self) -> bool;
    fn render(data: Store<Self>, handlers: Self::Handlers, providers: Self::Providers) -> Element
    where
        Self: Sized + 'static;
}

pub trait FromModel<Model> {
    fn from_model(model: &Model) -> Self;
}

pub trait ValidateForm<Model> {
    fn validate_form(&self, model: &Model) -> Vec<FormError>;
}

/// A reusable group of fields with no buttons of its own — `#[derive(FieldSet)]`
/// on a plain declaration struct, embeddable as a field inside a `Form` or
/// another `FieldSet` (e.g. `name`/`source`/tags shared by every question kind).
/// Mirrors [`Form`] minus everything button-related.
pub trait FieldSet: std::fmt::Debug {
    type State: FieldSetState + Default + Clone + std::fmt::Debug + 'static;
}

/// Mirrors [`FormState`] minus `Handlers` — a `FieldSet` has nothing to click,
/// so its `render` takes no handlers and its `validate`/`has_errors` are the
/// same shape either way.
pub trait FieldSetState:
    FromModel<Self::Model> + ValidateForm<Self::Model> + std::fmt::Debug
{
    type Model: std::fmt::Debug;
    /// Mirrors [`FormState::Providers`] — a `FieldSet` has no buttons but can
    /// still have provided-data fields (or embed another `FieldSet` that does),
    /// so it needs the same slot.
    type Providers;
    fn validate(&mut self) -> Option<Self::Model>;
    fn has_errors(&self) -> bool;
    /// Generic over the store's lens, not just `Store<Self>` (= `Store<Self,
    /// WriteSignal<Self>>`): a top-level field set (via [`use_field_set`]) is
    /// backed by a plain `WriteSignal`, but one embedded as a `#[form(field_set)]`
    /// field is handed a *projected* sub-store whose lens is a `dioxus_stores`
    /// field accessor type, not `WriteSignal` — `Store<Self>` alone wouldn't unify
    /// with that at the embedding call site in the generated `render_call`.
    fn render<L>(data: Store<Self, L>, providers: Self::Providers) -> Element
    where
        Self: Sized + 'static,
        // `Storage = UnsyncStorage`: every generated field accessor further down
        // (`render_default`/`FieldWidget::render`) needs to convert its own
        // projected `Store<FormField<T>, L2>` back into the plain `Store<FormField<T>>`
        // widgets are written against, and that conversion is only available when
        // the storage backend is `UnsyncStorage` — true everywhere in this
        // single-threaded (WASM) app, per the same `Rc`-not-`Arc` reasoning as
        // `Handler`/`UncheckedHandler` above.
        L: Writable<Target = Self, Storage = UnsyncStorage> + Copy + 'static;
}

pub trait FieldSetStoreExt {
    type Model;
    type Providers;
    fn validate(&self) -> Option<Self::Model>;
    fn render(&self, providers: Self::Providers) -> Element;
}

impl<S: FieldSetState + 'static> FieldSetStoreExt for Store<S> {
    type Model = S::Model;
    type Providers = S::Providers;

    fn validate(&self) -> Option<S::Model> {
        let mut store = *self;
        store.write().validate()
    }

    fn render(&self, providers: Self::Providers) -> Element {
        S::render(*self, providers)
    }
}

/// Create an empty field set (create mode), the same way [`use_form`] does for
/// a `Form`.
pub fn use_field_set<F: FieldSet>() -> Store<F::State> {
    use_store(|| F::State::default())
}

/// Create a field set seeded from an existing model (edit mode), the same way
/// [`use_form_from`] does for a `Form`.
pub fn use_field_set_from<F: FieldSet>(
    model: <F::State as FieldSetState>::Model,
) -> Store<F::State> {
    use_store(move || F::State::from_model(&model))
}

/// Create an empty form (create mode): the reactive `Store` for `F`'s state,
/// seeded with defaults. Call it as `use_form::<LoginForm>()` — you name only
/// your form struct; the synthetic `…State` type stays hidden behind `F::State`.
pub fn use_form<F: Form>() -> Store<F::State> {
    use_store(|| F::State::default())
}

/// Create a form seeded from an existing model (edit mode): every field starts
/// with `initial == value`. The model type is `F`'s cleaned output — for a form
/// whose declaration *is* its model, that's the form struct itself.
pub fn use_form_from<F: Form>(model: <F::State as FormState>::Model) -> Store<F::State> {
    use_store(move || F::State::from_model(&model))
}
