use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase,
    ToTrainCase, ToUpperCamelCase,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelCase {
    CamelLower,
    CamelCapitalized,
    KebabLower,
    KebabCapitalized,
    KebabAllCaps,
    SnakeLower,
    SnakeCapitalized,
    SnakeAllCaps,
    Title,
    Lower,
    AllCaps,
}

pub trait ToCase {
    fn to_case(self, case: LabelCase) -> String;
}

impl ToCase for &str {
    fn to_case(self, case: LabelCase) -> String {
        match case {
            LabelCase::CamelLower => self.to_lower_camel_case(),
            LabelCase::CamelCapitalized => self.to_upper_camel_case(),
            LabelCase::KebabLower => self.to_kebab_case(),
            LabelCase::KebabCapitalized => self.to_train_case(),
            LabelCase::KebabAllCaps => self.to_shouty_kebab_case(),
            LabelCase::SnakeLower => self.to_snake_case(),
            LabelCase::SnakeCapitalized => self.to_train_case().replace('-', "_"),
            LabelCase::SnakeAllCaps => self.to_shouty_snake_case(),
            LabelCase::Title => self.to_title_case(),
            LabelCase::Lower => self.to_title_case().to_lowercase(),
            LabelCase::AllCaps => self.to_title_case().to_uppercase(),
        }
    }
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;

    use super::{LabelCase, ToCase};

    #[gtest]
    fn camel_lower() {
        expect_that!("opt_flag".to_case(LabelCase::CamelLower), eq("optFlag"));
    }

    #[gtest]
    fn camel_upper() {
        expect_that!(
            "opt_flag".to_case(LabelCase::CamelCapitalized),
            eq("OptFlag")
        );
    }

    #[gtest]
    fn kebab_lower() {
        expect_that!("opt_flag".to_case(LabelCase::KebabLower), eq("opt-flag"));
    }

    #[gtest]
    fn kebab_upper() {
        expect_that!(
            "opt_flag".to_case(LabelCase::KebabCapitalized),
            eq("Opt-Flag")
        );
    }

    #[gtest]
    fn kebab_all_caps() {
        expect_that!("opt_flag".to_case(LabelCase::KebabAllCaps), eq("OPT-FLAG"));
    }

    #[gtest]
    fn snake_lower_is_the_identity_on_an_already_snake_case_ident() {
        expect_that!("opt_flag".to_case(LabelCase::SnakeLower), eq("opt_flag"));
    }

    #[gtest]
    fn snake_upper() {
        expect_that!(
            "opt_flag".to_case(LabelCase::SnakeCapitalized),
            eq("Opt_Flag")
        );
    }

    #[gtest]
    fn snake_all_caps() {
        expect_that!("opt_flag".to_case(LabelCase::SnakeAllCaps), eq("OPT_FLAG"));
    }

    #[gtest]
    fn title() {
        expect_that!("opt_flag".to_case(LabelCase::Title), eq("Opt Flag"));
    }

    #[gtest]
    fn lower() {
        expect_that!("opt_flag".to_case(LabelCase::Lower), eq("opt flag"));
    }

    #[gtest]
    fn all_caps_separates_words_with_spaces_not_underscores() {
        expect_that!("opt_flag".to_case(LabelCase::AllCaps), eq("OPT FLAG"));
    }

    #[gtest]
    fn every_case_handles_a_single_word_ident_without_a_stray_separator() {
        expect_that!("name".to_case(LabelCase::CamelCapitalized), eq("Name"));
        expect_that!("name".to_case(LabelCase::SnakeAllCaps), eq("NAME"));
        expect_that!("name".to_case(LabelCase::Title), eq("Name"));
        expect_that!("name".to_case(LabelCase::AllCaps), eq("NAME"));
    }
}
