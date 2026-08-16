//! Value-level parsing coverage for the numeric widgets (`crates/formoxus/src/widgets/number.rs`).
//!
//! `InputWidget`'s `oninput` handler runs a two-step pipeline: trim the raw
//! string, then `try_from::<T>` it. That pipeline is plain data — no Dioxus
//! runtime needed — so it's tested directly here, the same way `derive.rs` /
//! `signup.rs` test form validation without a browser.

use std::fmt::Display;
use std::str::FromStr;

use formoxus::error::{FieldError, try_from};
use googletest::prelude::*;

/// Mirrors exactly what `InputWidget`'s `oninput` does before calling `try_from`
/// (see `crates/formoxus/src/widgets/input.rs`).
fn parse<T>(raw: &str) -> Result<T, FieldError>
where
    T: FromStr,
    T::Err: Display,
{
    try_from(raw.trim())
}

#[gtest]
fn surrounding_whitespace_is_trimmed_for_every_int_type() {
    expect_that!(parse::<u8>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<u16>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<u32>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<u64>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<usize>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<i8>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<i16>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<i32>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<i64>(" 83  "), ok(eq(&83)));
    expect_that!(parse::<isize>(" 83  "), ok(eq(&83)));
}

#[gtest]
fn surrounding_whitespace_is_trimmed_for_float_types() {
    expect_that!(parse::<f32>(" 83  "), ok(eq(&83.0)));
    expect_that!(parse::<f64>(" 83  "), ok(eq(&83.0)));
}

#[gtest]
fn float_parses_integer_looking_strings_with_no_decimal_point() {
    // The concrete case from the widget review: "83" (not "83.0") must parse.
    expect_that!(parse::<f32>("83"), ok(eq(&83.0)));
    expect_that!(parse::<f64>("83"), ok(eq(&83.0)));
}

#[gtest]
fn float_accepts_leading_dot_and_scientific_notation() {
    expect_that!(parse::<f64>(".5"), ok(eq(&0.5)));
    expect_that!(parse::<f64>("1e3"), ok(eq(&1000.0)));
}

#[gtest]
fn negative_numbers_parse_for_signed_int_types() {
    expect_that!(parse::<i8>("-5"), ok(eq(&-5)));
    expect_that!(parse::<i16>("-5"), ok(eq(&-5)));
    expect_that!(parse::<i32>("-5"), ok(eq(&-5)));
    expect_that!(parse::<i64>("-5"), ok(eq(&-5)));
    expect_that!(parse::<isize>("-5"), ok(eq(&-5)));
}

#[gtest]
fn negative_numbers_are_rejected_for_unsigned_int_types() {
    expect_that!(parse::<u8>("-5"), err(anything()));
    expect_that!(parse::<u16>("-5"), err(anything()));
    expect_that!(parse::<u32>("-5"), err(anything()));
    expect_that!(parse::<u64>("-5"), err(anything()));
    expect_that!(parse::<usize>("-5"), err(anything()));
}

#[gtest]
fn out_of_range_values_are_rejected_not_wrapped() {
    // u8::MAX == 255 — a naive wrapping read would silently give 44, not an error.
    expect_that!(parse::<u8>("300"), err(anything()));
    // i8::MAX == 127.
    expect_that!(parse::<i8>("300"), err(anything()));
}

#[gtest]
fn non_numeric_text_is_rejected_for_every_type() {
    expect_that!(parse::<u8>("abc"), err(anything()));
    expect_that!(parse::<i32>("abc"), err(anything()));
    expect_that!(parse::<usize>("abc"), err(anything()));
    expect_that!(parse::<f32>("abc"), err(anything()));
    expect_that!(parse::<f64>("abc"), err(anything()));
}

#[gtest]
fn u128_and_i128_round_trip_values_that_overflow_64_bit() {
    // One past u64::MAX (18446744073709551615) — only representable in a 128-bit type.
    expect_that!(parse::<u128>("18446744073709551616"), ok(eq(&18446744073709551616)));
    // Below i64::MIN (-9223372036854775808).
    expect_that!(parse::<i128>("-9223372036854775809"), ok(eq(&-9223372036854775809)));
}

#[gtest]
fn u128_and_i128_still_reject_out_of_range_and_non_numeric_input() {
    expect_that!(parse::<u128>("-5"), err(anything()));
    expect_that!(parse::<i128>("abc"), err(anything()));
}
