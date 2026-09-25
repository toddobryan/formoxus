use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    last_name: String,
    first_name: String,
    middle_initial: Option<String>,
    street: String,
    city: String,
    state: String,
    zip: String,
    telephone_number: String,
    email: String,
    gender: String,
}

/// The choice list, as a plain `const` table.
///
/// `SelectChoice` holds `String`s, so a list of them cannot be `const` — hence
/// the pairs, and the `From<&(&str, &str)>` impl that lets them arrive as
/// choices anyway. The stored value is the postal code; the reader sees the
/// name.
const STATES: &[(&str, &str)] = &[
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("DC", "District of Columbia"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Iowa"),
    ("KS", "Kansas"),
    ("KY", "Kentucky"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachusetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nevada"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
];

/// A second choice list, for the radio group.
///
/// A `String` field rather than a three-variant `enum`, which is what it wants
/// to be: an enum field is a SHAPE choice, so it becomes a `VariantSet` with a
/// `<select>` over its variant names and cannot take a widget at all. A radio
/// group is a VALUE choice, and a value is a string on the wire. The stored
/// value is lowercase so it reads as data rather than as a label.
const GENDERS: &[(&str, &str)] = &[("male", "Male"), ("female", "Female"), ("other", "Other")];

fn address() -> FormSpec<Address> {
    form! {
        Address {
            browser_validation: off,
            label_case: "label-case",
            state => {
                widget: select {
                    choices: STATES,
                },
            },
            telephone_number => { widget: tel },
            email => { widget: email },
            gender => { widget: radio_group { choices: GENDERS } },
            zip => { pattern: r"\d{5}(-\d{4})?" },
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
