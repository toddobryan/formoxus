//! Named fields are required; a tuple struct must surface our message, not a
//! `.expect()` panic ("proc macro panicked") from the ident extraction.
use formoxus::Form;

#[derive(Form)]
struct Question(String);

fn main() {}
