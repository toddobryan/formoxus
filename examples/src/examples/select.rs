use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    last_name: String,
    first_name: String,
    middle_initial: Option<String>,
    street_address: String,
    city: String,
    state: String,
    zip: String,
    telephone_number: String,
    email: String,
}

fn address() -> FormSpec<Address> {
    form! {
        Address {
            browser_validation: off,
            label_case: "label-case",
            state => {
                widget: select,
                //choices: STATES,
            },
            telephone_number => { widget: tel },
            email => { widget: email },
            // zip => { pattern: "^\d{5}(-\d{4})?"},
            buttons: {
                reset: { type: reset, },
                submit: { type: submit },
            }
        }
    }
}

#[component]
pub fn AddressForm() -> Element {
    let mut submitted = use_signal(|| None::<String>);
    let form = use_form(|| empty_form(address()));

    rsx! {
        {
            form.render(using_fns! {
                reset: move || async move { form.reset() },
                submit: move |address: Address| async move {
                    submitted.set(Some(format!("{address:?}")));
                },
            })
        }

        if let Some(text) = submitted() {
            article { "Validated model: " code { "{text}" } }
        }
    }
}
