use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

use dioxus::core::{Attribute, Callback};
use formoxus_spec::Tag;

use crate::{
    classes::Classes,
    field_attrs::{FieldAttrs, Validity},
    nodes::{Edit, ErrorsStore, ErrorsStoreExt, ValuesStore},
    path::NodePath,
    widgets::{Choice, FieldHandle, Widget, field_handle::OnFieldEvent},
};

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetCommon {
    /// The field's name in the PAGE: the control's `id`, what its `<label for>`
    /// and `aria-describedby` point at. `{form id}-{path}`, built by
    /// `NodeContext::id`, so it differs between two copies of one form on a page.
    pub id: String,
    /// The field's name in the DATA: its `name` attribute, its key in the value
    /// store, and what the server matches a submitted value to. The same for
    /// every copy of the form, and never containing the form id, which the
    /// server does not have.
    pub path: NodePath,
    pub label: Option<String>,
    /// The field's hint (`help_text:`), which [`field_row`](super::field_row::field_row)
    /// renders after the caption, and which its control's `aria-describedby`,
    /// from [`current_attrs`](WidgetProps::current_attrs), names. A widget not using
    /// `field_row` renders it with [`field_help`](super::field_help).
    pub help_text: Option<String>,
    pub errors: ErrorsStore,
    /// The form's resolved classes, for the widget to look up its parts' slots
    /// (`classes.part_class::<Widget>(Tag::Label, invalid)`). A custom widget gets
    /// them too, so it can borrow a built-in slot and follow the app's preset.
    /// An `Rc` from `NodeContext`; see the rule on `NodeContext::classes`.
    pub classes: Rc<Classes>,
}

impl WidgetCommon {
    /// Whether the field has errors to show: [`validity`](Self::validity) is
    /// [`Invalid`](Validity::Invalid). What the invalid classes and the error
    /// list key on, since an unchecked field and a valid one look alike there;
    /// only `aria-invalid` tells them apart.
    pub fn is_invalid(&self) -> bool {
        self.validity() == Validity::Invalid
    }

    /// Where this field stands with validation: unchecked, valid or invalid.
    /// Reading it while rendering subscribes the reader to this one field's
    /// errors, so a new error wakes this widget and no other.
    ///
    /// A built-in widget rarely needs it directly:
    /// [`current_attrs`](WidgetProps::current_attrs) turns it into `aria-invalid`,
    /// and [`is_invalid`](Self::is_invalid) is the yes-or-no the classes want.
    pub fn validity(&self) -> Validity {
        self.errors.validity_at(&self.path)
    }

    /// `part`'s classes from the form's `Classes`, with its invalid addition
    /// when the field has errors. For a part's `class` other than the control,
    /// such as the wrapping `div`; [`control_class`](WidgetProps::control_class) is
    /// for the control itself.
    pub fn part_class<W: Widget>(&self, tag: Tag) -> Option<String> {
        self.classes.part_class::<W>(tag, self.is_invalid())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetProps {
    pub common: WidgetCommon,
    /// The field's attributes BEFORE its errors are applied: its own, its
    /// constraints and `required`, which the node can work out in the form's
    /// scope. [`current_attrs`](Self::current_attrs) adds `aria-invalid` and
    /// `aria-describedby`, reading the store, so a widget spreads that and
    /// never this. Read this directly only where the error attributes don't
    /// matter: a `contains` check, or a `class`.
    ///
    /// Crate-private so a custom widget can't spread this by mistake: it would
    /// compile, look right, and silently lose both aria attributes.
    ///
    /// One exception: `VariantNode` passes the attributes with the error
    /// attributes ALREADY applied, as its select's `help_text` is `None`
    /// (see there), and `VariantSelect` spreads this as is.
    pub(crate) base_attrs: FieldAttrs,
    pub extra_attrs: Vec<Attribute>,
    /// The field's choices, read from it on every render (`Field::choices`),
    /// so a field whose choices change is shown with the new ones. `None` for
    /// a field without any; a chooser over a `bool` then offers true/false.
    pub choices: Option<Vec<Choice>>,
    pub(crate) values: ValuesStore,
    pub(crate) on_field_event: OnFieldEvent,
}

impl WidgetProps {
    /// Props for the field at `path`, whose control has the page id `id`, with
    /// no label, hint, choices or extra attributes. Those four are `pub`, so a
    /// caller sets them afterwards.
    ///
    /// A widget is handed its props, so this is rarely needed: it is for a test
    /// of a widget on its own. `attrs` are the field's attributes before its
    /// errors are applied; [`current_attrs`](Self::current_attrs) adds those.
    ///
    /// **Props built here never trigger a live check.** They are not connected
    /// to a form, so a [`FieldHandle`] from them writes the value store and
    /// tells nobody: no check on blur, and a clean result is not made stale. A
    /// widget that renders another should pass on a clone of its own props, not
    /// build new ones, or the inner widget loses its live check.
    pub fn new(
        id: impl Into<String>,
        path: NodePath,
        attrs: FieldAttrs,
        values: ValuesStore,
        errors: ErrorsStore,
        classes: Rc<Classes>,
    ) -> Self {
        Self {
            common: WidgetCommon {
                id: id.into(),
                path,
                label: None,
                help_text: None,
                errors,
                classes,
            },
            base_attrs: attrs,
            extra_attrs: Vec::new(),
            values,

            choices: None,
            on_field_event: OnFieldEvent::unreported(),
        }
    }

    /// The field's attributes as its form control should carry them: its own
    /// from `form!` and its constraints, `required`, and then the two that
    /// follow its errors, `aria-invalid` and `aria-describedby`. A widget spreads
    /// these, with [`extra_attrs`](Self::extra_attrs) merged in, onto the element
    /// it renders:
    ///
    /// ```ignore
    /// let attrs = props.current_attrs().merge_with_attrs(props.extra_attrs.clone());
    /// ```
    ///
    /// This is the only way a custom widget reaches the field's attributes. If
    /// a widget skipped it, its markup would look right, but a screen reader
    /// would never hear that the field is invalid, or read its hint and errors.
    ///
    /// **Call it from a component.** It reads the field's errors from the
    /// store, and a read subscribes whatever is rendering at that moment.
    /// [`LeafWidget::render`](super::LeafWidget::render) runs in the form's scope, so calling it there makes
    /// every error re-render the whole form. Have `render` return a
    /// `#[component]` that takes the props and calls this, as the built-in
    /// widgets do (`TextareaView`, `SelectView`). Then an error re-renders only
    /// that field.
    ///
    /// `aria-describedby` names the hint only when [`help_text`](WidgetCommon::help_text)
    /// is set, so a widget that renders the hint itself still passes it here.
    pub fn current_attrs(&self) -> FieldAttrs {
        self.base_attrs
            .with_error_attrs(&self.id, self.help_text.as_deref(), self.validity())
    }

    /// The form control's `class`: `part`'s slot, as the field's own `class`
    /// and `class_plus` replace or extend it.
    pub fn control_class<W: Widget>(&self, tag: Tag) -> Option<String> {
        self.base_attrs
            .class(self.classes.part::<W>(tag), self.is_invalid())
    }

    /// How this widget reads and writes its field's value: the only way to
    /// write one. See [`FieldHandle`] for which method goes on which element.
    ///
    /// Call it while rendering, then move the closures its `on_*` methods
    /// return into the element's event attributes. Each closure owns a clone
    /// of the handle, so nothing borrows the props.
    pub fn field_handle(&self) -> FieldHandle {
        FieldHandle::new(&self.path, self.values, self.on_field_event)
    }
}

impl Deref for WidgetProps {
    type Target = WidgetCommon;

    fn deref(&self) -> &WidgetCommon {
        &self.common
    }
}

impl DerefMut for WidgetProps {
    fn deref_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetWidgetProps {
    pub common: WidgetCommon,
    pub choices: Vec<Choice>,
    pub chosen: Vec<String>,
    pub required: bool,
    pub on_edit: Callback<Edit>,
}

impl Deref for SetWidgetProps {
    type Target = WidgetCommon;

    fn deref(&self) -> &WidgetCommon {
        &self.common
    }
}

impl DerefMut for SetWidgetProps {
    fn deref_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
}
