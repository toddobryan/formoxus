use dioxus::prelude::*;
use facet::Facet;
use formoxus::{form2, prelude::*, reflect::FormSpec};
use googletest::prelude::*;

#[derive(Clone, Debug, Facet)]
pub struct FakeFormWithButtons {
    pub some_data: String,
}

fn fake_form() -> FormSpec<FakeFormWithButtons> {
    form2!(
        FakeFormWithButtons {
            some_data => {
                control: textarea,
            },
            buttons: {
                delete: { type: destructive, text: "Drop" },
                reload: { type: reset },
                update: { type: submit, text: "Save to Db"},
            }
        }
    )
}

#[component]
pub fn FakeForm() -> Element {
    let form = use_form(|| empty_form(fake_form()));
    form.render(        
        using_fns! {
            delete: todo!(),
            reload: todo!(),
            update: todo!(),
        }
    )
}