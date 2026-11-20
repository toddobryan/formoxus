use std::collections::HashMap;

use dioxus::html::FileData;
use dioxus::prelude::*;
use indexmap::IndexMap;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ValuesView(pub(crate) ValuesStore);

impl ValuesView {
    pub fn get(&self, path: &NodePath) -> String {
        let slot = self.0.get_unchecked(path.clone());
        slot.try_read().map(|v| v.clone()).unwrap_or_default()
    }

    pub fn peek(&self, path: &NodePath) -> String {
        self.0.peek().get(path).cloned().unwrap_or_default()
    }

    pub fn paths_under(&self, path: &NodePath) -> Vec<NodePath> {
        self.0
            .read()
            .keys()
            .filter(|p| p.starts_with(path))
            .cloned()
            .collect()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ErrorsView(pub(crate) ErrorsStore);

impl ErrorsView {
    pub fn at(&self, path: &NodePath) -> Option<Vec<ValidationMessage>> {
        self.0.get_unchecked(path.clone()).try_read().ok()?.cloned()
    }

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
/// leaf's qualified path. Only [`FieldHandle::pick_file`] writes it, and
/// [`FieldHandle::picked_file`] reads one entry.
///
/// One `Signal`, not a `Store`, so a pick re-renders every file widget in
/// the form (`WIDGETS_PLAN.md` decision KK). A store's `remove` and `clear` wake
/// only readers of the map's shape, not of the removed key, so per-widget
/// re-rendering would need `Option` values set to `None` instead. Crate-
/// private, so that choice can change without breaking anyone.
///
/// [`FieldHandle::pick_file`]: crate::widgets::FieldHandle::pick_file
/// [`FieldHandle::picked_file`]: crate::widgets::FieldHandle::picked_file
pub(crate) type FilesSignal = Signal<IndexMap<NodePath, FileData>>;

/// Client home for field errors. Held in a store so that an error on one
/// field only redraws that field, not the whole page. This is a three-way
/// distinction, for each path:
///
/// - `None`, or a missing key, means that path was never checked or was cleared.
/// - `Some(vec![])` means that path was valid.
/// - `Some(vec![...])` means that path did not validate.
pub(crate) type ErrorsStore = Store<HashMap<NodePath, Option<Vec<ValidationMessage>>>>;
