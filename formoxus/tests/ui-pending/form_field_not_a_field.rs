//! `field:` names something that is not a `Field`. The error should point at
//! the value after `field:`, and say a `Field` is wanted, not surface as a
//! trait-bound failure deep inside the expansion.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Lesson {
    snippet: String,
}

/// Looks like a field, implements nothing.
#[derive(Clone, Debug, PartialEq)]
struct SnippetChecker;

fn main() {
    let _ = empty_form::<Lesson>(form! {
        Lesson {
            snippet => { field: SnippetChecker },
        }
    });
}
