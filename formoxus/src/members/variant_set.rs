//! An enum-typed member, locked to one variant chosen before the form existed.

use crate::RenderCtx;
use crate::build::{FormMode, variant_members};
use crate::error::{FieldError, FormAccessError};
use crate::form::FieldErrors;
use crate::label_case::LabelCase;
use crate::members::{
    Edit, FieldSpecs, FormMember, default_label, ensure_owned, no_such_path, owns, qualify,
    variant_segment,
};
use crate::widgets::{VariantSelect, WidgetType};
use dioxus::prelude::*;
use facet::{EnumType, Partial, ReflectError, Variant};
use std::collections::HashMap;

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
/// [`VariantChoice::Unchosen`] it holds no members at all and writes `None`.
/// The choice itself is NOT a leaf — it never appears in a path or a submitted
/// value.
#[derive(Clone, Debug)]
pub struct VariantSet {
    pub name: String,
    pub label: Option<String>,
    pub custom_widget: Option<WidgetType>,
    pub enum_type: &'static EnumType,
    pub optional: bool,
    pub choice: VariantChoice,
    pub members: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FieldError>,
}

impl VariantSet {
    /// Where this member's *children* live: its own path plus a `$Variant`
    /// descriptor, so two variants that share a field name can't both claim
    /// `footprint.size`. `None` when unchosen — there are no children then.
    fn child_prefix(&self, prefix: &str) -> Option<String> {
        match &self.choice {
            VariantChoice::Named(variant) => Some(qualify(
                &qualify(prefix, &self.name),
                &variant_segment(variant),
            )),
            VariantChoice::Unchosen => None,
        }
    }

    /// Not me, but mine: an enum nested inside my chosen variant's fields. This is
    /// what the iterative disclosure loop used to arrange in advance — `outer.inner`
    /// simply becomes reachable once `outer` is answered.
    fn forward_to_child(&mut self, prefix: &str, edit: &Edit) -> Result<(), FormAccessError> {
        let path = edit.path();
        let Some(child_prefix) = self.child_prefix(prefix) else {
            return Err(no_such_path(path)); // unchosen: no children to be inside
        };
        // Paths are unique, so at most one child can own this one. Dispatching by
        // containment rather than trying each in turn is what lets a child's real
        // error reach the caller intact.
        for m in &mut self.members {
            if owns(&qualify(&child_prefix, &m.name()), path) {
                return m.edit(&child_prefix, edit);
            }
        }
        Err(no_such_path(path))
    }

    /// Swap to `variant`, or clear back to `Unchosen` with `None`.
    fn set_variant(&mut self, my_path: &str, variant: Option<&str>) -> Result<(), FormAccessError> {
        let Some(name) = variant else {
            // The `--none--` option. Always structurally legal; `validate()` decides
            // whether leaving it unchosen is an error.
            self.choice = VariantChoice::Unchosen;
            self.members = Vec::new();
            self.errors.clear();
            return Ok(());
        };
        let chosen = self.lookup_variant(name, my_path)?;
        self.choice = VariantChoice::Named(chosen.name.to_string());
        self.members = variant_members(
            chosen,
            None,
            FormMode::Blank,
            &qualify(my_path, &variant_segment(chosen.name)),
            self.optional,
        );
        self.errors.clear();
        Ok(())
    }

    /// Look up a variant by name, recording the failure against this member *and*
    /// returning it.
    ///
    /// Both, deliberately: a live `<select>` can send a stale name, and the useful
    /// place for that message is beside the select — but tests and programmatic
    /// callers still want the `Err`. Never a panic: on wasm that aborts rather than
    /// reaching an `ErrorBoundary`. Listing the real options makes it actionable.
    fn lookup_variant(
        &mut self,
        name: &str,
        my_path: &str,
    ) -> Result<&'static Variant, FormAccessError> {
        // Copy the `&'static` out first, so the `self.errors.push` below isn't
        // fighting a borrow of `self` held by the iterator.
        let enum_type = self.enum_type;
        if let Some(found) = enum_type.variants.iter().find(|v| v.name == name) {
            return Ok(found);
        }
        let known: Vec<&str> = enum_type.variants.iter().map(|v| v.name).collect();
        let message = format!(
            "{name:?} is not a variant of the enum at {my_path} (expected one of {known:?})"
        );
        self.errors.push(FieldError(message.clone()));
        Err(FormAccessError(message))
    }

    pub fn variants(&self) -> Vec<&'static str> {
        self.enum_type.variants.iter().map(|v| v.name).collect()
    }

    pub fn chosen(&self) -> Option<String> {
        match &self.choice {
            VariantChoice::Unchosen => None,
            VariantChoice::Named(v) => Some(v.clone()),
        }
    }
}

impl FormMember for VariantSet {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
    }

    fn render(&self, ctx: &RenderCtx) -> Element {
        // A `fieldset` so the picker and the fields it revealed read as one
        // thing — same treatment `FieldSet` gives a nested struct, and the
        // legend carries the label so the select doesn't repeat it.
        //
        // Both arms build the SAME rsx template: the select is unconditional
        // and `members` is simply empty when unchosen. That is deliberate — a
        // conditional wrapper would give the two arms different templates, and
        // dioxus would tear the `<select>` down and rebuild it on every change,
        // losing keyboard focus mid-interaction.
        let members: Vec<Element> = match &self.choice {
            VariantChoice::Unchosen => Vec::new(),
            VariantChoice::Named(variant) => {
                let nested = ctx.nested(&self.name).nested(&variant_segment(variant));
                self.members.iter().map(|m| m.render(&nested)).collect()
            }
        };
        rsx! {
            fieldset {
                if let Some(text) = self.label(ctx.label_case) {
                    legend {
                        "{text}"
                        if ctx.required {
                            span { class: "required", " *" }
                        }
                    }
                }
                VariantSelect {
                    path: ctx.path(&self.name),
                    // The legend above already names this group; a second copy
                    // beside the select would just be the same word twice.
                    label: None,
                    required: ctx.required,
                    errors: self.errors.clone(),
                    variants: self.variants(),
                    selected: self.chosen(),
                    on_edit: ctx.on_edit,
                }
                { members.into_iter() }
            }
        }
    }

    fn validate(&mut self) {
        self.errors.clear();
        if matches!(self.choice, VariantChoice::Unchosen) {
            self.errors.push(FieldError(
                "You must choose a variant for this field.".to_string(),
            ));
        }
        for m in &mut self.members {
            m.validate();
        }
    }

    fn edit(&mut self, prefix: &str, edit: &Edit) -> Result<(), FormAccessError> {
        // Two paths are in play, and they are NOT the same: this member sits at
        // `my_path`, where the `<select>` lives, but its children sit one segment
        // deeper under the chosen variant's descriptor. `forward_to_child` owns
        // that second one, via `child_prefix`.
        let path = edit.path();
        let my_path = qualify(prefix, &self.name);
        ensure_owned(&my_path, path)?;
        if path != my_path {
            return self.forward_to_child(prefix, edit);
        }
        match edit {
            Edit::ChooseVariant { variant, .. } => self.set_variant(&my_path, variant.as_deref()),
            _ => Err(FormAccessError(format!("{my_path} is an enum, not a list"))),
        }
    }

    fn push_field_error(
        &mut self,
        prefix: &str,
        path: &str,
        error: FieldError,
    ) -> Result<(), FormAccessError> {
        // Unlike `edit`, `path == my_path` IS meaningful here: `self.errors` is
        // already `Vec<FieldError>` — it's what renders beside the `<select>` —
        // so a server complaint about the CHOICE itself ("pick a grading
        // scheme") belongs right there, not forwarded to a child.
        let my_path = qualify(prefix, &self.name);
        ensure_owned(&my_path, path)?;
        if path == my_path {
            self.errors.push(error);
            return Ok(());
        }
        // Otherwise it's about a field inside the chosen variant — same
        // containment walk as `forward_to_child`.
        let Some(child_prefix) = self.child_prefix(prefix) else {
            return Err(no_such_path(path)); // unchosen: no children to be inside
        };
        for m in &mut self.members {
            if owns(&qualify(&child_prefix, &m.name()), path) {
                return m.push_field_error(&child_prefix, path, error);
            }
        }
        Err(no_such_path(path))
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
        for m in &mut self.members {
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
        // Not a scalar — the choice is fixed, not a widget of its own.
        String::new()
    }

    fn collect_leaves(&self, prefix: &str, out: &mut Vec<(String, String)>) {
        let Some(nested) = self.child_prefix(prefix) else {
            return; // unchosen: no members, so no leaves
        };
        for m in &self.members {
            m.collect_leaves(&nested, out);
        }
    }

    fn collect_errors(&self, prefix: &str, out: &mut FieldErrors) {
        // The inverse of `push_field_error`, NOT of `collect_leaves` above —
        // which is why this does NOT early-return when unchosen, and why it
        // reports `my_path` rather than only recursing.
        //
        // `self.errors` sits at `my_path`, where the `<select>` is, one segment
        // SHALLOWER than `child_prefix`'s `$Variant` — the same two-paths trap
        // `edit` documents. Reporting them under `nested` would name a path no
        // widget renders.
        //
        // Three writers put errors there and all three have to come back out:
        // `validate` (unchosen), `lookup_variant` (a stale name from a live
        // select), and `push_field_error` (a server verdict about the choice
        // itself). The first fires PRECISELY when there are no children to
        // recurse into, so an early return would drop the one error this
        // member reliably produces.
        //
        // Emitted before recursing so the collected order matches `render`,
        // which puts `VariantSelect` above `members`.
        let my_path = qualify(prefix, &self.name);
        if !self.errors.is_empty() {
            out.push((my_path, self.errors.clone()));
        }
        let Some(nested) = self.child_prefix(prefix) else {
            return; // unchosen: no children, and my own errors are already out
        };
        for m in &self.members {
            m.collect_errors(&nested, out);
        }
    }

    fn is_present(&self) -> bool {
        !matches!(self.choice, VariantChoice::Unchosen)
    }

    fn apply_leaves(&mut self, prefix: &str, values: &HashMap<String, String>) {
        let Some(nested) = self.child_prefix(prefix) else {
            return; // unchosen: nothing to apply into
        };
        for m in &mut self.members {
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
                for m in &self.members {
                    partial = m.write_into(partial)?;
                }
            }
            VariantChoice::Unchosen => {
                unreachable!("A VariantChoice::Unchosen is handled by OptionMember")
            }
        }
        Ok(partial)
    }

    fn apply_specs(&mut self, prefix: &str, fields: &FieldSpecs) {
        if let Some(spec) = fields.get(&qualify(prefix, &self.name)) {
            self.label = spec.label.clone().or(self.label.take());
            // A variant chooser is the one container that DOES have a widget of
            // its own — the `<select>` — so an override here is meaningful (a
            // radio group), even though nothing renders one yet.
            self.custom_widget = spec.custom_widget.clone().or(self.custom_widget.take());
        }
        // Two paths again, exactly as in `edit`: this member sits at `my_path`,
        // but its children sit one segment deeper under the chosen variant's
        // descriptor. `Unchosen` means there are none to visit.
        if let Some(child_prefix) = self.child_prefix(prefix) {
            for m in &mut self.members {
                m.apply_specs(&child_prefix, fields);
            }
        }
    }
}
