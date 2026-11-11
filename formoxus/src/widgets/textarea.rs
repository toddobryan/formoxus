//! The `<textarea>`.

use dioxus::prelude::*;
use formoxus_spec::{Attr, AttrSet, FieldType, FieldTypeSet, HtmlElement, Tag, WidgetName};

use super::field_row::{LabelSide, field_row};
use crate::widgets::{Widget, widget::LeafWidget, widget_props::WidgetProps};

/// `<textarea>`, a multi-line text input. For a `String` field only.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Textarea;

impl Widget for Textarea {
    const NAME: &'static str = WidgetName::Textarea.form_name();

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Textarea);

    const PARTS: &'static [Tag] = &[Tag::Div, Tag::Label, Tag::Textarea];
}

impl LeafWidget for Textarea {
    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Text);

    const ACCEPTS_OPTIONAL: bool = true;

    const TAKES_CHOICES: bool = false;

    fn render(&self, props: WidgetProps) -> Element {
        rsx! {
            TextareaView { widget_props: props }
        }
    }
}

/// A multi-line text input bound to one path in the value map.
///
/// Otherwise identical to `InputView` — same controlled-value binding, same
/// `aria-invalid` rule, same label layout. There is no hidden-input branch to
/// mirror: `Textarea` is only ever chosen as an override on a `Text` field, and
/// nothing here needs the extra `InputType` cases (`Password`'s masking,
/// `Hidden`'s bare markup) that make `Input` carry one.
#[component]
pub(crate) fn TextareaView(widget_props: WidgetProps) -> Element {
    // Not `props`: Dioxus reads a component's lone `props` parameter as its
    // whole props struct, which `WidgetProps` is not.
    let props = widget_props;
    let handle = props.field_handle();
    let current = handle.current();
    let attrs: Vec<Attribute> = props
        .current_attrs()
        .merge_with_attrs(props.extra_attrs.clone());

    let control = rsx! {
        textarea {
            class: props.control_class::<Textarea>(Tag::Textarea),
            id: "{props.id}",
            name: "{props.path}",
            value: "{current}",
            // A textarea's text comes from its CONTENT, not a `value` attribute,
            // and hydration never re-applies attributes, so `value` alone
            // leaves a seeded textarea blank in a fullstack app. Not a text
            // child: with hydration on, SSR wraps each dynamic text node in
            // `<!--node-id…-->` comments, which a textarea, being raw text,
            // shows literally. `dangerous_inner_html` is written as bare
            // content. `value` stays for every client-side change (the draft
            // clearing, Edit, Cancel): once the user has typed, content
            // only moves the default value, not what is shown.
            dangerous_inner_html: escape_text(&current),
            oninput: handle.on_input(),
            onblur: handle.on_blur(),
            // TODO: delete pattern, since not allowed in HTML
            ..attrs,
        }
    };
    let marked = props.base_attrs.contains(Attr::Required);
    field_row::<Textarea>(&props, marked, LabelSide::Before, control)
}

/// `text` escaped for a textarea's content, where `&` would start a
/// character reference and `</textarea` would end the element.
///
/// Public for a custom widget that renders its own `<textarea>`: it needs the
/// same `dangerous_inner_html: escape_text(&current)` beside `value:` as
/// `TextareaView`, or a seeded value is blank after hydration (see there).
pub fn escape_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::escape_text;
    use googletest::prelude::*;

    #[gtest]
    fn escapes_what_could_end_the_textarea_or_start_a_reference() {
        expect_that!(
            escape_text("a < b && </textarea> c"),
            eq("a &lt; b &amp;&amp; &lt;/textarea&gt; c")
        );
    }

    #[gtest]
    fn leaves_quotes_and_newlines_alone() {
        expect_that!(escape_text("\"one\"\ntwo"), eq("\"one\"\ntwo"));
    }
}
