//! `Form`/`FormState`/`Model` require `Debug` (a derive can't add it to the
//! user's own struct), so a form struct without `#[derive(Debug)]` must fail with
//! a clear trait-bound error rather than compiling to something unloggable.
use formoxus::Form;

#[derive(Form)]
#[form(button(type = "submit", name = submit))]
struct NoDebug {
    name: String,
}

fn main() {}
