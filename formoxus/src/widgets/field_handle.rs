use dioxus::core::Callback;
use dioxus::prelude::{FocusEvent, FormEvent};
use dioxus::signals::{ReadableExt, WritableExt};

use crate::nodes::ValuesStore;
use crate::path::NodePath;

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
pub struct FieldHandle {
    path: NodePath,
    values: ValuesStore,
    handler: OnFieldEvent,
}

impl FieldHandle {
    pub(crate) fn new(path: &NodePath, values: ValuesStore, handler: OnFieldEvent) -> Self {
        Self {
            path: path.clone(),
            values,
            handler,
        }
    }

    /// A handle whose writes the form never hears of: no live check follows
    /// one, and a clean result is not made stale. For the form's own writes
    /// in `absorb`, `reset` and `apply_edit`, each of which settles the
    /// field's errors itself. Never for a widget, which would then go
    /// unchecked until submit.
    pub(crate) fn unreported(path: &NodePath, values: ValuesStore) -> Self {
        Self {
            path: path.clone(),
            values,
            handler: OnFieldEvent::unreported(),
        }
    }

    /// The field's raw value, or `""` if it has none yet. Reading it while
    /// rendering subscribes the reader to this one field.
    ///
    /// **No value is normal, not an error.** The store can lack a key the
    /// form renders: a variant chosen after mount reveals fields that were
    /// never given a value. An absent key reads as `""`, the same "empty IS
    /// absence" rule the form applies when it validates.
    pub fn current(&self) -> String {
        let slot = self.values.get_unchecked(self.path.clone());
        slot.try_read().map(|v| v.clone()).unwrap_or_default()
    }

    /// The user typed: write `raw`, then report it. A field that shows an
    /// error is checked again at once, so the error goes as soon as it is
    /// fixed; any other field waits for [`blur`](Self::blur).
    pub fn input(&self, raw: String) {
        self.write_value(raw);
        self.handler.send(&self.path, FieldEventKind::Written);
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
        self.write_value(raw);
        self.handler.send(&self.path, FieldEventKind::Written);
        self.handler.send(&self.path, FieldEventKind::Blurred);
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
        self.handler.send(&self.path, FieldEventKind::Blurred);
    }

    /// [`blur`](Self::blur) as an `onblur` handler, to go with
    /// [`on_input`](Self::on_input).
    pub fn on_blur(&self) -> impl FnMut(FocusEvent) + 'static {
        let handle = self.clone();
        move |_| handle.blur()
    }

    /// Put `raw` in the store, and nothing else: reporting it is the caller's
    /// job, which is why this is private and [`input`](Self::input) and
    /// [`change`](Self::change) are not.
    ///
    /// **`set` when the key is there, `insert` only when it is not.** Both wake
    /// this field's own readers, but `insert` also marks the map's keys as
    /// changed (dioxus-stores 0.7.10, `HashMap` `insert`). That wakes anything
    /// that read the whole map, as a custom adder counting its list's entries
    /// does, on every keystroke. Only a new key changes the keys.
    fn write_value(&self, raw: String) {
        let mut values = self.values;
        let populated = values.peek().contains_key(&self.path);
        if populated {
            values.get_unchecked(self.path.clone()).set(raw);
        } else {
            values.insert(self.path.clone(), raw);
        }
    }
}
