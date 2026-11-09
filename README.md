# formoxus

Reflection-based forms for [Dioxus](https://dioxuslabs.com), built at runtime
from a model's [`facet`](https://facet.rs) shape instead of from a hand-written
form struct.

Your model derives `Facet` and nothing else. `form!` declares the form over its
shape — labels, widgets, validators, buttons — and `use_form` makes it live.
There is no parallel "form" type to keep in sync with the model, and no
`FromStr`/`Display` bounds to satisfy: values convert through facet's own
vtables.

> **Status: pre-release.** Not yet published to crates.io. The API is still
> moving, and some declarable widgets are not implemented — see
> [What isn't done](#what-isnt-done) before adopting it.

## Example

```rust
use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
    password: String,
}

fn signup_spec() -> FormSpec<Signup> {
    form! {
        Signup {
            title: "Create an account",
            email => { widget: email },
            password => { widget: password },
            buttons: {
                create: { type: submit, text: "Sign up" },
            }
        }
    }
}

#[component]
fn SignupForm() -> Element {
    let form = use_form(|| empty_form(signup_spec()));

    form.render(using_fns! {
        // Arity picks the contract: `|model|` runs only once validation passes.
        create: |model| async move {
            let _ = model;
        },
    })
}
```

Every field path named in `form!` is checked against the model's real shape at
compile time, so `emial` is a compile error rather than a runtime
`no_such_path`.

## Checked as you go

A field is checked as the user goes, by the same rules a submit uses:

- not while they first type into it, but when they leave it;
- then on every keystroke while it shows an error, so the error goes the
  moment it is fixed;
- at once when a select, checkbox or radio changes.

Tabbing through a form without typing says nothing, and a server's verdict
("that name is taken") stays until the user changes the field. A field checked
and found clean renders `aria-invalid="false"`; one never checked renders no
`aria-invalid` at all.

To check only on submit, write `live_validation: off` in one form's `form!`,
or turn it off for the whole app with
`provide_defaults(Formoxus::new().with_live_validation(false))`.

## Crossing the wire

This is the part that's usually painful, so it's worth showing directly.

A form does **not** cross a server-function boundary — `FormState` holds
`Box<dyn Node>` and isn't serializable. What crosses is a `WireForm`: the
form's values and its errors as plain data. The server rebuilds the form from
the *same* `FormSpec` and validates it there, so cross-field rules live in one
place instead of being duplicated on both sides.

Client side, send the form and take the reply back into it:

```rust,ignore
let reply = change_password(form.to_wire()).await?; // `?`: a transport failure
let ok = reply.is_clean();
// The server's errors back onto their fields, plus any value it normalized.
// `Err` names a path this form doesn't have: the two sides' specs disagree.
form.absorb(reply)?;
if ok { /* success */ }
```

Server side, `Submission` packages the rebuild-and-validate boilerplate:

```rust,ignore
#[server]
async fn change_password(
    form: WireForm<ChangePassword>,
) -> ServerFnResult<WireForm<ChangePassword>> {
    // Runs the spec's own validators, including cross-field ones.
    let submission = match Submission::accept(change_password_spec(), form.values()) {
        Ok(s) => s,
        Err(errors) => return Ok(errors.into()),
    };

    if !current_password_matches(submission.model()).await {
        // `reject_field` CONSUMES the submission, so "don't validate again
        // after pushing a server error" is a compile error, not a comment.
        return Ok(submission.reject_field(
            &path!(ChangePassword.current),
            "That isn't your password",
        ));
    }

    apply(submission.into_model()).await;
    Ok(WireForm::empty())
}
```

`is_clean()` rather than `errors().fields.is_empty()`, because a cross-field
failure has no path: it lands in `errors().form`, and would otherwise read as
success.

## What isn't done

Being explicit, since this is pre-release:

- **Not every `(value kind, widget)` pair renders**, and `form!` refuses
  the ones that don't at compile time, pointing at the widget's name:
  `checkbox` goes only on a `bool`, `textarea` only on a string, no
  `<input type=…>` on a `bool`, `select` and `radio_group` on anything but a
  `bool` only with `choices:`, and neither `checkbox` nor `radio_group` on an
  `Option`. `select_multiple`, `checkbox_multiple` and `file` render nothing
  yet and are refused by name. Run
  `cargo run -p formoxus-examples --bin widget_matrix` for the current table.
- **No file upload**, which is the `file` widget above.
- **An error that appears as a field loses focus moves what is below it.**
  Clicking a button just below a field with a bad value blurs the field on
  the press, and its error renders before the release, so the button can move
  out from under the click, which then has to be repeated
  ([#24](https://github.com/toddobryan/formoxus/issues/24)).
- Error rendering is not yet configurable; `FieldErrors` is the component to
  swap when it becomes so.

A widget of your own is the escape hatch in the meantime: any type
implementing `formoxus::widgets::Widget`, named by its path after `widget:`.
Props go in its braces and the rest come from `Default`, so
`widget: ui::Markdown { rows: 8 }` sets one field. The type and the impl are
both in your crate, so there is no orphan-rule problem, and `form!` checks
each attribute on the field against the widget's `EXTENDS`.

## Minimum supported Rust version

**1.90**, inherited from `facet`. This is derived from the dependency rather
than verified by building on that exact toolchain, and it is not currently
treated as a semver-stable guarantee.

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
