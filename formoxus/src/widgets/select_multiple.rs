//! `<select multiple>` over a set's choices.

use dioxus::prelude::*;
use formoxus_spec::{AttrSet, HtmlElement, Tag, WidgetName};

use crate::field_attrs::Validity;
use crate::nodes::{Edit, ItemValue};
use crate::path::NodePath;
use crate::widgets::field_row::{LabelSide, field_row};
use crate::widgets::{SetWidget, SetWidgetProps, Widget, described_by};

/// A `<select multiple>`, one option per choice, for a set: each selected
/// option is one item.
///
/// One control, so it is laid out as [`Select`](crate::widgets::Select) is:
/// a `<label>` names it, with the caption, the required marker and the set's
/// own errors in their usual places.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SelectMultiple;

impl Widget for SelectMultiple {
    const NAME: &'static str = WidgetName::SelectMultiple.form_name();

    const PARTS: &'static [Tag] = &[Tag::Div, Tag::Label, Tag::Select];

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Select);
}

impl SetWidget for SelectMultiple {
    fn render(&self, props: SetWidgetProps) -> Element {
        rsx! {
            SelectMultipleView { widget_props: props }
        }
    }
}

/// The edits that take a set from `chosen` to `now`: every entry that left, as
/// a `RemoveItem` at its index in `chosen`, HIGHEST index first, so removing
/// one never shifts the index of one still to go; then every entry that
/// arrived, as an `AddItem`. Adds come after removes, and an add has no index,
/// so nothing an add does can shift a remove.
///
/// One change can move several options at once (shift- or ctrl-click), which
/// is why the handler diffs rather than reading one option.
fn edits_between(path: &NodePath, chosen: &[String], now: &[String]) -> Vec<Edit> {
    let removes = chosen
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, entry)| !now.contains(entry))
        .map(|(index, _)| Edit::RemoveItem {
            path: path.clone(),
            index,
        });
    let adds = now
        .iter()
        .filter(|entry| !chosen.contains(entry))
        .map(|entry| Edit::AddItem {
            path: path.clone(),
            before: None,
            value: Some(ItemValue::raw(entry.clone())),
        });
    removes.chain(adds).collect()
}

#[component]
pub(crate) fn SelectMultipleView(widget_props: SetWidgetProps) -> Element {
    // Not `props`: Dioxus reads a component's lone `props` parameter as its
    // whole props struct.
    let props = widget_props;
    let at_max = props.max_items.is_some_and(|max| props.chosen.len() >= max);
    // A `<select multiple>` is role `listbox`, which DOES take `aria-invalid`,
    // unlike the checkbox group's bare fieldset; so it gets it as a leaf
    // control does: absent until checked, then "true" or "false".
    let aria_invalid = match props.validity() {
        Validity::Invalid => Some("true"),
        Validity::Valid => Some("false"),
        _ => None,
    };
    let path = props.path.clone();
    let chosen = props.chosen.clone();
    let on_edit = props.on_edit;
    let on_change = move |e: FormEvent| {
        // `values()`, not `value()`: in the browser `value()` is only the
        // first selected option. On a `<select>`, `values()` is one pair per
        // selected option, named by the select's `name`, which is `""` here
        // (no `name`, question Q), so the names are ignored.
        let now: Vec<String> = e
            .values()
            .into_iter()
            .filter_map(|(_, value)| match value {
                FormValue::Text(text) => Some(text),
                FormValue::File(_) => None,
            })
            .collect();
        for edit in edits_between(&path, &chosen, &now) {
            on_edit.call(edit);
        }
    };

    let control = rsx! {
        select {
            class: props.part_class::<SelectMultiple>(Tag::Select),
            id: "{props.id}",
            multiple: true,
            // On a `<select multiple>`, unlike a checkbox, `required` means
            // what `min_items >= 1` does: at least one option selected. The
            // browser then blocks an empty submit itself; a larger
            // `min_items` is still held by the set's validation.
            required: props.required,
            aria_invalid,
            aria_describedby: described_by(&props.id, props.help_text.is_some(), props.is_invalid()),
            onchange: on_change,
            for choice in props.choices.iter() {
                option {
                    value: "{choice.value}",
                    selected: props.chosen.contains(&choice.value),
                    // At the limit only the selected options stay live, as
                    // the Add button stops a list: the user can still
                    // deselect one and pick another.
                    disabled: at_max && !props.chosen.contains(&choice.value),
                    "{choice.display}"
                }
            }
        }
    };
    field_row::<SelectMultiple>(&props, props.required, LabelSide::Before, control)
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;

    use super::edits_between;
    use crate::nodes::{Edit, ItemValue};
    use crate::path::NodePath;

    fn entries(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    fn remove(index: usize) -> Edit {
        Edit::RemoveItem {
            path: NodePath::root().join("levels"),
            index,
        }
    }

    fn add(entry: &str) -> Edit {
        Edit::AddItem {
            path: NodePath::root().join("levels"),
            before: None,
            value: Some(ItemValue::raw(entry.to_string())),
        }
    }

    #[gtest]
    fn no_change_is_no_edits() {
        let path = NodePath::root().join("levels");
        let chosen = entries(&["Low", "High"]);
        expect_that!(edits_between(&path, &chosen, &chosen), is_empty());
    }

    #[gtest]
    fn one_new_option_is_one_add() {
        let path = NodePath::root().join("levels");
        expect_that!(
            edits_between(&path, &entries(&["Low"]), &entries(&["Low", "High"])),
            elements_are![eq(&add("High"))]
        );
    }

    #[gtest]
    fn several_removes_go_highest_index_first() {
        let path = NodePath::root().join("levels");
        expect_that!(
            edits_between(
                &path,
                &entries(&["Low", "Medium", "High"]),
                &entries(&["Medium"])
            ),
            elements_are![eq(&remove(2)), eq(&remove(0))]
        );
    }

    #[gtest]
    fn removes_come_before_adds() {
        let path = NodePath::root().join("levels");
        expect_that!(
            edits_between(&path, &entries(&["Low"]), &entries(&["High"])),
            elements_are![eq(&remove(0)), eq(&add("High"))]
        );
    }
}
