# formoxus

Reflection-based forms for [Dioxus](https://dioxuslabs.com), built at runtime
from a model's [`facet`](https://facet.rs) shape instead of from a hand-written
form struct.

Your model derives `Facet` and nothing else. `form!` declares the form over its
shape — labels, controls, validators, buttons — and `use_form` makes it live.
There is no parallel "form" type to keep in sync with the model, and no
`FromStr`/`Display` bounds to satisfy: values convert through facet's own
vtables.

> **Status: pre-release.** Not yet published to crates.io. The API is still
> moving, and some declarable controls are not implemented — see
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
            email => { control: email },
            password => { control: password },
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

## Crossing the wire

This is the part that's usually painful, so it's worth showing directly.

A form does **not** cross a server-function boundary — `FormState` holds
`Box<dyn FormMember>` and isn't serializable. What crosses is the leaves: plain
`(path, value)` string pairs. The server rebuilds the form from the *same*
`FormSpec` and validates it there, so cross-field rules live in one place
instead of being duplicated on both sides.

Client side, send the values and route any errors back onto the fields:

```rust,ignore
let values = form.values().read().clone(); // HashMap<String, String>

match change_password(values).await {
    Ok(Ok(())) => { /* success */ }
    Ok(Err(errors)) => {
        for (path, field_errors) in errors.fields {
            for e in field_errors {
                // An unplaceable path degrades to a form-level error rather
                // than being dropped — the user is still owed the message.
                if form.push_field_error(&path, &e.0).is_err() {
                    form.push_error(e);
                }
            }
        }
        for e in errors.form {
            form.push_error(e);
        }
    }
    Err(e) => { /* transport failure */ }
}
```

Server side, `Submission` packages the rebuild-and-validate boilerplate:

```rust,ignore
#[server]
async fn change_password(
    values: HashMap<String, String>,
) -> ServerFnResult<Result<(), FormErrors>> {
    // Runs the spec's own validators, including cross-field ones.
    let submission = match Submission::accept(change_password_spec(), &values) {
        Ok(s) => s,
        Err(errors) => return Ok(Err(errors)),
    };

    if !current_password_matches(submission.model()).await {
        // `reject_field` CONSUMES the submission, so "don't validate again
        // after pushing a server error" is a compile error, not a comment.
        return Ok(Err(submission.reject_field("current", "That isn't your password")));
    }

    apply(submission.into_model()).await;
    Ok(Ok(()))
}
```

`Result<(), FormErrors>` rather than "an empty `FormErrors` means success",
because a cross-field failure has an empty `fields` list and would otherwise
read as success.

## What isn't done

Being explicit, since this is pre-release:

- **Four declarable controls panic at render**: `select_multiple`,
  `checkbox_multiple`, `radio_group`, and `file`. `select` currently works only
  on `bool`. `form!` accepts all of these names today; only the `<input type=…>`
  family, `textarea`, `checkbox`, and `custom(...)` are fully wired.
- **No file upload**, which is the `file` control above.
- **No live/as-you-type validation** — validation runs on submit.
- **`Vec` rows are named by index**, so reordering rows renames their paths.
- Error rendering is not yet configurable; `FieldErrors` is the component to
  swap when it becomes so.

`custom(MyWidget)` is the escape hatch in the meantime: a custom control is an
ordinary Dioxus component, with no trait to implement and so no orphan-rule
problem.

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
