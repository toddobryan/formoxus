//! A field literally named `errors` collides with the form-level errors field
//! formoxus adds to every generated state struct — rejected as a clean darling
//! error at derive time, not left to surface as a `duplicate_field` error on
//! deserialize far from the mistake.
use formoxus::Form;

#[derive(Form)]
#[form(button(type = "submit", name = submit))]
struct Question {
    name: String,
    errors: String,
}

fn main() {}
