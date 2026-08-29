//! Stress test modeled on Django's `inlineformset_factory` pattern: a parent
//! form (`Order`) with a dynamically-sized list of child rows (`line_items`),
//! where each row has an FK-style `ModelChoiceField` (`product`, backed by a
//! `Provider`) alongside a plain field (`quantity`). Every other formoxus test
//! exercises `#[form(field_set)]` on `Vec<T>` (`field_set_list.rs`) or
//! `#[form(component = ..., provided)]` (`providers.rs`) in isolation; this is
//! the first to combine both — a repeating group whose *rows* each need
//! externally-supplied data — which is exactly the shape Django reaches for
//! `ModelChoiceField` inside a formset.

use std::str::FromStr;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use formoxus::prelude::*;
use googletest::prelude::*;

/// A minimal, dependency-free `block_on` for the one test below that needs to
/// actually drive a `Provider`'s future to completion. Every future in this
/// file resolves on its first poll (no real `.await` inside), so a no-op
/// waker is all that's needed — this isn't a real executor, just enough to
/// prove the `Provider` contract (an `Rc`-cloneable, repeatedly-callable
/// async fn) actually works end to end, not just that it type-checks.
fn block_on<F: std::future::Future>(mut f: F) -> F::Output {
    let mut f = unsafe { std::pin::Pin::new_unchecked(&mut f) };
    fn noop(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    fn noop_fn(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(noop, noop_fn, noop_fn, noop_fn);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(output) = f.as_mut().poll(&mut cx) {
            return output;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ProductId(String);

impl FromStr for ProductId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ProductId(s.to_string()))
    }
}

/// The `Choices` a real `ModelChoiceField` would evaluate its queryset into —
/// never actually read by this test (the widget is never rendered), just
/// present so `Provider<Vec<ProductChoice>>` has a concrete type.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductChoice {
    pub id: ProductId,
    pub name: String,
}

pub struct ProductPicker;

impl ProvidedWidget<ProductId> for ProductPicker {
    type Choices = Vec<ProductChoice>;

    fn render(
        _field: dioxus::stores::Store<FormField<ProductId>>,
        _props: FieldProps,
        _provide: Provider<Vec<ProductChoice>>,
    ) -> dioxus::core::Element {
        unimplemented!("never called — this test only proves the codegen and validate logic")
    }
}

#[derive(FieldSet, Debug, Clone, Default)]
#[allow(dead_code)]
pub struct LineItem {
    #[form(component = ProductPicker, provided)]
    product: ProductId,
    quantity: u32,
}

#[derive(Form, Debug)]
#[form(button(type = "submit", name = submit))]
#[allow(dead_code)]
pub struct OrderForm {
    customer_name: String,
    #[form(field_set)]
    line_items: Vec<LineItem>,
}

/// Django's `ModelChoiceField` re-evaluates its queryset once per formset
/// render, not once per row — every row's dropdown shares the same query.
/// `OrderFormProviders` shows formoxus lands in the same place structurally:
/// there's exactly one `Provider<Vec<ProductChoice>>` slot for the whole
/// repeating group (nested under `LineItemProviders`), not one per row, and
/// it can be invoked repeatedly (it's `Rc`-backed) — once per row, as each
/// row's widget would do on mount.
#[gtest]
fn one_provider_serves_every_row_in_the_group() {
    let products = vec![
        ProductChoice {
            id: ProductId("p1".to_string()),
            name: "Widget".to_string(),
        },
        ProductChoice {
            id: ProductId("p2".to_string()),
            name: "Gadget".to_string(),
        },
    ];

    let providers = OrderFormProviders {
        line_items: LineItemProviders {
            product: provider(move || {
                let products = products.clone();
                async move { products }
            }),
        },
    };

    // Simulate three rows each independently invoking the shared provider —
    // exactly what three mounted `ProductPicker`s would do.
    let fetch = providers.line_items.product.clone();
    for _ in 0..3 {
        let result = block_on(fetch());
        expect_that!(result, len(eq(2)));
    }
}

/// Every row's required fields validate independently — including the
/// `provided`-widget field. `provided` only changes how the field is
/// *rendered* (via `Providers`, not part of `validate` at all — confirmed by
/// reading the derive's `assign_to_vars` codegen); to `validate()` it's an
/// ordinary required field, same as `quantity`.
#[gtest]
fn each_rows_provided_field_validates_like_any_other_required_field() {
    let mut form = OrderFormState::default();
    form.customer_name = FormField::with_value("Ada".to_string());
    form.line_items.push(LineItemState::default()); // row 0: empty product AND quantity
    form.line_items.push({
        let mut row = LineItemState::default();
        row.product = FormField::with_value(ProductId("p1".to_string()));
        row.quantity = FormField::with_value(5);
        row
    });

    expect_that!(form.validate(), none());
    expect_that!(form.line_items[0].product.errors, len(eq(1)));
    expect_that!(form.line_items[0].quantity.errors, len(eq(1)));
    expect_that!(form.line_items[1].product.errors, is_empty());
    expect_that!(form.line_items[1].quantity.errors, is_empty());
}

/// A fully-valid order (parent field plus every row) round-trips through
/// `validate()` into the model, in row order — the same shape a submit
/// handler would receive.
#[gtest]
fn a_fully_valid_order_round_trips_through_validate() {
    let mut form = OrderFormState::default();
    form.customer_name = FormField::with_value("Ada".to_string());
    form.line_items.push({
        let mut row = LineItemState::default();
        row.product = FormField::with_value(ProductId("p1".to_string()));
        row.quantity = FormField::with_value(3);
        row
    });
    form.line_items.push({
        let mut row = LineItemState::default();
        row.product = FormField::with_value(ProductId("p2".to_string()));
        row.quantity = FormField::with_value(1);
        row
    });

    let model = form.validate();
    expect_that!(model, some(anything()));
    let model = model.expect("checked above");
    expect_that!(model.customer_name, eq(&"Ada".to_string()));
    expect_that!(
        model.line_items[0].product,
        eq(&ProductId("p1".to_string()))
    );
    expect_that!(model.line_items[0].quantity, eq(3));
    expect_that!(
        model.line_items[1].product,
        eq(&ProductId("p2".to_string()))
    );
    expect_that!(model.line_items[1].quantity, eq(1));
}

/// **Gap surfaced by this test, not a passing assertion of one:** unlike
/// Django's `ModelChoiceField.clean()`, which re-queries the DB to confirm
/// the submitted pk is actually in the queryset, formoxus's `validate()` for
/// a `provided` field never looks at `Providers` at all — it's wired only
/// into `render`. A `ProductId` that parses via `FromStr` (any string, here —
/// `ProductId`'s `FromStr` is infallible) but names a product that was never
/// in the picker's choices, or was deleted after the picker fetched them,
/// validates successfully with no error. Confirmed here: `"nonexistent"`
/// isn't in `products` above, yet this row validates clean.
#[gtest]
fn validate_does_not_cross_check_the_submitted_value_against_the_provider() {
    let mut form = OrderFormState::default();
    form.customer_name = FormField::with_value("Ada".to_string());
    form.line_items.push({
        let mut row = LineItemState::default();
        row.product = FormField::with_value(ProductId("nonexistent".to_string()));
        row.quantity = FormField::with_value(1);
        row
    });

    let model = form.validate();
    expect_that!(model, some(anything()));
}
