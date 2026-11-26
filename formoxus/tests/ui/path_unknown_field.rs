//! The guarantee `ModelPath<T>` exists for: a misspelled field is a compile error at
//! the call site, not a `no_such_path` panic on a server. The witness borrow is
//! what rustc rejects — `path!` itself never sees the field's type.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, path};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
    password: String,
}

fn main() {
    let _ = path!(Signup.pasword);
}
