//! `[]` means "every row of this list", so it only makes sense on something
//! iterable — the same rule `form!` enforces, reached through `path!`.
use facet::Facet;
use formoxus::path;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Article {
    headline: String,
}

fn main() {
    let _ = path!(Article.headline[]);
}
