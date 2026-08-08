mod error;
pub use error::*;

mod fields;
pub use fields::*;

mod form;
pub use form::*;

mod widgets;
pub use widgets::*;

/// `#[derive(Form)]`. The trait [`Form`] and the derive share a name but live in
/// different namespaces (type vs. macro), so a single `use formoxus::Form;` brings
/// in whichever the context needs.
#[cfg(feature = "derive")]
pub use formoxus_macros::Form;

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
