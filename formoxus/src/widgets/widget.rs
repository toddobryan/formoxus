//! The `Widget` trait, which built-in and custom widgets alike implement, and
//! `AnyWidget`, the form every holder stores one in.

use std::{any::Any, fmt::Debug};

// Dioxus's `Element`, what `render` returns. The attribute table's is
// `formoxus_spec::HtmlElement`, named apart so the two never need an alias.
use dioxus::prelude::Element;
use formoxus_spec::{AttrSet, FieldTypeSet};

use crate::{classes::Slot, widgets::widget_props::WidgetProps};

/// What renders a field: a struct whose `pub` fields are its props, the way a
/// Dioxus component's are. `form!` sets them in the widget's braces
/// (`widget: MarkdownWidget { preview: true }`) and fills in the rest from
/// `Default`, so rustc checks every prop's name and type.
///
/// A widget is held as an [`AnyWidget`], so the bounds are what that needs:
/// `Clone`, `Debug` and `PartialEq` so the spec holding it keeps deriving
/// them, `Send + Sync` so it can go wherever a spec goes, and `'static` for
/// the type check inside `AnyWidget`'s `==`.
pub trait Widget: Clone + Debug + PartialEq + Default + Send + Sync + 'static {
    /// Keys this widget's class slots in `Classes`. Unique per widget.
    const NAME: &'static str;

    /// The HTML attributes this widget passes through: the ones `form!`
    /// accepts on a field it renders, beyond the field's own settings. Read
    /// only at compile time, by `form!`'s checks.
    const EXTENDS: AttrSet;

    /// The field types this widget renders without `choices`. It renders a
    /// field of type `t` when `ACCEPTS.contains(t) || (TAKES_CHOICES &&
    /// has_choices)`: a `Select` accepts only `Bool` on its own, but any type
    /// it is given choices for. Read only at compile time, by `form!`'s
    /// checks, which is why it is a const and not a method: a `const fn` can
    /// read a trait's consts but cannot call its methods.
    ///
    /// Defaults to every type, so a widget that declares nothing is checked
    /// for nothing.
    const ACCEPTS: FieldTypeSet = FieldTypeSet::ALL;

    /// Whether this widget can render an `Option<_>` field, which needs a way
    /// to say "no value". `false` for a checkbox (two states, where an
    /// `Option<bool>` has three) and a radio group (a picked radio cannot be
    /// un-picked); `select`, with its blank option, is the way out for both.
    /// Defaults to `true`.
    const ACCEPTS_OPTIONAL: bool = true;

    /// Whether `form!` may give this widget `choices`, and so whether a field
    /// outside `ACCEPTS` is still renderable when it has them. Defaults to
    /// `true`: choices reach every widget through `WidgetProps`, and whether
    /// one uses them is its own business.
    const TAKES_CHOICES: bool = true;

    /// Renders the field from `props`, which are built fresh for each render
    /// and so are the widget's to move out of.
    ///
    /// **A plain method, not a component, so it must not call hooks.** It
    /// runs inside the component rendering the field, and a hook here would
    /// join that component's hooks. A widget that needs state renders a
    /// `#[component]` of its own and passes `props` on to it.
    fn render(&self, props: WidgetProps) -> Element;

    /// Each class slot this widget has, by name, with its default classes.
    /// A fn rather than a const because `Slot::with_invalid` is not `const`.
    fn default_classes() -> Vec<(&'static str, Slot)>;
}

/// `Widget` without its consts, which is what lets it be `dyn`; the blanket
/// impl below makes every `Widget` one. Only `AnyWidget` holds one, so no dev
/// ever names it.
pub(crate) trait DynWidget: Debug + Send + Sync {
    fn render(&self, props: WidgetProps) -> Element;
    fn name(&self) -> &'static str;
    fn default_classes(&self) -> Vec<(&'static str, Slot)>;
    fn clone_box(&self) -> Box<dyn DynWidget>;
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn DynWidget) -> bool;
}

impl<W: Widget> DynWidget for W {
    fn render(&self, props: WidgetProps) -> Element {
        Widget::render(self, props)
    }
    fn name(&self) -> &'static str {
        W::NAME
    }
    fn default_classes(&self) -> Vec<(&'static str, Slot)> {
        W::default_classes()
    }
    fn clone_box(&self) -> Box<dyn DynWidget> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_dyn(&self, other: &dyn DynWidget) -> bool {
        other.as_any().downcast_ref::<W>() == Some(self)
    }
}

/// Any [`Widget`], boxed: what a field node stores, so that one form can hold
/// widgets of many types.
///
/// `Clone` and `PartialEq` are written by hand, since neither can be derived
/// over a `Box<dyn …>`. Two `AnyWidget`s are equal when they hold the same
/// widget type with equal props; two different types never are.
#[derive(Debug)]
pub struct AnyWidget(Box<dyn DynWidget>);

impl AnyWidget {
    pub fn new(widget: impl Widget) -> Self {
        Self(Box::new(widget))
    }
    pub fn render(&self, props: WidgetProps) -> Element {
        self.0.render(props)
    }
    pub fn name(&self) -> &'static str {
        self.0.name()
    }
    pub fn default_classes(&self) -> Vec<(&'static str, Slot)> {
        self.0.default_classes()
    }
}
impl<W: Widget> From<W> for AnyWidget {
    fn from(w: W) -> Self {
        Self::new(w)
    }
}
impl Clone for AnyWidget {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}
impl PartialEq for AnyWidget {
    fn eq(&self, o: &Self) -> bool {
        self.0.eq_dyn(&*o.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;
    use googletest::prelude::*;

    /// Two toy widgets with the same props, so that only their TYPES differ:
    /// what tells `eq_dyn`'s downcast apart from a comparison of fields.
    #[derive(Clone, Debug, Default, PartialEq)]
    struct Notes {
        rows: u32,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    struct Markdown {
        rows: u32,
    }

    impl Widget for Notes {
        const NAME: &'static str = "notes";
        const EXTENDS: AttrSet = AttrSet::EMPTY;
        fn render(&self, _props: WidgetProps) -> Element {
            rsx! {}
        }
        fn default_classes() -> Vec<(&'static str, Slot)> {
            vec![
                ("div", Slot::new("notes").with_invalid("bad")),
                ("textarea", Slot::new("notes-box")),
            ]
        }
    }

    impl Widget for Markdown {
        const NAME: &'static str = "markdown";
        const EXTENDS: AttrSet = AttrSet::EMPTY;
        fn render(&self, _props: WidgetProps) -> Element {
            rsx! {}
        }
        fn default_classes() -> Vec<(&'static str, Slot)> {
            Vec::new()
        }
    }

    fn notes(rows: u32) -> AnyWidget {
        Notes { rows }.into()
    }

    #[gtest]
    fn widgets_of_one_type_with_equal_props_are_equal() {
        expect_that!(notes(4), eq(&notes(4)));
    }

    #[gtest]
    fn widgets_of_one_type_with_different_props_are_not() {
        expect_that!(notes(4), not(eq(&notes(5))));
    }

    /// Same props, different types: unequal, because `eq_dyn` downcasts to
    /// its own type first and only then compares props.
    #[gtest]
    fn widgets_of_different_types_are_never_equal() {
        let markdown: AnyWidget = Markdown { rows: 4 }.into();
        expect_that!(notes(4), not(eq(&markdown)));
        expect_that!(markdown, not(eq(&notes(4))));
    }

    #[gtest]
    fn a_clone_is_equal_and_keeps_its_type() {
        let original = notes(4);
        let copy = original.clone();
        expect_that!(copy, eq(&original));
        expect_that!(copy.name(), eq("notes"));
    }

    #[gtest]
    fn name_and_default_classes_reach_through_any_widget() {
        let widget = notes(4);
        expect_that!(widget.name(), eq("notes"));
        expect_that!(
            widget.default_classes(),
            elements_are![
                eq(&("div", Slot::new("notes").with_invalid("bad"))),
                eq(&("textarea", Slot::new("notes-box"))),
            ]
        );
        expect_that!(AnyWidget::from(Markdown::default()).name(), eq("markdown"));
    }

    /// `AnyWidget::new` and `From` build the same thing.
    #[gtest]
    fn new_and_into_agree() {
        expect_that!(AnyWidget::new(Notes { rows: 4 }), eq(&notes(4)));
    }
}
