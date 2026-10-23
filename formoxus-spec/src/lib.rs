//! The HTML attribute facts that **formoxus** and **formoxus-macros** share.
//! Consumers depend on the `formoxus` facade crate, not on this one.
//!
//! One table of the attributes formoxus handles, each with rules for how it is
//! handled: its value type, who owns it, which elements (and which `<input>`
//! types) it is valid on, which value kinds it applies to, and whether
//! `validate` also enforces it on the server.
//!
//! **Why a separate crate.** `formoxus-macros` cannot depend on `formoxus`,
//! because `formoxus` depends on it to re-export the macros. Both can depend on
//! this one, so the macro reads the same table at expansion time (to parse each
//! value by its type, and to report an attribute that is invalid on a widget)
//! that `formoxus` reads at run time (to check and emit it).
//!
//! **No dependencies.** It is compiled for the macro on the build machine and
//! for `formoxus` on the app's target, including wasm, so it is plain `const`
//! data and `const fn`s.
//!
//! The design is in `.claude/memory/attribute_rules_design.md`, and the WHATWG
//! facts it encodes are in `.claude/memory/html_attributes_reference.md`.

mod attrs;
mod bound;
mod buttons;
mod elements;
mod fixed_keys;
mod input_types;
mod widget_names;
mod widgets;

pub use attrs::{
    Attr, AttrKey, AttrSet, AttrType, AttrValue, FieldType, FieldTypeSet, FieldTypeWithOptional,
    Owner,
};
pub use bound::{Bound, Step};
pub use buttons::{ButtonType, Invocation};
pub use elements::{HtmlElement, Tag};
pub use fixed_keys::FixedKey;
pub use input_types::{InputType, RenderedInputType};
pub use widget_names::WidgetName;
pub use widgets::{BuiltinWidget, Part};
