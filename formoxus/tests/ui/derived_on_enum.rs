//! Our message, not syn's generic "expected `struct`".
use formoxus::Form;

#[derive(Form)]
enum Question {
    TrueFalse,
}

fn main() {}
