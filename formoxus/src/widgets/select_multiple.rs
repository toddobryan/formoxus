//! `<select multiple>` over a set's choices.

use dioxus::prelude::*;
use formoxus_spec::{AttrSet, HtmlElement, Tag, WidgetName};

use crate::field_attrs::Validity;
use crate::nodes::ItemValue;
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

/// One step from the set's items as they were to the options selected now:
/// what [`changes_between`] returns, and what the handler turns into a call on
/// the set's bound field. No path, because the bound field supplies its own.
#[derive(Clone, Debug, PartialEq)]
enum SetChange {
    /// Remove the item at this index in `chosen`.
    Remove(usize),
    /// Add this entry, a choice's value.
    Add(String),
}

/// The changes that take a set from `chosen` to `now`: every entry that left,
/// as a `Remove` at its index in `chosen`, HIGHEST index first, so removing
/// one never shifts the index of one still to go; then every entry that
/// arrived, as an `Add`. Adds come after removes, and an add has no index, so
/// nothing an add does can shift a remove.
///
/// One change can move several options at once (shift- or ctrl-click), which
/// is why the handler diffs rather than reading one option.
fn changes_between(chosen: &[String], now: &[String]) -> Vec<SetChange> {
    let removes = chosen
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, entry)| !now.contains(entry))
        .map(|(index, _)| SetChange::Remove(index));
    let adds = now
        .iter()
        .filter(|entry| !chosen.contains(entry))
        .map(|entry| SetChange::Add(entry.clone()));
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
    let field = props.field();
    let chosen = props.chosen.clone();
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
        for change in changes_between(&chosen, &now) {
            match change {
                SetChange::Remove(index) => field.remove(index),
                SetChange::Add(entry) => field.add(Some(ItemValue::raw(entry))),
            }
        }
    };

    let control = rsx! {
        select {
            class: props.part_class::<SelectMultiple>(Tag::Select),
            id: "{props.id()}",
            multiple: true,
            // On a `<select multiple>`, unlike a checkbox, `required` means
            // what `min_items >= 1` does: at least one option selected. The
            // browser then blocks an empty submit itself; a larger
            // `min_items` is still held by the set's validation.
            required: props.required,
            aria_invalid,
            aria_describedby: described_by(props.id(), props.help_text().is_some(), props.is_invalid()),
            onchange: on_change,
            for choice in props.choices.iter() {
                option {
                    value: "{choice.key}",
                    selected: props.chosen.contains(&choice.key),
                    // At the limit only the selected options stay live, as
                    // the Add button stops a list: the user can still
                    // deselect one and pick another.
                    disabled: at_max && !props.chosen.contains(&choice.key),
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

    use super::{SetChange, changes_between};

    fn entries(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    fn add(entry: &str) -> SetChange {
        SetChange::Add(entry.to_string())
    }

    #[gtest]
    fn no_change_is_no_changes() {
        let chosen = entries(&["Low", "High"]);
        expect_that!(changes_between(&chosen, &chosen), is_empty());
    }

    #[gtest]
    fn one_new_option_is_one_add() {
        expect_that!(
            changes_between(&entries(&["Low"]), &entries(&["Low", "High"])),
            elements_are![eq(&add("High"))]
        );
    }

    #[gtest]
    fn several_removes_go_highest_index_first() {
        expect_that!(
            changes_between(&entries(&["Low", "Medium", "High"]), &entries(&["Medium"])),
            elements_are![eq(&SetChange::Remove(2)), eq(&SetChange::Remove(0))]
        );
    }

    #[gtest]
    fn removes_come_before_adds() {
        expect_that!(
            changes_between(&entries(&["Low"]), &entries(&["High"])),
            elements_are![eq(&SetChange::Remove(0)), eq(&add("High"))]
        );
    }
}
