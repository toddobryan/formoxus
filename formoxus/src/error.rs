use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormError(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError(pub String);

pub fn try_from<T>(raw: &str) -> Result<T, FieldError>
where
    T: FromStr,
    T::Err: Display,
{
    raw.parse::<T>().map_err(|e| FieldError(e.to_string())) 
}
