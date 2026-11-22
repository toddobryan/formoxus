//! `Form::errors()` is a read-only view (#22): errors come from a check, or
//! from a server's reply through `absorb` or `distribute_errors`.
use facet::Facet;
use formoxus::Form;
use formoxus::path::NodePath;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Contact {
    name: String,
}

fn an_error_nobody_checked(form: Form<Contact>) {
    form.errors().insert(NodePath::new("name"), None);
}

fn main() {}
