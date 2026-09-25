//! A picked radio can't be un-picked, so an optional field would need a
//! synthetic "none" radio — and `--none--` is a poor stand-in for the explicit
//! "None of the above" that a real unanswerable question wants. If absent is a
//! legitimate answer, it is either a real choice value or a `select`.
//!
//! This is a SECOND assert, separate from the chooser rule: the list is
//! supplied here, so `radio_group` passes the check it shares with `select`
//! and fails only on the `Option`.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    state: Option<String>,
}

const STATES: &[(&str, &str)] = &[("AL", "Alabama")];

fn main() {
    let _ = empty_form::<Address>(form! {
        Address {
            state => { widget: radio_group { choices: STATES } },
        }
    });
}
