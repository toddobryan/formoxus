//! `[]` means "every row of this list", so it only makes sense on something
//! iterable. On a `String` the witness's `.iter()` is what catches it.
use facet::Facet;
use formoxus::form;
use formoxus::empty_form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Article {
    headline: String,
}

fn main() {
    let _ = empty_form::<Article>(form! {
        Article {
            headline[] => { control: textarea },
        }
    });
}
