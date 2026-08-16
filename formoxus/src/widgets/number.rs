use dioxus::prelude::*;

use crate::{fields::FormField, widgets::{DefaultWidget, FieldProps, FieldWidget, InputWidget}};

macro_rules! int_input {
    ($($int_type:ident),* $(,)?) => {
        paste::paste! {
            $(
                pub struct [<$int_type:camel Input>];

                impl FieldWidget<$int_type> for [<$int_type:camel Input>] {
                    fn render(field: Store<FormField<$int_type>>, props: FieldProps) -> Element {
                        rsx! { 
                            InputWidget::<$int_type> { input_type: "text", inputmode: "numeric", field, props } 
                        }
                    }
                }

                impl DefaultWidget for $int_type {
                    type Widget = [<$int_type:camel Input>];
                }
            )*
        }
    };
}

int_input!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, usize, isize);

macro_rules! float_input {
    ($($float_type:ident),* $(,)?) => {
        paste::paste! {
            $(
                pub struct [<$float_type:camel Input>];

                impl FieldWidget<$float_type> for [<$float_type:camel Input>] {
                    fn render(field: Store<FormField<$float_type>>, props: FieldProps) -> Element {
                        rsx! { 
                            InputWidget::<$float_type> { input_type: "text", inputmode: "decimal", field, props } 
                        }
                    }
                }

                impl DefaultWidget for $float_type {
                    type Widget = [<$float_type:camel Input>];
                }
            )*
        }
    };
}

float_input!(f32, f64);
