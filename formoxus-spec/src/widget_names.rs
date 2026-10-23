//! The names `form!` accepts after `widget:` and in a `classes_plus:`
//! selector, and the type each one names.
//!
//! Data the macro needs at expansion time and cannot get any other way: the
//! widget TYPES live in `formoxus`, which `formoxus-macros` cannot depend on.
//! Everything else about a widget (what it renders, which parts it has) the
//! macro leaves to rustc, emitting `const _` asserts on the type's consts.
//!
//! The names are a stable surface, flat and lowercase on purpose. HTML's
//! spelling where HTML has one, with `_` for a hyphen (`tel`,
//! `datetime_local`); formoxus's own otherwise (`textarea`, `radio_group`).

use crate::attrs::str_eq;

/// The enum, its `ALL`, and both names, from one row per widget:
/// `Variant => "form_name"`. The variant is the widget's type name in
/// `formoxus::widgets`, so `type_name` is `stringify!` of it and the two
/// cannot disagree.
macro_rules! widget_names {
    ($( $(#[$doc:meta])* $variant:ident => $form_name:literal, )+) => {
        /// One built-in widget that renders, by name: an enum so that a
        /// `match` on it is exhaustive and a misspelling does not compile.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum WidgetName {
            $( $(#[$doc])* $variant, )+
        }

        impl WidgetName {
            /// Every built-in widget that renders, in table order: the order
            /// "unknown widget … expected one of" lists them in.
            pub const ALL: &[Self] = &[ $( Self::$variant, )+ ];

            /// What a dev writes after `widget:` in `form!`, and the widget's
            /// `Widget::NAME`.
            pub const fn form_name(self) -> &'static str {
                match self { $( Self::$variant => $form_name, )+ }
            }

            /// The type's name in [`WidgetName::MODULE`], which `form!` emits
            /// after it: `::formoxus::widgets::<type_name>`. `formoxus` tests that each
            /// type's `NAME` is its row's `form_name`, so a wrong row cannot
            /// make `widget: tel` render some other widget.
            pub const fn type_name(self) -> &'static str {
                match self { $( Self::$variant => stringify!($variant), )+ }
            }
        }
    };
}

widget_names! {
    TextInput => "text",
    PasswordInput => "password",
    HiddenInput => "hidden",
    NumberInput => "number",
    EmailInput => "email",
    TelInput => "tel",
    UrlInput => "url",
    SearchInput => "search",
    ColorInput => "color",
    DateInput => "date",
    TimeInput => "time",
    DatetimeLocalInput => "datetime_local",
    MonthInput => "month",
    WeekInput => "week",
    CheckboxInput => "checkbox",
    Textarea => "textarea",
    Select => "select",
    RadioGroup => "radio_group",
}

impl WidgetName {
    /// Where every built-in widget type is reachable from a dev's crate, as
    /// path segments: `form!` emits `::formoxus::widgets::TelInput` as
    /// `#( ::#segment )* :: #type_name`, one ident per segment. Segments,
    /// not one `"formoxus::widgets"`, because `format_ident!` makes a single
    /// ident and panics on `::`. Moving the widgets means changing this and
    /// nothing else in the macro.
    pub const MODULE: &[&str] = &["formoxus", "widgets"];

    /// Names reserved for widgets nothing renders yet. `form!` refuses them
    /// with "not supported yet" rather than "unknown widget", since they are
    /// coming. Not variants: every exhaustive `match` would need an arm for
    /// a widget that cannot reach it.
    pub const NOT_YET: &[&str] = &["select_multiple", "checkbox_multiple", "file"];

    /// The widget `form!` calls `form_name`, or `None` if no rendered widget
    /// is called that (check [`Self::NOT_YET`] for why not).
    ///
    /// A loop with `str_eq` rather than a `match` or an iterator: a `const fn`
    /// can do neither on a `&str`.
    pub const fn from_form_name(form_name: &str) -> Option<Self> {
        let mut i = 0;
        while i < Self::ALL.len() {
            if str_eq(form_name, Self::ALL[i].form_name()) {
                return Some(Self::ALL[i]);
            }
            i += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetName;
    use crate::BuiltinWidget;
    use googletest::prelude::*;
    use std::collections::HashSet;

    #[gtest]
    fn the_lookup_finds_every_widget_and_nothing_else() {
        for &w in WidgetName::ALL {
            expect_that!(WidgetName::from_form_name(w.form_name()), some(eq(w)));
        }
        expect_that!(WidgetName::from_form_name("Text"), none());
        expect_that!(WidgetName::from_form_name("TextInput"), none());
        expect_that!(WidgetName::from_form_name(""), none());
    }

    #[gtest]
    fn not_yet_names_are_not_widgets() {
        for name in WidgetName::NOT_YET {
            expect_that!(WidgetName::from_form_name(name), none(), "{name}");
        }
    }

    #[gtest]
    fn form_names_are_unique() {
        let names: HashSet<_> = WidgetName::ALL.iter().map(|w| w.form_name()).collect();
        expect_that!(names.len(), eq(WidgetName::ALL.len()));
    }

    #[gtest]
    fn the_type_name_is_the_variant() {
        expect_that!(WidgetName::TelInput.type_name(), eq("TelInput"));
        expect_that!(WidgetName::RadioGroup.type_name(), eq("RadioGroup"));
    }

    /// `form!` names have to be idents, and lowercase ones, or `form!` reads
    /// them as a path to a dev's own widget.
    #[gtest]
    fn every_name_is_a_lowercase_ident() {
        let names = WidgetName::ALL
            .iter()
            .map(|w| w.form_name())
            .chain(WidgetName::NOT_YET.iter().copied());
        for name in names {
            expect_that!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                eq(true),
                "{name}"
            );
        }
    }

    /// While `BuiltinWidget` still exists: `ALL` is its rendered widgets'
    /// names, in its order, and `NOT_YET` its others. Goes when
    /// `BuiltinWidget` does.
    #[gtest]
    fn the_names_agree_with_builtin_widget() {
        let rendered: Vec<&str> = BuiltinWidget::ALL
            .iter()
            .filter(|w| w.is_rendered())
            .map(BuiltinWidget::form_name)
            .collect();
        let not_yet: Vec<&str> = BuiltinWidget::ALL
            .iter()
            .filter(|w| !w.is_rendered())
            .map(BuiltinWidget::form_name)
            .collect();
        let names: Vec<&str> = WidgetName::ALL.iter().map(|w| w.form_name()).collect();
        expect_that!(names, eq(&rendered));
        expect_that!(WidgetName::NOT_YET.to_vec(), eq(&not_yet));
    }
}
