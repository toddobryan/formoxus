//! An unknown `#[form(...)]` key is a clean darling error, not a silently-ignored
//! attribute — guards the attribute surface (`model`, `validator`).
use formoxus::Form;

#[derive(Form)]
#[form(bogus = Whatever)]
struct Question {
    name: String,
}

fn main() {}
