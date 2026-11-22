//! `Form::values()` is a read-only view (#22): a dev writes a value through
//! `Form::write`, and a widget through its `BoundField`, both of which tell
//! the form so it can check the field. Nothing writes the store directly.
use facet::Facet;
use formoxus::Form;
use formoxus::path::NodePath;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Contact {
    name: String,
}

fn write_behind_the_forms_back(form: Form<Contact>) {
    form.values().insert(NodePath::new("name"), "Ada".to_string());
}

fn main() {}
