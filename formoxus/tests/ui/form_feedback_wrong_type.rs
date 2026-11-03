//! The event's item type is the set's: a closure over `u8` events does not
//! fit a set of `String`, and rustc says so at the closure.
use facet::Facet;
use formoxus::nodes::SetEvent;
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

fn main() {
    let _ = empty_form::<Post>(form! {
        Post {
            tags => { feedback: |_: SetEvent<'_, u8>| None },
        }
    });
}
