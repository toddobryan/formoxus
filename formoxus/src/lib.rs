pub mod error;
pub mod fields;
pub mod form;
pub mod widgets;

/// `#[derive(Form)]`. The trait [`form::Form`] and the derive share a name but live
/// in different namespaces (type vs. macro), so importing `Form` brings whichever the
/// context needs.
#[cfg(feature = "derive")]
pub use formoxus_macros::Form;

/// The common surface. `use formoxus::prelude::*;` brings in the derive, the trait
/// vocabulary, and the built-in field widgets — the one blessed glob. For precise
/// imports, reach into the modules directly (`formoxus::fields::FormField`, …); the
/// crate root deliberately does *not* re-export everything.
pub mod prelude {
    // The derive macro. The trait of the same name is re-exported from `form` below;
    // they share a name across namespaces, so a single glob import brings both.
    #[cfg(feature = "derive")]
    pub use crate::Form;

    pub use crate::error::{FieldError, FormError};
    pub use crate::fields::{FormField, FormFieldStoreExt};
    pub use crate::form::{Form, FormState, FormStoreExt, FromModel, ValidateForm};
    pub use crate::widgets::{
        CheckboxInput, DefaultWidget, FieldErrors, FieldProps, FieldWidget, NumberInput,
        SelectChoice, SelectWidget, TextInput, UnsetBooleanSelect, render_default,
    };
}

/// Re-exports for macro-generated code — NOT a public API. The `Form` derive emits
/// fully-qualified `::formoxus::__private::…` paths (e.g. the `Store` derive) so a
/// consumer needs only `formoxus` in its `Cargo.toml`, never `dioxus` under that
/// exact name. Mirrors surreal-table's `__private` pattern.
#[doc(hidden)]
pub mod __private {
    pub use dioxus;

    /// A single-name scope that generated `Store`-derived structs glob-import, so
    /// the bare `dioxus_stores::…` paths that dioxus's `Store` derive emits resolve
    /// — without pulling the whole dioxus prelude into the caller's module. Glob
    /// import (not `use … as`) so multiple `#[derive(Form)]` in one module don't
    /// collide on the name.
    pub mod store_scope {
        pub use ::dioxus::stores as dioxus_stores;
    }
}
