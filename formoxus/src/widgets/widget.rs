//! The widget traits, which built-in and custom widgets alike implement, and
//! `AnyWidget`, the form every holder stores one in.
//!
//! **A family, split by who reads what** (`WIDGETS_PLAN.md` decision G′).
//! [`Widget`] is what every widget has, whatever it renders: a name, its parts
//! and the attributes it passes through, which are all the class system and
//! `form!`'s attribute checks read. A second trait says what it renders:
//! [`LeafWidget`] for one value, which is every built-in today. A widget on a
//! whole set is to be the next, so that nothing meaningless on one kind is
//! required of the other.

use std::{any::Any, fmt::Debug};

// Dioxus's `Element`, what `render` returns. The attribute table's is
// `formoxus_spec::HtmlElement`, named apart so the two never need an alias.
use dioxus::prelude::Element;
use formoxus_spec::{AttrSet, FieldTypeSet, Tag};

use crate::field_attrs::FieldAttrs;
use crate::widgets::SetWidgetProps;
use crate::widgets::widget_props::WidgetProps;

/// What every widget is, whatever it renders: its [`NAME`](Self::NAME), its
/// [`PARTS`](Self::PARTS) and what it [`EXTENDS`](Self::EXTENDS). A widget
/// also implements the trait for what it renders, which today is always
/// [`LeafWidget`]; a type with only this one renders nothing.
///
/// A widget is a struct whose `pub` fields are its props, the way a
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

    /// A list of the HTML tags in this widget that classes can be attached to.
    const PARTS: &'static [Tag];

    /// The HTML attributes this widget passes through: the ones `form!`
    /// accepts on a field it renders, beyond the field's own settings. Read
    /// only at compile time, by `form!`'s checks.
    const EXTENDS: AttrSet;
}

/// A widget that renders one leaf: a single value, handed to it as one
/// [`WidgetProps`], read and written through its
/// [`FieldHandle`](crate::widgets::FieldHandle). Every built-in is one.
///
/// What `form!`'s type checks read (`ACCEPTS` and the two after it, which is
/// why they are consts), and what an [`AnyWidget`] holds. A widget of your
/// own implements both traits:
///
/// ```ignore
/// impl Widget for Stars {
///     const NAME: &'static str = "stars";
///     const PARTS: &'static [Tag] = &[Tag::Div, Tag::Label, Tag::Input];
///     const EXTENDS: AttrSet = AttrSet::EMPTY;
/// }
///
/// impl LeafWidget for Stars {
///     const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Int);
///     fn render(&self, props: WidgetProps) -> Element { /* … */ }
/// }
/// ```
pub trait LeafWidget: Widget {
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

    /// The raw value a field starts with when it would otherwise be empty, for
    /// a widget that cannot SHOW empty. `None`, the default, for every widget
    /// that can.
    ///
    /// A range is the one built-in that cannot: given `value=""`, the browser
    /// draws its midpoint, so the store would hold `""` while the user saw 50.
    /// Asked once, when the field's node is built (`LeafArgs::node`), so
    /// `empty_form`, `reset` and a server's rebuild all start from the same
    /// value, and the field parses it as it would anything typed.
    ///
    /// `attrs` are what the widget renders from: the field's own (`min`,
    /// `max`) then the spec's (`step`), bounds resolved to the field's type.
    fn value_when_empty(&self, _attrs: &FieldAttrs) -> Option<String> {
        None
    }
}

/// `LeafWidget` without its consts, which is what lets it be `dyn`; the blanket
/// impl below makes every `LeafWidget` one. Only `AnyWidget` holds one, so no dev
/// ever names it.
pub(crate) trait DynWidget: Debug + Send + Sync {
    fn render(&self, props: WidgetProps) -> Element;
    fn name(&self) -> &'static str;
    fn extends(&self) -> AttrSet;
    fn value_when_empty(&self, attrs: &FieldAttrs) -> Option<String>;
    fn clone_box(&self) -> Box<dyn DynWidget>;
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn DynWidget) -> bool;
}

impl<LW: LeafWidget> DynWidget for LW {
    fn render(&self, props: WidgetProps) -> Element {
        LW::render(self, props)
    }
    fn name(&self) -> &'static str {
        LW::NAME
    }
    fn extends(&self) -> AttrSet {
        LW::EXTENDS
    }
    fn value_when_empty(&self, attrs: &FieldAttrs) -> Option<String> {
        LW::value_when_empty(self, attrs)
    }
    fn clone_box(&self) -> Box<dyn DynWidget> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_dyn(&self, other: &dyn DynWidget) -> bool {
        other.as_any().downcast_ref::<LW>() == Some(self)
    }
}

/// Any [`LeafWidget`], boxed: what a field node stores, so that one form can hold
/// widgets of many types.
///
/// `Clone` and `PartialEq` are written by hand, since neither can be derived
/// over a `Box<dyn …>`. Two `AnyWidget`s are equal when they hold the same
/// widget type with equal props; two different types never are.
#[derive(Debug)]
pub struct AnyWidget(Box<dyn DynWidget>);

impl AnyWidget {
    pub fn new(widget: impl LeafWidget) -> Self {
        Self(Box::new(widget))
    }
    pub fn render(&self, props: WidgetProps) -> Element {
        self.0.render(props)
    }
    pub fn name(&self) -> &'static str {
        self.0.name()
    }
    /// The widget's [`Widget::EXTENDS`]: the attributes its element takes.
    pub fn extends(&self) -> AttrSet {
        self.0.extends()
    }
    /// The widget's [`LeafWidget::value_when_empty`].
    pub fn value_when_empty(&self, attrs: &FieldAttrs) -> Option<String> {
        self.0.value_when_empty(attrs)
    }
}
impl<LW: LeafWidget> From<LW> for AnyWidget {
    fn from(w: LW) -> Self {
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

pub trait SetWidget: Widget {
    fn render(&self, props: SetWidgetProps) -> Element;
}

/// `SetWidget` without its consts, so it can be `dyn`; the blanket impl makes
/// every `SetWidget` one. Only `AnySetWidget` holds one.
pub(crate) trait DynSetWidget: Debug + Send + Sync {
    fn render(&self, props: SetWidgetProps) -> Element;
    fn name(&self) -> &'static str;
    fn clone_box(&self) -> Box<dyn DynSetWidget>;
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn DynSetWidget) -> bool;
}

impl<SW: SetWidget> DynSetWidget for SW {
    fn render(&self, props: SetWidgetProps) -> Element {
        SW::render(self, props)
    }
    fn name(&self) -> &'static str {
        SW::NAME
    }
    fn clone_box(&self) -> Box<dyn DynSetWidget> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_dyn(&self, other: &dyn DynSetWidget) -> bool {
        other.as_any().downcast_ref::<SW>() == Some(self)
    }
}

#[derive(Debug)]
pub struct AnySetWidget(Box<dyn DynSetWidget>);

impl AnySetWidget {
    pub fn new(widget: impl SetWidget) -> Self {
        Self(Box::new(widget))
    }
    pub fn render(&self, props: SetWidgetProps) -> Element {
        self.0.render(props)
    }
    pub fn name(&self) -> &'static str {
        self.0.name()
    }
}

impl Clone for AnySetWidget {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

impl PartialEq for AnySetWidget {
    fn eq(&self, o: &Self) -> bool {
        self.0.eq_dyn(&*o.0)
    }
}

impl<SW: SetWidget> From<SW> for AnySetWidget {
    fn from(value: SW) -> Self {
        Self(Box::new(value))
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
        const PARTS: &'static [Tag] = &[Tag::Div, Tag::Textarea];
        const EXTENDS: AttrSet = AttrSet::EMPTY;
    }

    impl LeafWidget for Notes {
        fn render(&self, _props: WidgetProps) -> Element {
            rsx! {}
        }
    }

    impl Widget for Markdown {
        const NAME: &'static str = "markdown";
        const PARTS: &'static [Tag] = &[];
        const EXTENDS: AttrSet = AttrSet::EMPTY;
    }

    impl LeafWidget for Markdown {
        fn render(&self, _props: WidgetProps) -> Element {
            rsx! {}
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
    fn the_name_reaches_through_any_widget() {
        expect_that!(notes(4).name(), eq("notes"));
        expect_that!(AnyWidget::from(Markdown::default()).name(), eq("markdown"));
    }

    /// `AnyWidget::new` and `From` build the same thing.
    #[gtest]
    fn new_and_into_agree() {
        expect_that!(AnyWidget::new(Notes { rows: 4 }), eq(&notes(4)));
    }

    /// `AnySetWidget` is built as `AnyWidget` is, over its own `DynSetWidget`,
    /// so it gets the same tests, over two toy set widgets that again differ
    /// only in type.
    #[derive(Clone, Debug, Default, PartialEq)]
    struct Ticks {
        columns: u32,
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    struct Picks {
        columns: u32,
    }

    impl Widget for Ticks {
        const NAME: &'static str = "ticks";
        const PARTS: &'static [Tag] = &[Tag::Fieldset, Tag::Legend, Tag::Input, Tag::Label];
        const EXTENDS: AttrSet = AttrSet::EMPTY;
    }

    impl SetWidget for Ticks {
        fn render(&self, _props: SetWidgetProps) -> Element {
            rsx! {}
        }
    }

    impl Widget for Picks {
        const NAME: &'static str = "picks";
        const PARTS: &'static [Tag] = &[];
        const EXTENDS: AttrSet = AttrSet::EMPTY;
    }

    impl SetWidget for Picks {
        fn render(&self, _props: SetWidgetProps) -> Element {
            rsx! {}
        }
    }

    fn ticks(columns: u32) -> AnySetWidget {
        Ticks { columns }.into()
    }

    #[gtest]
    fn set_widgets_of_one_type_with_equal_props_are_equal() {
        expect_that!(ticks(2), eq(&ticks(2)));
    }

    #[gtest]
    fn set_widgets_of_one_type_with_different_props_are_not() {
        expect_that!(ticks(2), not(eq(&ticks(3))));
    }

    #[gtest]
    fn set_widgets_of_different_types_are_never_equal() {
        let picks: AnySetWidget = Picks { columns: 2 }.into();
        expect_that!(ticks(2), not(eq(&picks)));
        expect_that!(picks, not(eq(&ticks(2))));
    }

    #[gtest]
    fn a_set_widget_clone_is_equal_and_keeps_its_type() {
        let original = ticks(2);
        let copy = original.clone();
        expect_that!(copy, eq(&original));
        expect_that!(copy.name(), eq("ticks"));
    }

    #[gtest]
    fn the_name_reaches_through_any_set_widget() {
        expect_that!(ticks(2).name(), eq("ticks"));
        expect_that!(AnySetWidget::from(Picks::default()).name(), eq("picks"));
    }

    #[gtest]
    fn any_set_widget_new_and_into_agree() {
        expect_that!(AnySetWidget::new(Ticks { columns: 2 }), eq(&ticks(2)));
    }
}
