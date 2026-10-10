#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Bound {
    Int(i128),
    Float(f64),
}

macro_rules! bound_from_int {
    // Infallible and lossless — `i128::from` says so in the type system.
    (from: $($t:ty),* $(,)?) => { $(
        impl From<$t> for Bound {
            fn from(v: $t) -> Self { Bound::Int(i128::from(v)) }
        }
    )* };
    // `usize`/`isize` have no `From<_> for i128` — their width is
    // target-dependent — so these need the cast. Lossless on every target
    // Rust supports: `i128` is wider than any pointer.
    (cast: $($t:ty),* $(,)?) => { $(
        impl From<$t> for Bound {
            fn from(v: $t) -> Self { Bound::Int(v as i128) }
        }
    )* };
}
bound_from_int!(from: i8, i16, i32, i64, i128, u8, u16, u32, u64);
bound_from_int!(cast: isize, usize);

impl From<f32> for Bound {
    fn from(v: f32) -> Self {
        Bound::Float(f64::from(v))
    }
}
impl From<f64> for Bound {
    fn from(v: f64) -> Self {
        Bound::Float(v)
    }
}
