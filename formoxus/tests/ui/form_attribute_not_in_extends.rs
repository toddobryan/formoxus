//! `rows` suits a String field, so the table's `for:` check passes, but a
//! `text` widget renders an `<input>`, which has no `rows`: the widget's
//! `EXTENDS` refuses it, at the key.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Survey {
    comments: String,
}

fn main() {
    let _ = empty_form::<Survey>(form! {
        Survey {
            comments => { widget: text, rows: 5 },
        }
    });
}
