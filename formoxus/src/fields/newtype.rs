use std::fmt::Debug;
use std::marker::PhantomData;

use facet::{Facet, HeapValue, Partial, Peek, ReflectError, Shape, Type, UserType};
use formoxus_spec::FieldType;

use crate::error::ValidationMessage;
use crate::field::Field;
use crate::field_attrs::FieldAttrs;
use crate::widgets::{AnyWidget, Choice};

/// The [`Field`] for a newtype: `T`, a struct with one field, read and
/// written through `F`, the field for the type it wraps.
///
/// A `struct Markdown(String)` should read and write as its `String`, but
/// formoxus never assumes so. A model derives only `Facet`, and no newtype
/// is special, `#[facet(transparent)]` included, so the app says which of
/// its types are newtypes, either in one line or with an inner field of its
/// choosing:
///
/// ```ignore
/// TypeRegistry::new()
///     .newtype::<Markdown, String>()
///     .field_for(Newtype::<Motto, _>::new(StringField {
///         max_length: Some(40),
///         ..StringField::default()
///     }))
///     .install();
/// ```
///
/// Everything goes to `F`. `parse` parses with it and wraps the result;
/// `format`, `check` and `check_on_server` unwrap and pass it the inside.
/// `KIND`, `Context`, the widget, its attributes, the choices and whether to
/// trim are `F`'s, so a newtype over a `String` renders, constrains and
/// trims like one, and a widget registered for it must accept `F`'s `KIND`.
///
/// It wraps and unwraps through facet's reflection rather than `From`, so
/// the model needs no impls of its own. `T` may be a tuple struct or a
/// struct with one named field.
///
/// ```
/// use facet::Facet;
/// use formoxus::field::Field;
/// use formoxus::fields::{Newtype, StringField};
///
/// #[derive(Clone, Debug, PartialEq, Facet)]
/// struct Motto(String);
///
/// let field = Newtype::<Motto, _>::new(StringField {
///     max_length: Some(5),
///     ..StringField::default()
/// });
/// assert_eq!(field.parse("Carpe").unwrap(), Motto("Carpe".into()));
/// assert_eq!(field.format(&Motto("Carpe".into())), "Carpe");
/// assert!(!field.check(&Motto("Carpe diem".into())).is_empty());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Newtype<T, F> {
    field: F,
    _data: PhantomData<fn() -> T>,
}

impl<T, F: Field + Default> Default for Newtype<T, F>
where
    T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    /// `F`'s default, through [`new`](Self::new), so it is checked the same
    /// way and panics the same way.
    fn default() -> Self {
        Self::new(F::default())
    }
}

const fn one_field(shape: &Shape) -> bool {
    matches!(shape.ty, Type::User(UserType::Struct(s)) if s.fields.len() == 1)
}

impl<T, F: Field> Newtype<T, F>
where
    T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    /// `T`, read and written through `field`.
    ///
    /// # Panics
    ///
    /// If `T`'s one field is not `F::Value`, as in a
    /// `Newtype::<Note, NumberField<u32>>` for a `struct Note(String)`. The
    /// message names both types.
    ///
    /// A `T` that is not a struct with exactly one field does not get this
    /// far: it fails to compile. That check runs when the app is built, not
    /// under `cargo check`, so rust-analyzer will not show it.
    pub fn new(field: F) -> Self {
        const {
            assert!(
                one_field(T::SHAPE),
                "`Newtypes` T must be a struct with exactly one field"
            );
        };
        let Type::User(UserType::Struct(s)) = T::SHAPE.ty else {
            unreachable!()
        }; // the const assert guarantees this
        let inner = s.fields[0].shape();
        assert!(
            inner == F::Value::SHAPE,
            "{} wraps {inner}, not {}",
            T::SHAPE,
            F::Value::SHAPE
        );
        Self {
            field,
            _data: PhantomData,
        }
    }
}

/// Builds a `T` around `inner`, through facet rather than a `From` impl, so
/// the model needs none.
fn wrap<T: for<'f> Facet<'f>, V: for<'f> Facet<'f>>(inner: V) -> T {
    build::<T, V>(inner)
        .expect("Newtype::new checked that T holds one V")
        .materialize::<T>()
        .expect("Newtype::new checked that T holds one V")
}

/// Every step returns `ReflectError`, so `?` chains them; `materialize`
/// returns a different error type, so [`wrap`] calls it outside.
fn build<T: for<'f> Facet<'f>, V: for<'f> Facet<'f>>(
    inner: V,
) -> Result<HeapValue<'static>, ReflectError> {
    Partial::alloc::<T>()?
        .begin_nth_field(0)?
        .set(inner)?
        .end()?
        .build()
}

/// Borrows the `V` inside `outer`, for the methods that read a value rather
/// than make one.
fn unwrap<T: for<'f> Facet<'f>, V: for<'f> Facet<'f>>(outer: &T) -> &V {
    Peek::new(outer)
        .into_struct()
        .ok()
        .and_then(|s| s.field(0).ok())
        .and_then(|f| f.get::<V>().ok())
        .expect("Newtype::new checked that T holds one V")
}

impl<T, F: Field> Field for Newtype<T, F>
where
    T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    type Value = T;

    type Context = F::Context;

    const KIND: FieldType = F::KIND;

    fn parse(&self, raw: &str) -> Result<T, ValidationMessage> {
        let inner = self.field.parse(raw)?;
        Ok(wrap::<T, F::Value>(inner))
    }

    fn format(&self, value: &Self::Value) -> String {
        self.field.format(unwrap(value))
    }

    fn check(&self, value: &Self::Value) -> Vec<ValidationMessage> {
        self.field.check(unwrap(value))
    }

    fn check_on_server(
        &self,
        value: &Self::Value,
        ctx: &Self::Context,
    ) -> impl Future<Output = Vec<ValidationMessage>> + Send {
        self.field.check_on_server(unwrap(value), ctx)
    }

    // Every `Field` method with a default body has to be listed here, or it
    // silently uses the default instead of the inner field's: leave out
    // `trims` and a newtype over `String` stops trimming. A method added to
    // `Field` later belongs here too.

    fn widget_attrs(&self) -> FieldAttrs {
        self.field.widget_attrs()
    }

    fn default_widget(&self, optional: bool) -> AnyWidget {
        self.field.default_widget(optional)
    }

    fn choices(&self) -> Option<Vec<Choice>> {
        self.field.choices()
    }

    fn trims(&self, password: bool) -> bool {
        self.field.trims(password)
    }
}
