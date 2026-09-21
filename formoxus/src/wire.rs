//! What crosses a server-fn boundary, in both directions.
//!
//! Neither [`Form`] nor [`FormState`] can cross one. `Form` holds `Signal`s and
//! a `Callback`; `FormState` holds `Vec<Box<dyn FormMember>>`, and trait objects
//! do not deserialize. [`FormSpec`] holds `fn` pointers for the validator and
//! every custom widget.
//!
//! So the **spec crosses as code** — a function both sides call — and the
//! **values cross as data**. That split is not only a workaround: a spec that
//! travelled with the form would be a spec the client controls, and the server
//! naming its own `spec()` is what makes [`Submission::accept`] a trustworthy
//! check rather than a re-run of whatever rules arrived in the request.
//!
//! [`WireForm`] is the data half, and it goes both ways: values and errors out,
//! values and errors back. That return leg is what lets a server normalize what
//! it was sent and hand the corrected form back for confirmation.
//!
//! ```ignore
//! // server
//! #[post]
//! async fn create_account(wire: WireForm<Signup>) -> Result<WireForm<Signup>, ServerFnError> {
//!     let sub = Submission::accept(spec(), wire.values())?;
//!
//!     if email_taken(&sub.model().email).await? {
//!         return Ok(sub.reject_field(path!(Signup.email), "already registered"));
//!     }
//!
//!     // Normalize the TYPED model, then regenerate the leaves from it, so the
//!     // values cannot drift out of step with the value they describe.
//!     let mut model = sub.into_model();
//!     model.email = model.email.trim().to_lowercase();
//!     insert(&model).await?;
//!     Ok(WireForm::from_model(&model, spec()))
//! }
//!
//! // client
//! form.absorb(create_account(form.to_wire()).await?);
//! ```
//!
//! [`Form`]: crate::Form
//! [`FormState`]: crate::FormState
//! [`FormSpec`]: crate::FormSpec
//! [`Submission::accept`]: crate::Submission::accept

use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::form::{FormErrors, FormSpec, form_for};

/// A form's mutable state, as plain data: raw values by leaf path, plus
/// whatever anyone has objected to.
///
/// This is the whole of what a server needs and the whole of what it can send
/// back. Everything else about a form — its structure, labels, widgets,
/// validator — is rebuilt on each side from the shared [`FormSpec`].
///
/// **`initial` is deliberately absent.** Carrying it would be cheap, and it is
/// tempting for "skip the write if nothing changed". But it would arrive from
/// the client, so the server would be comparing against a number the caller
/// chose. Compare against the row being updated instead — the server has to
/// load it anyway.
#[derive(Serialize, Deserialize)]
// `bound = ""`: the derive would otherwise demand `T: Serialize + Deserialize`
// for the `PhantomData`, and there is no `T` in here to serialize.
#[serde(bound = "")]
pub struct WireForm<T> {
    values: HashMap<String, String>,
    errors: FormErrors,
    #[serde(skip)]
    model: PhantomData<fn() -> T>,
}

impl<T> WireForm<T> {
    /// Values and errors as they stand.
    pub fn new(values: HashMap<String, String>, errors: FormErrors) -> Self {
        Self {
            values,
            errors,
            model: PhantomData,
        }
    }

    /// Errors with no values — the common rejection, since the client already
    /// holds the values it sent. [`Form::absorb`](crate::Form::absorb) leaves
    /// values untouched when this is empty.
    pub fn from_errors(errors: FormErrors) -> Self {
        Self::new(HashMap::new(), errors)
    }

    /// Nothing wrong and nothing to change.
    pub fn empty() -> Self {
        Self::new(HashMap::new(), FormErrors::default())
    }

    /// The raw values, keyed by leaf path — what
    /// [`Submission::accept`](crate::Submission::accept) takes.
    pub fn values(&self) -> &HashMap<String, String> {
        &self.values
    }

    pub fn errors(&self) -> &FormErrors {
        &self.errors
    }

    /// True when nothing objected. Note an empty `fields` is not the test: a
    /// cross-field verdict has no path and lands in [`FormErrors::form`].
    pub fn is_clean(&self) -> bool {
        self.errors.form.is_empty() && self.errors.fields.is_empty()
    }
}

impl<T: Clone + Debug + PartialEq + Facet<'static>> WireForm<T> {
    /// Leaves regenerated from a model — the server's answer when it has
    /// changed something and wants the client to show the corrected form.
    ///
    /// Takes the spec because leaf paths come from the built tree, not from `T`
    /// alone: which paths exist depends on the chosen variants and row counts
    /// that [`form_for`] derives from the value.
    pub fn from_model(model: &T, spec: FormSpec<T>) -> Self {
        Self::new(form_for(model, spec).as_hash_map(), FormErrors::default())
    }
}

// Hand-written for the reason `crate::path::Path` documents: a derive would
// demand `T: Clone`/`T: Debug` for a `PhantomData` that holds no `T`.
impl<T> Clone for WireForm<T> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            errors: self.errors.clone(),
            model: PhantomData,
        }
    }
}

impl<T> Debug for WireForm<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WireForm")
            .field("values", &self.values)
            .field("errors", &self.errors)
            .finish()
    }
}

impl<T> From<FormErrors> for WireForm<T> {
    fn from(errors: FormErrors) -> Self {
        Self::from_errors(errors)
    }
}
