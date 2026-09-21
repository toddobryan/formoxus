# The same form, both ways

A side-by-side of the deleted `#[derive(Form)]` path and the current `form!`
reflection path, end to end: declare, populate, render, cross the wire, come
back.

**Provenance.** The derive-path code is recovered from `3e9b8c0^` — the commit
before "Delete the derive path" — chiefly `formoxus/examples/form_example.rs`
and `formoxus/tests/derive.rs`. It is not reconstructed from memory. The
reflection-path code is what compiles today, **except** for the pieces marked
PROPOSED (`WireForm`, `absorb`, `path!`), which are designs, not code.

---

## 0. The shared premise: what your model has to derive

| | derive path | reflection path |
|---|---|---|
| model derives | `Form` | `Facet` |
| model crate depends on | `formoxus` → and transitively `dioxus` | `facet` only |
| does the model crate know formoxus exists? | **yes** | **no** |
| model you don't own | write a parallel struct + two conversions | write a parallel struct + two conversions |

Both paths lose to a foreign model the same way — you write a stand-in and
convert. The difference is what the stand-in costs. On the derive path it drags
`formoxus` and `dioxus` into a crate that may have no business knowing about
either; a server-only `api` crate ends up depending on a UI framework. On the
reflection path the stand-in derives `Facet` and nothing else.

One asymmetry worth naming up front, because it isn't about dependencies:

**The derive path made the form and the struct the same thing.** Two forms over
one model meant two structs. A create form and an admin-edit form over the same
`User` were two declarations that had to be kept in sync by hand. On the
reflection path a spec is a value, so `fn create_spec()` and `fn admin_spec()`
are two functions over one model.

---

## 1. Declare

### Derive

```rust
#[derive(Form, Debug, Clone, Serialize, Deserialize)]
#[form(title = "Sign up", button(type = "submit", name = create, text = "Create account"))]
pub struct Signup {
    pub email: String,
    #[form(component = PasswordInput)]
    pub password: String,
    pub bio: String,
    pub agreed: bool,
}
```

The derive was `model-is-Self`: `Signup` is both the declaration and the
validated output. From it the macro generated:

```rust
// generated
#[derive(Clone, Debug, Default, Serialize, Deserialize, Store)]
pub struct SignupState {
    pub email: FormField<String>,
    pub password: FormField<String>,
    pub bio: FormField<String>,
    pub agreed: FormField<bool>,
    pub errors: Vec<FormError>,
}

impl FormState for SignupState {
    type Model = Signup;
    type Handlers = SignupHandlers;
    type Providers = ();
    fn validate(&mut self) -> Option<Signup> { /* … */ }
    fn render(data: Store<Self>, handlers: Self::Handlers, providers: Self::Providers) -> Element { /* … */ }
    fn has_errors(&self) -> bool { /* … */ }
}

impl FromModel<Signup> for SignupState { /* … */ }
```

You hand-wrote only the cross-field validator, which the macro couldn't infer:

```rust
impl ValidateForm<Signup> for SignupState {
    fn validate_form(&self, model: &Signup) -> Vec<FormError> { /* … */ }
}
```

Note `#[form(component = PasswordInput)]`. The widget is named **inside the
model's crate**, so that crate has to see the widget's crate. This is the orphan
problem that eventually killed the approach: a clean `models` crate cannot name
`ui::MarkdownWidget` without depending on `ui`.

### Reflection

```rust
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
            email    => { widget: email },
            password => { widget: password },
            bio      => { label: "About you", widget: textarea },
            agreed   => { label: "I agree to the terms" },
            buttons: {
                create: { type: submit, text: "Create account" },
            }
        }
    }
}
```

Nothing is generated as a type. `form!` expands to a `FormSpec<Signup>` builder
chain plus a never-called witness function that makes rustc verify every path.

The declaration lives wherever you want, which is what dissolves the orphan
problem: `form!` expands at the **call site**, so a spec in crate `web` can name
a model from `models` and a widget from `ui` without either of them knowing
about the other.

---

## 2. Populate

### Derive

```rust
let state = SignupState::default();               // create mode
let state = SignupState::from_model(&existing);   // edit mode — initial == value
```

### Reflection

```rust
let form = use_form(|| empty_form(spec()));            // create mode
let form = use_form(|| form_for(&existing, spec()));   // edit mode
```

Even trade. The reflection version threads the spec through, which is one extra
argument and the reason a spec can be reused.

---

## 3. Render

### Derive

`render` was an associated function on the generated impl, and the generated
body named every field explicitly:

```rust
#[component]
fn SignupForm() -> Element {
    let data = use_store(SignupState::default);
    SignupState::render(
        data,
        SignupHandlers { create: handler(create_account) },
        (),
    )
}
```

Handlers arrived as a **generated struct** — one field per declared button, so a
missing or misnamed handler was a compile error.

### Reflection

```rust
#[component]
fn SignupForm() -> Element {
    let form = use_form(|| empty_form(spec()));

    rsx! {
        {form.render(using_fns! {
            create: move |model: Signup| async move { create_account(model).await },
        })}
    }
}
```

Handlers arrive as a **runtime-reconciled map**, because there is no per-form
type to hang a struct literal on. Names are checked when `render` reconciles
them against the spec's buttons, not at compile time.

**This is a real regression and it is structural.** The generated-struct
approach was tried on the reflection path and proved impossible for exactly this
reason. Arity still carries meaning — `|model|` runs only after validation,
`||` runs regardless — but a typo in a button name surfaces at render.

---

## 4. Cross the wire

This is where the two paths genuinely diverge, and the reason is one line of
type theory.

### Derive — the form *was* the wire type

`SignupState` is a concrete generated struct of `FormField<T>`, so
`#[derive(Serialize, Deserialize)]` just worked. The comment in the original
example says it outright: *"the `Store` content **and** the serializable wire
type (it round-trips whole)"*.

```rust
#[post]
async fn create_account(mut state: SignupState) -> Result<SignupState, ServerFnError> {
    // No reconstruction. The form arrived intact.
    let Some(model) = state.validate() else {
        return Ok(state);              // field errors already attached in place
    };

    if email_taken(&model.email).await? {
        state.email.errors.push(FieldError("already registered".into()));
        return Ok(state);
    }

    insert(model).await?;
    Ok(SignupState::default())
}
```

Client side, the return trip was an assignment:

```rust
let returned = create_account(data.peek().clone()).await?;
*data.write() = returned;
```

And the server had `state.email.initial`, `.raw`, and `.value` — typed, per
field, no paths involved.

### Reflection — the form tree cannot cross

`FormState<T>` holds `Vec<Box<dyn FormMember>>`. Trait objects do not
deserialize. `FormSpec<T>` holds `fn` pointers for the validator and every
custom widget. Neither type can cross a server fn, and no amount of API design
changes that.

So the spec crosses as **code** and the values cross as **data**:

```rust
// `spec()` lives in a crate both sides see.
#[post]
async fn create_account(values: HashMap<String, String>) -> Result<(), FormErrors> {
    let sub = Submission::accept(spec(), &values)?;

    if email_taken(&sub.model().email).await? {
        return Err(sub.reject_field("email", "already registered"));
    }

    insert(sub.into_model()).await?;
    Ok(())
}
```

**This is not pure loss.** A spec that crossed the wire would be a spec the
client controls. The server naming `spec()` is what makes `Submission::accept` a
trustworthy check rather than a re-run of whatever rules the client sent. On the
derive path the same property held, but by accident of `fn` pointers rather than
by design.

---

## 5. Come back

### Derive

One assignment, shown above. Every field's errors, values and initial values
came home together.

### Reflection, today

This is the part that stings, and it is the honest low point of the current API:

```rust
match create_account(form.values().peek().clone()).await {
    Ok(()) => { /* navigate away */ }
    Err(errs) => {
        for (path, msgs) in errs.fields {
            for m in msgs {
                let _ = form.push_field_error(&path, &m);
            }
        }
        // form-level errors in `errs.form` have nowhere to go yet
    }
}
```

`FormState::collect_errors()` produces a `FormErrors`; **nothing in the crate
consumes one.** The return trip is a hand-written loop with a stringly-typed
path, at every call site.

### Reflection, PROPOSED

```rust
#[post]
async fn create_account(wire: WireForm<Signup>) -> Result<WireForm<Signup>, ServerFnError> {
    let sub = Submission::accept(spec(), &wire)?;

    if email_taken(&sub.model().email).await? {
        return Ok(sub.reject_field(path!(Signup.email), "already registered").into_wire());
    }

    insert(sub.into_model()).await?;
    Ok(WireForm::clean())
}
```

```rust
form.absorb(create_account(form.to_wire()).await?);
```

`WireForm<T>` is `{ values: HashMap<String, String>, errors: FormErrors }` —
plain data, serializable. Every primitive it needs already exists:

| direction | existing primitive |
|---|---|
| values out | `FormState::leaves()` / `as_hash_map()` |
| values in | `Form::reset()`'s per-path `write_value` loop |
| errors out | `FormState::collect_errors()` |
| errors in | `Form::push_field_error(path, msg)` |

`reset()` is already `absorb` with local data — it replaces the whole
`FormState` in the signal, then writes each leaf individually so every input's
own subscription fires. `absorb` is that function with a different source.

---

## 6. What the server can do *to* the form

Validating and rejecting is not all a server wants. Three more things, and they
land differently.

### Normalizing values and sending them back

`FormState` already exposes three public mutators, and `edit` is the same
channel the client's widgets use:

```rust
pub fn apply(&mut self, values: &HashMap<String, String>)
pub fn apply_form_values(&mut self, values: &[(String, String)])
pub fn edit(&mut self, edit: &Edit) -> Result<(), FormAccessError>
```

So the server can already modify the form. What it cannot do is **return** the
result — which is the `WireForm` gap again, not a separate one. Once values
travel in both directions:

```rust
let sub = Submission::accept(spec(), &wire)?;
let mut model = sub.into_model();
model.email = model.email.trim().to_lowercase();

let state = form_for(&model, spec());      // regenerate every leaf from the model
Ok(WireForm { values: state.as_hash_map(), errors: Default::default() })
```

This is **better than the derive path**, not worse. You normalize the typed
model and the leaves are derived from it, so values cannot drift out of sync.
The derive path normalized by poking string fields, with nothing keeping
`state.email.raw` and `state.email.value` consistent.

### Typed values per field

`sub.model()` is `&T`, fully parsed, today — the derive path's
`FieldValue::Valid(T)` was also a parsed value, so neither approach avoided
parsing.

The real gap is narrower: **per-field typed values on a form that failed
validation.** The derive path let you read `state.count.value` as `Valid(3)`
while `state.name` carried an error. `Submission::accept` returns
`Err(FormErrors)` and drops the `FormState`, so a partially-valid form yields
nothing typed.

Expensive to recover, unlike the others. `FormState` holds
`Vec<Box<dyn FormMember>>`, so the per-field `T` is erased — you would need an
`Any` downcast plus a path-typed getter, `state.get::<i32>(path!(Signup.count))`.
New machinery, not plumbing.

### Initial values, and comparing them

Genuinely absent server-side: `Form.initial` is client-only and `accept` sees
submitted values alone.

But look at what the derive path actually provided. `SignupState` crossed whole
*including its `initial`*, supplied by the client. So "has this changed?" was
answered from a number the caller chose. For "skip the write if nothing changed"
the worst case is a silent no-op — a correctness bug. The neighbouring pattern
is the dangerous one: *"only permission-check the fields that changed"* lets a
lying client skip the check entirely.

So: carrying `initial` in `WireForm` is cheap and fine for dirty-tracking and
UI. **Never gate a trust decision on it.** For the write-skipping case, compare
against the row the server loads anyway — trustworthy, and free, since an update
needs that row regardless.

---

## Scorecard

| | derive | reflection (today) | reflection (+ proposed) |
|---|---|---|---|
| model crate knows formoxus | yes | **no** | no |
| model crate pulls in dioxus | yes | **no** | no |
| foreign model | stand-in + conversions | stand-in + conversions | same |
| two forms over one model | two structs | **a function each** | same |
| custom widget from another crate | orphan problem | **works** | same |
| typed field access (`state.email.errors`) | **yes** | no — paths | no — but paths are checked |
| path safety | n/a (fields) | strings at runtime | **`path!`, compile-checked** |
| button-name safety | **compile error** | render-time reconcile | unchanged |
| form crosses the wire whole | **yes** | no — values + shared spec | no, by design |
| return trip | **one assignment** | hand-written loop | **one `absorb`** |
| server sees raw + parsed model | yes, typed | **yes** — `sub.model()` | same |
| per-field typed value on a FAILED form | **yes** | no — state is dropped | no (needs `Any` downcast) |
| server modifies values | yes | **yes** — `apply`/`edit` | same |
| …and returns them to the client | **yes** | no | **yes** — `WireForm` |
| server sees initial values | yes, but client-supplied | no | optional, still untrusted |
| cross-field validator | hand-written impl | `validator:` in `form!` | same |

---

## The one-sentence version

The derive path generated a concrete struct, so serialization, typed field
access and compile-checked handler names all came free — and it paid for them by
putting formoxus, dioxus, and every widget's crate into the model's dependency
graph, and by making one struct mean exactly one form.

The reflection path pays in stringly-typed paths and a manual return trip, and
buys a model crate that only knows `facet`.

Of the two costs, **the return trip is a missing function, not a missing
capability** — every primitive exists. And **path safety is recoverable in
full**, because the witness trick that checks `venue.city` inside `form!`
generalizes to `path!(Signup.venue.city)` anywhere, for any field, whether or
not the form mentions it.

What does not come back is typed field access — `state.email.errors` needs a
generated struct, and a generated struct needs the field list at macro time,
which only a derive on the model has. That is the trade, and it is the whole
trade.
