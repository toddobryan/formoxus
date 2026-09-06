//! An enum-typed member, locked to one variant chosen before the form existed.

use dioxus::prelude::*;
use facet::{EnumType, Partial, ReflectError};
use std::collections::HashMap;
use crate::reflect::RenderCtx;
use crate::reflect::build::{FormMode, variant_members};
use crate::error::{FieldError, FormAccessError};
use crate::reflect::members::{ABSENT_DISPLAY, FormMember, owns, qualify};

/// The enum variant at a particular point
///
/// Deliberately not a bare `String`: `enum Filter { None, ByDate { .. } }` is a
/// perfectly ordinary model, so a `"None"` sentinel would make "leave this
/// optional field empty" indistinguishable from "pick the `None` variant."
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantChoice {
    // May be valid if behind an OptionMember.
    Unchosen,
    /// This variant, by name.
    Named(String),
}


/// An enum-typed field, locked to one answer chosen before the form (per the
/// design — variant choice is a construction parameter, not an editable field).
/// For a [`VariantChoice::Named`] answer it's a `FieldSet` over that variant's
/// fields, plus the name so `write_into` can replay the choice; for
/// [`VariantChoice::Absent`] it holds no members at all and writes `None`.
/// The choice itself is NOT a leaf — it never appears in a path or a submitted
/// value.
#[derive(Clone, Debug)]
pub struct VariantSet {
    pub name: String,
    pub label: Option<String>,
    pub enum_type: &'static EnumType,
    pub choice: VariantChoice,
    pub members: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FieldError>,
}

impl FormMember for VariantSet {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self) -> Option<String> {
        self.label.clone()
    }

    fn render(&self, ctx: &RenderCtx) -> Element {
        match &self.choice {
            // Visible but inert, so the user can see they chose to leave a value
            // out rather than the field silently vanishing. `disabled` also means
            // the browser won't submit it, so `ABSENT_DISPLAY` never round-trips.
            // This is the one member that renders without being a leaf — and the
            // natural spot for a `<select>` if variant choice ever goes live.
            VariantChoice::Unchosen => {
                let path = ctx.path(&self.name);
                let input = rsx! {
                    input { 
                        r#type: "text", 
                        name: "{path}",
                        value: "{ABSENT_DISPLAY}",
                        disabled: true
                    }
                };
                match &self.label {
                    Some(label) => rsx! { 
                        label { "{label}",
                            { input }
                        }
                    },
                    None => input,
                }
            }
            VariantChoice::Named(_) => {
                let nested = ctx.nested(&self.name);
                let members_rendered = self.members.iter().map(|m| m.render(&nested));
                rsx! {
                    { members_rendered.into_iter() }
                }
            },
        }
    }

    fn validate(&mut self) {
        self.errors.clear();
        if matches!(self.choice, VariantChoice::Unchosen) {
            self.errors.push(FieldError("You must choose a variant for this field.".to_string()))
        }
        for m in self.members.iter_mut() {
            m.validate();
        }
    }

    fn choose_variant(&mut self, prefix: &str, path: &str, variant: &str) -> Result<(), FormAccessError> {
        let nested = qualify(prefix, &self.name);
        if !owns(&nested, path) {
            return Err(FormAccessError(format!("no such path: {path}")));
        }

        // Not me, but mine: an enum nested inside my chosen variant's fields.
        // This is what the iterative disclosure loop used to arrange in advance —
        // now `outer.inner` simply becomes reachable once `outer` is answered.
        if path != nested {
            for m in self.members.iter_mut() {
                if owns(&qualify(&nested, &m.name()), path) {
                    return m.choose_variant(&nested, path, variant);
                }
            }
            return Err(FormAccessError(format!("no such path: {path}")));
        }

        let Some(chosen) = self.enum_type.variants.iter().find(|v| v.name == variant) else {
            // An `Err`, never a panic: with a live `<select>` this can arrive
            // from a stale client, and on wasm a panic aborts rather than
            // reaching an ErrorBoundary. Listing the real options makes the
            // message actionable.
            let known: Vec<&str> = self.enum_type.variants.iter().map(|v| v.name).collect();
            return Err(FormAccessError(format!(
                "{variant:?} is not a variant of the enum at {path} (expected one of {known:?})"
            )));
        };

        // Replace, never merge. A half-filled `Circle` has no coherent
        // `Rectangle` reading, so the old variant's members are discarded
        // wholesale — the destructive switch the UX rule warns about.
        self.choice = VariantChoice::Named(chosen.name.to_string());
        self.members = variant_members(chosen, None, FormMode::Blank, &nested);
        self.errors.clear();
        Ok(())
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
        for m in self.members.iter_mut() {
            m.clear_errors();
        }
    }

    fn clone_box(&self) -> Box<dyn FormMember> {
        Box::new(self.clone())
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.members.iter().any(|m| m.has_errors())
    }

    fn raw_value(&self) -> String {
        // Not a scalar — the choice is fixed, not an input of its own.
        String::new()
    }

    fn collect_leaves(&self, prefix: &str, out: &mut Vec<(String, String)>) {
        let nested = qualify(prefix, &self.name);
        for m in self.members.iter() {
            m.collect_leaves(&nested, out);
        }
    }

    fn is_present(&self) -> bool {
        !matches!(self.choice, VariantChoice::Unchosen)
    }

    fn apply_leaves(&mut self, prefix: &str, values: &HashMap<String, String>) {
        let nested = qualify(prefix, &self.name);
        for m in self.members.iter_mut() {
            m.apply_leaves(&nested, values);
        }
    }

    fn write_value_into<'p>(&self, mut partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        match &self.choice {
            VariantChoice::Named(variant) => {
                // The one thing a plain field set doesn't do: lock in the
                // variant before writing its fields, so `Partial::build`
                // materializes the right one.
                partial = partial.select_variant_named(variant)?;
                for m in self.members.iter() {
                    partial = m.write_into(partial)?;
                }
            }
            VariantChoice::Unchosen => unreachable!("A VariantChoice::Unchosen is handled by OptionMember")
        }
        Ok(partial)
    }
}
