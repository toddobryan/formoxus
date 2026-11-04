//! A field's `validator:` takes the field's own value, a `u8` here, so a
//! check over another type is a compile error, at the check, not a panic at
//! run time. A number's check takes it by value, so `&u8` is wrong too.
use facet::Facet;
use formoxus::{ValidationMessage, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    age: u8,
}

fn not_empty(name: &str) -> Vec<ValidationMessage> {
    if name.is_empty() { vec!["Required.".into()] } else { Vec::new() }
}

fn must_be_adult(age: &u8) -> Vec<ValidationMessage> {
    if *age < 18 { vec!["Adults only.".into()] } else { Vec::new() }
}

fn main() {
    let _ = form! { Signup { age => { validator: not_empty } } };
    let _ = form! { Signup { age => { validator: must_be_adult } } };
}
