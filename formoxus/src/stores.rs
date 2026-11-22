use std::collections::HashMap;

use dioxus::html::FileData;
use dioxus::prelude::*;
use indexmap::IndexMap;

/// A read-only view of a form's live values: one raw string per leaf, keyed
/// by its path (`answers.#0.text`). From [`Form::values`](crate::form::Form::values),
/// or, for a widget that shows more than its own value, its bound field's
/// [`values`](crate::bound_field::BoundField::values).
///
/// **Read-only on purpose** (#22). A value is written through the form,
/// [`Form::write`](crate::form::Form::write), or through a widget's bound
/// field, and both tell the form, which is what checks the field. A write
/// straight into the store would skip that, so this view cannot make one.
///
/// Each read subscribes the reader to as little as it can: [`get`](Self::get)
/// to one key, [`peek`](Self::peek) to nothing, and
/// [`paths_under`](Self::paths_under) to which keys exist. `Copy`, because it
/// is a handle on the store, not a copy of the values.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ValuesView(pub(crate) ValuesStore);

impl ValuesView {
    /// The raw value at `path`, or `""` if there is none: empty IS absence,
    /// as when the form validates. Reading it while rendering subscribes the
    /// reader to `path` alone, so a write to another field does not wake it.
    pub fn get(&self, path: &NodePath) -> String {
        let slot = self.0.get_unchecked(path.clone());
        slot.try_read().map(|v| v.clone()).unwrap_or_default()
    }

    /// [`get`](Self::get) without subscribing: for an event handler, or a
    /// read that should not re-render its reader when the value changes.
    pub fn peek(&self, path: &NodePath) -> String {
        self.0.peek().get(path).cloned().unwrap_or_default()
    }

    /// Every path in the store at or under `path`, compared segment by
    /// segment, so `answers_note` is not under `answers`. The root gives every
    /// path. In no particular order: the store is a hash map.
    ///
    /// Subscribes the reader to which keys exist, not to their values: it
    /// re-renders when a key is added or removed, as when an item is, and
    /// not when a value changes. So an adder that counts its list's entries
    /// is not re-rendered on every keystroke in an item.
    pub fn paths_under(&self, path: &NodePath) -> Vec<NodePath> {
        self.0
            .iter()
            .map(|(key, _)| key)
            .filter(|p| p.starts_with(path))
            .collect()
    }
}

/// A read-only view of a form's field errors, keyed by path. From
/// [`Form::errors`](crate::form::Form::errors), or a bound field's
/// [`errors`](crate::bound_field::BoundField::errors).
///
/// **Read-only on purpose** (#22): an error comes from a check, or from a
/// server's reply through [`Form::absorb`](crate::form::Form::absorb) or
/// [`Form::distribute_errors`](crate::form::Form::distribute_errors), never
/// from a direct write. Like [`ValuesView`], `Copy`, and each read
/// subscribes the reader to one path's errors only.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ErrorsView(pub(crate) ErrorsStore);

impl ErrorsView {
    /// The errors at `path`, in three states: `None` if nothing has checked
    /// it, an empty `Vec` if it was checked and passed, and its messages if
    /// it failed. Reading it while rendering subscribes the reader to `path`
    /// alone.
    pub fn at(&self, path: &NodePath) -> Option<Vec<ValidationMessage>> {
        self.0.get_unchecked(path.clone()).try_read().ok()?.cloned()
    }

    /// [`at`](Self::at)'s three states as a [`Validity`]: `Unchecked`,
    /// `Valid` or `Invalid`. What a bound field's
    /// [`validity`](crate::bound_field::BoundField::validity) reads for its
    /// own path.
    pub fn validity_at(&self, path: &NodePath) -> Validity {
        Validity::of(self.at(path).as_deref())
    }
}

use crate::{error::ValidationMessage, file::FileMeta, path::NodePath, widgets::Validity};

/// The live raw values, keyed by qualified path — what `leaves()` produces
/// and what `apply()` consumes.
pub type ValuesByPath = IndexMap<NodePath, String>;

/// The files picked or received, keyed by the file leaf's qualified path: the
/// size and content type of each, which the store (one string per path) can't
/// hold. What [`Node::distribute_files`](crate::nodes::Node::distribute_files) takes; always the whole table.
pub type FilesByPath = IndexMap<NodePath, FileMeta>;

/// A form's field errors, one entry per path. An empty `Vec` means the path was
/// checked and is clean, which is not the same as no entry, where nothing has
/// checked it. `IndexMap` over a plain `HashMap` for the same reason
/// `FormSpec`'s own fields use it: insertion order is preserved, and
/// `collect_errors` deliberately builds this in the same order `render` walks
/// the tree.
pub type ErrorsByPath = IndexMap<NodePath, Vec<ValidationMessage>>;

/// A [`std::collections::HashMap`] equivalent to [`ValuesByPath`] held in
/// a store so a write touches one input. The two maps are different,
/// because [`ValuesByPath`] maintains insertion order, but
/// [`struct@dioxus_stores::Store`] doesn't support [`indexmap::IndexMap`], just
/// [`std::collections::HashMap`] and a few others.
pub(crate) type ValuesStore = Store<HashMap<NodePath, String>>;

/// The form's file table: each file the user picked, keyed by its file
/// leaf's qualified path. Only [`BoundField::pick_file`] writes it, and
/// [`BoundField::picked_file`] reads one entry.
///
/// One `Signal`, not a `Store`, so a pick re-renders every file widget in
/// the form (`WIDGETS_PLAN.md` decision KK). A store's `remove` and `clear` wake
/// only readers of the map's shape, not of the removed key, so per-widget
/// re-rendering would need `Option` values set to `None` instead. Crate-
/// private, so that choice can change without breaking anyone.
///
/// [`BoundField::pick_file`]: crate::bound_field::BoundField::pick_file
/// [`BoundField::picked_file`]: crate::bound_field::BoundField::picked_file
pub(crate) type FilesSignal = Signal<IndexMap<NodePath, FileData>>;

/// Client home for field errors. Held in a store so that an error on one
/// field only redraws that field, not the whole page. This is a three-way
/// distinction, for each path:
///
/// - `None`, or a missing key, means that path was never checked or was cleared.
/// - `Some(vec![])` means that path was valid.
/// - `Some(vec![...])` means that path did not validate.
pub(crate) type ErrorsStore = Store<HashMap<NodePath, Option<Vec<ValidationMessage>>>>;
