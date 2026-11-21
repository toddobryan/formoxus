use std::marker::PhantomData;
use std::rc::Rc;

use dioxus::core::Callback;
use dioxus::html::FileData;
use dioxus::prelude::{FocusEvent, FormEvent};
use dioxus::signals::{ReadableExt, WritableExt};
use formoxus_spec::Tag;

use crate::classes::Classes;
use crate::field_attrs::Validity;
use crate::nodes::{Edit, FormWide, ItemValue};
use crate::path::NodePath;
use crate::stores::{ErrorsView, ValuesView};
use crate::widgets::widget::Widget;

/// What a widget tells the form happened to one field, for
/// `apply_field_event` (in `form.rs`, private there, so not a link).
///
/// Named for what happened, not for what the form does about it, which the
/// form decides from the field's errors and whether it was typed into. A
/// chooser (a select, a checkbox, a radio) sends both, `Written` then
/// `Blurred`: it has no typing, so its change counts as the blur, and that
/// is what makes it check at once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FieldEventKind {
    /// The field's new value is already in the value store. The widget
    /// writes first and sends this second, so a check reads the new value
    /// and not the one before this keystroke.
    Written,
    /// The field lost focus.
    Blurred,
}

/// One report from a widget to its form: what happened, to which field, and
/// whether that field is live-validated.
///
/// `is_live` is copied from the [`OnFieldEvent`] that sent it. It is carried
/// so the form can act on one event without knowing which part of the tree
/// it came from.
#[derive(Debug)]
pub(crate) struct FieldEvent {
    /// The field's store key, as the widget's props have it (`answers.#3.text`).
    pub(crate) path: NodePath,
    pub(crate) kind: FieldEventKind,
    /// Whether this field is checked as the user goes; see
    /// [`OnFieldEvent::is_live`].
    pub(crate) is_live: bool,
}

/// Where a widget sends its [`FieldEvent`]s, as threaded down the node tree in
/// [`NodeContext`](crate::nodes::NodeContext) and handed to each widget in
/// [`WidgetProps`](crate::widgets::WidgetProps).
///
/// Crate-private, and so are both its fields, so that only the form can build
/// one. A widget never sees it: it gets a [`FieldHandle`], which sends through
/// this after every write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OnFieldEvent {
    /// The form's callback, from `use_form`. `None` for props not connected to
    /// a form ([`WidgetProps::new`](crate::widgets::WidgetProps::new)), whose
    /// events go nowhere.
    pub(crate) callback: Option<Callback<FieldEvent>>,
    /// Whether the form checks fields as the user goes. Set once, in
    /// `root_ctx`, from the form's `live_validation:`.
    pub(crate) is_live: bool,
}

impl OnFieldEvent {
    /// Connected to no form, so every event is dropped. `is_live` is then
    /// never read, and `false` is only for definiteness.
    pub(crate) fn unreported() -> Self {
        Self {
            callback: None,
            is_live: false,
        }
    }

    /// Tell the form `kind` happened to the field at `path`. Does nothing when
    /// there is no form to tell.
    pub(crate) fn send(&self, path: &NodePath, kind: FieldEventKind) {
        if let Some(callback) = self.callback {
            callback(FieldEvent {
                path: path.clone(),
                kind,
                is_live: self.is_live,
            });
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Leaf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Variant;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct List;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Set;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Items;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SetItem;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Struct;

/// How a widget reads and writes its field's value, from
/// [`WidgetProps::field_handle`](crate::widgets::WidgetProps::field_handle).
///
/// **The only way a widget can write a value.** Every write goes on to tell
/// the form, which is what checks a field as the user goes: once they leave a
/// field they typed into, and on every keystroke once it shows an error. A
/// widget that wrote the value store directly would never be checked before
/// submit. That is why this type exists rather than a bare `write_value`.
///
/// Which handlers go on which element:
///
/// | element                                      | handlers                    |
/// |----------------------------------------------|-----------------------------|
/// | something typed into (`<input>`, `<textarea>`) | `oninput: field.on_input()`, `onblur: field.on_blur()` |
/// | a chooser (`<select>`, a checkbox, a radio)  | `onchange: field.on_change()` |
///
/// ```ignore
/// let field = props.field_handle();
/// rsx! {
///     input {
///         value: field.current(),
///         oninput: field.on_input(),
///         onblur: field.on_blur(),
///     }
/// }
/// ```
///
/// For an element whose event does not carry the value as `event.value()`,
/// call [`input`](Self::input), [`change`](Self::change) and
/// [`blur`](Self::blur) from a handler of your own.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundField<K = Leaf> {
    id: String,
    path: NodePath,
    label: Option<String>,
    help_text: Option<String>,
    form: Rc<FormWide>,
    _data: PhantomData<K>,
}

impl<K> BoundField<K> {
    pub(crate) fn new(
        id: String,
        path: NodePath,
        label: Option<String>,
        help_text: Option<String>,
        form: Rc<FormWide>,
    ) -> Self {
        Self {
            id,
            path,
            label,
            help_text,
            form,
            _data: PhantomData,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &NodePath {
        &self.path
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub(crate) fn with_label(self, label: String) -> Self {
        Self {
            label: Some(label),
            ..self
        }
    }

    pub fn help_text(&self) -> Option<&str> {
        self.help_text.as_deref()
    }

    pub(crate) fn with_help_text(self, help_text: String) -> Self {
        Self {
            help_text: Some(help_text),
            ..self
        }
    }

    pub fn classes(&self) -> &Rc<Classes> {
        &self.form.classes
    }

    pub fn errors(&self) -> ErrorsView {
        self.form.errors
    }

    pub fn values(&self) -> ValuesView {
        ValuesView(self.form.contents.values)
    }

    fn send_edit(&self, edit: Edit) {
        (self.form.on_edit)(edit)
    }

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
    /// [`current_attrs`](crate::widgets::WidgetProps::current_attrs) turns it into `aria-invalid`,
    /// and [`is_invalid`](Self::is_invalid) is the yes-or-no the classes want.
    pub fn validity(&self) -> Validity {
        self.errors().validity_at(&self.path)
    }

    /// `part`'s classes from the form's `Classes`, with its invalid addition
    /// when the field has errors. For a part's `class` other than the control,
    /// such as the wrapping `div`; [`control_class`](crate::widgets::WidgetProps::control_class) is
    /// for the control itself.
    pub fn part_class<W: Widget>(&self, tag: Tag) -> Option<String> {
        self.form.classes.part_class::<W>(tag, self.is_invalid())
    }
}

impl BoundField {
    /// The field's raw value, or `""` if it has none yet. Reading it while
    /// rendering subscribes the reader to this one field.
    ///
    /// **No value is normal, not an error.** The store can lack a key the
    /// form renders: a variant chosen after mount reveals fields that were
    /// never given a value. An absent key reads as `""`, the same "empty IS
    /// absence" rule the form applies when it validates.
    pub fn current(&self) -> String {
        let slot = self.form.contents.values.get_unchecked(self.path.clone());
        slot.try_read().map(|v| v.clone()).unwrap_or_default()
    }

    /// The user typed: write `raw`, then report it. A field that shows an
    /// error is checked again at once, so the error goes as soon as it is
    /// fixed; any other field waits for [`blur`](Self::blur).
    pub fn input(&self, raw: String) {
        self.form.contents.write_value(&self.path, raw);
        self.form
            .on_field_event
            .send(&self.path, FieldEventKind::Written);
    }

    /// [`input`](Self::input) as an `oninput` handler, for an element the user
    /// types into. Pair it with [`on_blur`](Self::on_blur), or the field is
    /// not checked until submit.
    pub fn on_input(&self) -> impl FnMut(FormEvent) + 'static {
        let handle = self.clone();
        move |event| handle.input(event.value())
    }

    /// The user chose: write `raw`, then check it at once. A chooser has no
    /// typing to wait out, so its change counts as leaving the field too.
    pub fn change(&self, raw: String) {
        self.form.contents.write_value(&self.path, raw);
        self.form
            .on_field_event
            .send(&self.path, FieldEventKind::Written);
        self.form
            .on_field_event
            .send(&self.path, FieldEventKind::Blurred);
    }

    /// [`change`](Self::change) as an `onchange` handler, for a `<select>`, a
    /// checkbox or a radio. Needs no `onblur`.
    pub fn on_change(&self) -> impl FnMut(FormEvent) + 'static {
        let handle = self.clone();
        move |event| handle.change(event.value())
    }

    /// The user left the field. It is checked if they typed into it since it
    /// was last checked; otherwise nothing happens, so tabbing through a form
    /// does not cover it in errors.
    pub fn blur(&self) {
        self.form
            .on_field_event
            .send(&self.path, FieldEventKind::Blurred);
    }

    /// [`blur`](Self::blur) as an `onblur` handler, to go with
    /// [`on_input`](Self::on_input).
    pub fn on_blur(&self) -> impl FnMut(FocusEvent) + 'static {
        let handle = self.clone();
        move |_| handle.blur()
    }

    /// The user picked a file, or removed one: write its name to the store
    /// (`""` for `None`), put the file in the form's file table at this path
    /// or take it out, then check it at once, as [`change`](Self::change)
    /// does. A pick is finished the moment it happens, and a file input's blur
    /// means nothing, so there is no typing to wait out.
    ///
    /// The name goes in the store like any leaf's value, which is what the
    /// widget shows and what `validate` reads as the file's name. The file
    /// itself goes only in the table, which `validate` and the live check read
    /// for its size and type, and which the form sends with the wire. A widget
    /// reaches the table only through here, never directly.
    ///
    /// **`None` drops the file:** call it with `None` only for a Remove
    /// button. A browser may send a `change` with no file when the user
    /// cancels the dialog, and that must keep the file, which is why
    /// [`on_pick_file`](Self::on_pick_file) never calls this with `None`.
    pub fn pick_file(&self, file_data: Option<&FileData>) {
        let mut files = self.form.contents.files;
        self.form
            .contents
            .write_value(&self.path, file_data.map_or(String::new(), FileData::name));
        match file_data {
            Some(file) => files.write().insert(self.path.clone(), file.clone()),
            None => files.write().shift_remove(&self.path),
        };
        self.form
            .on_field_event
            .send(&self.path, FieldEventKind::Written);
        self.form
            .on_field_event
            .send(&self.path, FieldEventKind::Blurred);
    }

    /// [`pick_file`](Self::pick_file) as an `onchange` handler, for an
    /// `<input type="file">`. Needs no `onblur`.
    ///
    /// It takes the first of the event's `files()`, and does nothing when
    /// there are none, which is what a cancelled dialog can send: so the file
    /// already chosen, or kept from before, stays. Only a Remove button, with
    /// `pick_file(None)`, drops it. It never reads the event's `value()`,
    /// which for a file input is `C:\fakepath\` and the name.
    pub fn on_pick_file(&self) -> impl FnMut(FormEvent) + 'static {
        let handle = self.clone();
        move |event| {
            if let Some(file) = event.files().first() {
                handle.pick_file(Some(file));
            }
        }
    }

    /// The file the user picked for this field, from the form's file table,
    /// or `None` if they have picked none since the form was filled. Reading
    /// it while rendering subscribes the reader to the whole table, so any
    /// pick in the form re-renders it.
    ///
    /// **`None` with a name in the store is a kept file, not a missing one.**
    /// An edit form filled from a model starts with the name of the file it
    /// already has (`FileUpload::Existing`) and no file in the table, because
    /// that file is on the server. So a widget tells the two apart by the
    /// table, not the name: [`current`](Self::current) is the name either
    /// way, and this is `Some` only for a file picked here.
    ///
    /// The result is a copy, and changing it changes nothing in the form.
    /// The only way in is [`pick_file`](Self::pick_file).
    pub fn picked_file(&self) -> Option<FileData> {
        self.form.contents.files.read().get(&self.path).cloned()
    }

    /// How many times the form has been reset. Reading it while rendering
    /// subscribes the reader, so it re-renders on every reset.
    ///
    /// **For a file widget's `key`.** A script cannot clear what an
    /// `<input type="file">` shows, so after a reset the only way to make the
    /// browser forget the user's pick is a new element, and a new key is what
    /// makes Dioxus build one. Any other widget can ignore this: its input
    /// shows the store's value, which `reset` has already put back.
    ///
    /// **A `key` is easy to put where Dioxus ignores it.** Dioxus reads one only
    /// on a node it diffs among siblings: an iterator's child, or the first
    /// root of an `rsx!` whose `Element` is placed with `{…}`. On an element
    /// nested inside a template, it does nothing. A loop of one always works,
    /// and makes only the input again:
    ///
    /// ```ignore
    /// let resets = field.resets();
    /// rsx! {
    ///     for n in [resets] {
    ///         input { key: "{n}", r#type: "file", onchange: field.on_pick_file() }
    ///     }
    /// }
    /// ```
    pub fn resets(&self) -> u32 {
        *self.form.contents.resets.read()
    }
}

impl BoundField<Set> {
    /// Add an item to the set: `value`, or with `None` the set's draft.
    pub fn add(&self, value: Option<ItemValue>) {
        self.send_edit(Edit::AddItem {
            path: self.path.clone(),
            value,
            // A set is sorted, so there is nowhere to insert before.
            before: None,
        });
    }

    /// Remove the item at `index`, its place in the set as sorted now.
    pub fn remove(&self, index: usize) {
        self.send_edit(Edit::RemoveItem {
            path: self.path.clone(),
            index,
        });
    }
}
