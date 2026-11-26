//! `[]` means "every item of this list", so it only makes sense on something
//! iterable — the same rule `form!` enforces, reached through `path!`.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, path};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Article {
    headline: String,
}

fn main() {
    let _ = path!(Article.headline[]);
}
