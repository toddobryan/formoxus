//! Proves `IntoSlot`'s one job: a bare closure, dropped into struct-literal
//! position with no turbofish anywhere, resolves to whichever slot type the
//! field declares.
//!
//! This is the compile test step 3 of the button-layer plan
//! (`crates/formoxus/BUTTONS_PLAN.md`) needed before committing to the
//! one-macro slot-struct design in step 4: if target-type inference couldn't
//! do this, the fallback was a macro generated per form instead of one shared
//! macro. `ToySlots` below stands in for what the eventual `supply!` macro
//! will generate.
//!
//! It also pins the reason [`Provider`] is a newtype and not a type alias:
//! `UncheckedHandler` and a bare `Rc<dyn Fn() -> Pin<Box<dyn Future<Output =
//! C>>>>` are both zero-argument closures, so at `C = ()` a type-ALIAS
//! `Provider<C>` would be the exact same type as `UncheckedHandler` — making
//! `IntoSlot<UncheckedHandler> for F` and `IntoSlot<Provider<C>> for F` the
//! *same* impl at that substitution. That is a hard `E0119`
//! conflicting-implementation error, caught when the two `impl` blocks
//! themselves are compiled, not at any call site — so unlike most bugs here,
//! there is no failing-test version of it: get it wrong and this whole crate
//! stops compiling. (Verified by hand against the type-alias version before
//! writing the newtype.)

use formoxus::*;
use googletest::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// One field per slot kind. Everything is built through `IntoSlot::into_slot`
/// below with no annotation telling it which impl to use — the field's own
/// declared type is the only thing disambiguating `validated` (takes the
/// model), `unchecked` (takes nothing, returns `()`), and `source` (takes
/// nothing, returns a `Vec<String>`) from each other.
struct ToySlots {
    validated: Handler<String>,
    unchecked: UncheckedHandler,
    source: Provider<Vec<String>>,
}

#[gtest]
fn into_slot_resolves_by_field_type_with_no_turbofish() {
    // Records which closure actually ran. The side effect lives OUTSIDE each
    // `async move` block deliberately: calling a `Handler`/`UncheckedHandler`/
    // `Provider` only produces a future, it doesn't poll one, so a side
    // effect placed *inside* the async body would never observably run
    // without a full executor. Put it before the `async move` instead, and
    // "the future got constructed" is proof enough that dispatch reached the
    // right closure.
    let log: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));

    let slots = ToySlots {
        validated: {
            let log = log.clone();
            IntoSlot::into_slot(move |_m: String| {
                log.borrow_mut().push("validated");
                async move {}
            })
        },
        unchecked: {
            let log = log.clone();
            IntoSlot::into_slot(move || {
                log.borrow_mut().push("unchecked");
                async move {}
            })
        },
        source: {
            let log = log.clone();
            IntoSlot::into_slot(move || {
                log.borrow_mut().push("source");
                async move { vec!["a".to_string()] }
            })
        },
    };

    // Each slot is genuinely callable as its own type, and dispatches to the
    // closure that was actually assigned to it rather than to one of the
    // other two impls.
    //
    // The futures are dropped unawaited on purpose, which is what the `allow` is
    // for: each closure pushes to `log` in its BODY and only then returns an
    // async block, so calling is enough to prove dispatch. Awaiting would need a
    // runtime and would test nothing further.
    #[allow(clippy::let_underscore_future)]
    {
        let _ = (slots.validated)("model".to_string());
        let _ = (slots.unchecked)();
        let _ = slots.source.call();
    }

    expect_that!(
        *log.borrow(),
        elements_are![eq(&"validated"), eq(&"unchecked"), eq(&"source")]
    );
}
