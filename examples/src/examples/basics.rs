//! The scaffold's own proof: if this renders and the buttons respond, the
//! gallery is wired up correctly and a new example only has to add itself.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
    password: String,
    bio: String,
    agreed: bool,
}

fn spec() -> FormSpec<Signup> {
    form! {
        Signup {
            title: "Sign up",
            email => { widget: email },
            password => { widget: password },
            bio => { label: "About you", widget: textarea },
            agreed => { label: "I agree to the terms" },
            buttons: {
                create: { type: submit, text: "Create account" },
            }
        }
    }
}

#[component]
pub fn Basics() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| empty_form(spec()));

    rsx! {
        {form.render(using_fns! {
            // `|model|` — arity says this only runs once validation passes.
            create: move |model: Signup| async move {
                submitted.set(Some(format!("{model:?}")));
            },
        })}

        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}
