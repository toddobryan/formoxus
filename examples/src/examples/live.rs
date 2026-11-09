//! A field checked as the user goes (`live_validation:`, on by default), and a
//! server's verdict that a local check leaves alone.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Account {
    username: String,
    password: String,
    agreed: bool,
}

fn spec() -> FormSpec<Account> {
    form! {
        Account {
            title: "Create an account",
            // So that what shows is formoxus's own errors, as the user goes,
            // rather than the browser's tooltip on submit.
            browser_validation: off,
            username => { min_length: 3, help_text: "Try \"admin\" to see a server's answer." },
            password => { widget: password, min_length: 8, help_text: "At least 8 characters." },
            agreed => { label: "I agree to the terms", required_true },
            buttons: {
                create: { type: submit, text: "Create account" },
            }
        }
    }
}

#[component]
pub fn LiveSignup() -> Element {
    let mut created = use_signal(|| None::<String>);
    let form = use_form(|| empty_form(spec()));

    rsx! {
        {form.render(using_fns! {
            create: move |model: Account| async move {
                // Standing in for a server that knows something the form
                // cannot: the name is taken. Its error stays until the user
                // changes the field, however often they click in and out.
                if model.username == "admin" {
                    form.push_field_error(&path!(Account.username), "That username is taken.")
                        .expect("username is a field of this form");
                    created.set(None);
                } else {
                    created.set(Some(model.username));
                }
            },
        })}

        if let Some(name) = created() {
            article { "Created " code { "{name}" } "." }
        }
    }
}
