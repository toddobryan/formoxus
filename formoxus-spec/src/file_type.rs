use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    result::Result,
    str::FromStr,
};

/// Why a string is not a [`FileType`]. Its one use is its message, through
/// `Display`: `form!` turns it into a compile error, and a runtime caller
/// panics with it. The field is private so every message comes from this
/// file, in the one form "`{entry}` is not a file type: {reason}".
#[derive(Clone, Debug)]
pub struct FileTypeError(String);

impl Error for FileTypeError {}

impl FileTypeError {
    const EMPTY: &str = "an empty string is not a legal file type";
    const COMMA: &str =
        "a comma is not allowed in a file type; you can parse a list with `FileType::parse_list`";
    const NOTHING_AFTER_DOT: &str = "an extension needs at least one character after the `.`";
    const EXTENSION_WHITESPACE: &str = "an extension cannot contain whitespace";
    const NOT_ONE_OF_THE_FORMS: &str = "a file type is an extension starting with `.`; \
        a MIME type like `application/pdf`; or one of `audio/*`, `video/*` or `image/*`";
    const EVERY_FILE: &str =
        "`accept` has no wildcard for every file; leave `accept:` off to allow any file";
    const NOT_A_WILDCARD: &str = "`*` is a wildcard only in `audio/*`, `video/*` and `image/*`";
    const MIME_WHITESPACE: &str = "a MIME type cannot contain whitespace";
    const SECOND_SLASH: &str = "a MIME type has only one `/`";
    const MISSING_HALF: &str = "a MIME type needs a type before its `/` and a subtype after it";

    fn error(entry: &str, reason: &str) -> Self {
        Self(format!("`{entry}` is not a file type: {reason}"))
    }
}

impl Display for FileTypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(&self.0)
    }
}

/// One entry of a file input's `accept`: an extension (`.pdf`), a MIME type
/// (`application/pdf`) or one of the three wildcards (`image/*`).
///
/// Made only by parsing (`".pdf".parse::<FileType>()`), so every value has
/// passed the checks in `FromStr` and is ASCII-lowercased. The field is private
/// for that reason: a public variant would let `Extension(".PDF")` skip both.
#[derive(Clone, Debug, PartialEq)]
pub struct FileType(Kind);

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    /// `.pdf`: the dot included, ASCII-lowercased.
    Extension(String),
    /// `application/pdf`: ASCII-lowercased, with no parameters.
    Mime(String),
    /// `image` for `image/*`: one of `audio`, `video` and `image`.
    AnyOf(String),
}

impl FileType {
    /// The types that have a wildcard, `audio/*` and so on. HTML has these
    /// three and no others.
    const WILDCARD_TYPES: [&str; 3] = ["audio", "video", "image"];

    /// The symbols among HTTP's token code points. A token is these and ASCII
    /// letters and digits, nothing else. `*` is one, but formoxus refuses it
    /// outside the three wildcards before this is consulted.
    const TOKEN_SYMBOLS: &str = "!#$%&'*+-.^_`|~";

    /// We're committed to an Extension or error at this point.
    /// We've already trimmed the string and it starts with '.'.
    fn try_extension(s: &str) -> Result<Self, FileTypeError> {
        // What follows the dot, and the same with any whitespace straight after
        // the dot removed. `word` can't be empty: `s` is trimmed and isn't ".".
        // Both come from `s`, not lowercased, so a suggestion keeps the dev's case.
        let after_dot = &s[1..];
        let word = after_dot.trim_ascii_start();
        if s == "." {
            Err(FileTypeError::error(s, FileTypeError::NOTHING_AFTER_DOT))
        } else if word.len() < after_dot.len() && !word.contains(|c: char| c.is_ascii_whitespace())
        {
            // Whitespace straight after the dot and nowhere else: a stray space.
            Err(Self::did_you_mean(s, &format!(".{word}")))
        } else if s.contains(|c: char| c.is_ascii_whitespace()) {
            Err(FileTypeError::error(s, FileTypeError::EXTENSION_WHITESPACE))
        } else {
            Ok(FileType(Kind::Extension(s.to_ascii_lowercase())))
        }
    }

    /// We're committed to a MIME type or error at this point. `s` is trimmed,
    /// holds a `/` and no comma, and isn't one of the three wildcards.
    fn try_mime_type(s: &str) -> Result<Self, FileTypeError> {
        if let Some((meant, _)) = s.split_once(';') {
            let meant = meant.trim_ascii();
            return Err(FileTypeError::error(
                s,
                &format!("`accept` doesn't take MIME parameters; did you mean `{meant}`?"),
            ));
        }
        if s == "*/*" {
            return Err(FileTypeError::error(s, FileTypeError::EVERY_FILE));
        }
        if s.contains(|c: char| c.is_ascii_whitespace()) {
            return Err(FileTypeError::error(s, FileTypeError::MIME_WHITESPACE));
        }
        let Some((ty, subtype)) = s.split_once('/') else {
            unreachable!("try_mime_type is only called on an entry with a `/`");
        };
        if subtype.contains('/') {
            return Err(FileTypeError::error(s, FileTypeError::SECOND_SLASH));
        }
        if ty.is_empty() || subtype.is_empty() {
            return Err(FileTypeError::error(s, FileTypeError::MISSING_HALF));
        }
        if ty.contains('*') {
            return Err(FileTypeError::error(
                s,
                &format!(
                    "{}; list the types you want, or their extensions",
                    FileTypeError::NOT_A_WILDCARD
                ),
            ));
        }
        if subtype.contains('*') {
            return Err(FileTypeError::error(
                s,
                &format!(
                    "{}; list the `{ty}/` types you want, or their extensions",
                    FileTypeError::NOT_A_WILDCARD
                ),
            ));
        }
        // By `char`, not by byte, so a non-ASCII character is named whole.
        if let Some(bad) = s
            .chars()
            .find(|&c| c != '/' && !c.is_ascii_alphanumeric() && !Self::TOKEN_SYMBOLS.contains(c))
        {
            return Err(FileTypeError::error(
                s,
                &format!("`{bad}` cannot appear in a MIME type"),
            ));
        }
        Ok(FileType(Kind::Mime(s.to_ascii_lowercase())))
    }

    /// We're committed to an error at this point: `s` is trimmed, isn't empty,
    /// and has no `.` at its start, no `/` and no comma. Suggest what it looks
    /// like it meant, if anything.
    fn no_slash_error(s: &str) -> FileTypeError {
        let lower = s.to_ascii_lowercase();
        if s == "*" {
            FileTypeError::error(s, FileTypeError::EVERY_FILE)
        } else if Self::WILDCARD_TYPES.contains(&lower.as_str()) {
            Self::did_you_mean(s, &format!("{s}/*"))
        } else if s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
        {
            // The shape of an extension, missing its dot.
            Self::did_you_mean(s, &format!(".{s}"))
        } else {
            FileTypeError::error(s, FileTypeError::NOT_ONE_OF_THE_FORMS)
        }
    }

    fn did_you_mean(entry: &str, meant: &str) -> FileTypeError {
        FileTypeError::error(entry, &format!("did you mean `{meant}`?"))
    }
}

impl FromStr for FileType {
    type Err = FileTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // HTML trims ASCII whitespace only; `trim` would take a no-break space too.
        let s = s.trim_ascii();
        let lower = s.to_ascii_lowercase();
        if s.is_empty() {
            Err(FileTypeError(FileTypeError::EMPTY.to_string()))
        } else if s.contains(',') {
            Err(FileTypeError::error(s, FileTypeError::COMMA))
        } else if s.starts_with('.') {
            Self::try_extension(s)
        } else if let Some(ty) = lower
            .strip_suffix("/*")
            .filter(|ty| Self::WILDCARD_TYPES.contains(ty))
        {
            Ok(FileType(Kind::AnyOf(ty.to_string())))
        } else if s.contains('/') {
            Self::try_mime_type(s)
        } else {
            Err(Self::no_slash_error(s))
        }
    }
}

#[cfg(test)]
mod tests {
    //! `FromStr` for ONE `accept` entry; `parse_list` (commas, duplicates) and
    //! matching get their own tests once they exist. The rules are
    //! `WIDGETS_PLAN.md` 11e item 3, "What `accept` allows". Every error but the empty one is
    //! "`{entry}` is not a file type: {reason}", with the entry trimmed.
    //!
    //! These compare whole messages, because the reason is what a dev reads in
    //! `form!`'s compile error. Where the entry shows what the dev meant, the
    //! reason suggests the fix ("did you mean `.pdf`?"), the way rustc's
    //! `help:` does (Todd, 2026-11-14).
    use super::{FileType, FileTypeError, Kind};
    use googletest::prelude::*;

    fn parse(entry: &str) -> std::result::Result<FileType, String> {
        entry.parse::<FileType>().map_err(|e| e.to_string())
    }

    fn extension(s: &str) -> FileType {
        FileType(Kind::Extension(s.to_string()))
    }

    fn mime(s: &str) -> FileType {
        FileType(Kind::Mime(s.to_string()))
    }

    fn any_of(s: &str) -> FileType {
        FileType(Kind::AnyOf(s.to_string()))
    }

    fn error(entry: &str, reason: &str) -> std::result::Result<FileType, String> {
        Err(format!("`{entry}` is not a file type: {reason}"))
    }

    fn did_you_mean(entry: &str, meant: &str) -> std::result::Result<FileType, String> {
        error(entry, &format!("did you mean `{meant}`?"))
    }

    /// The reasons that suggest nothing, once each, so a test reads as which
    /// rule an entry breaks.
    const NOT_ONE_OF_THE_FORMS: &str = "a file type is an extension starting with `.`; \
        a MIME type like `application/pdf`; or one of `audio/*`, `video/*` or `image/*`";
    const NOTHING_AFTER_THE_DOT: &str = "an extension needs at least one character after the `.`";
    const EXTENSION_WHITESPACE: &str = "an extension cannot contain whitespace";
    const MIME_WHITESPACE: &str = "a MIME type cannot contain whitespace";
    const MISSING_HALF: &str = "a MIME type needs a type before its `/` and a subtype after it";
    const SECOND_SLASH: &str = "a MIME type has only one `/`";
    const EVERY_FILE: &str =
        "`accept` has no wildcard for every file; leave `accept:` off to allow any file";
    const A_LIST: &str =
        "a comma is not allowed in a file type; you can parse a list with `FileType::parse_list`";

    // ---- extensions ----

    #[gtest]
    fn an_extension_is_stored_ascii_lowercase() {
        expect_that!(parse(".pdf"), eq(&Ok(extension(".pdf"))));
        expect_that!(parse(".PDF"), eq(&Ok(extension(".pdf"))));
        expect_that!(parse(".DocX"), eq(&Ok(extension(".docx"))));
    }

    /// HTML folds A-Z only. `to_lowercase` would turn `Ñ` into `ñ`, and then
    /// `.Ñame` would match a file named `x.ñame`, which a browser's picker
    /// would not offer.
    #[gtest]
    fn an_extension_folds_only_ascii_letters() {
        expect_that!(parse(".ÑAME"), eq(&Ok(extension(".Ñame"))));
    }

    /// HTML restricts nothing after the dot, and neither does formoxus beyond
    /// "something" and "no whitespace". `.*` is a literal extension, not a
    /// wildcard: harmless, since it matches only a name ending in `.*`.
    #[gtest]
    fn an_extension_may_hold_anything_after_its_dot() {
        for entry in [".tar.gz", ".c++", ".ñ", ".*", ".a/b", ".x;y"] {
            expect_that!(parse(entry), eq(&Ok(extension(entry))), "{entry}");
        }
    }

    /// formoxus's own rule, not HTML's: a bare `.` would match every name that
    /// ends in a dot.
    #[gtest]
    fn a_bare_dot_is_not_an_extension() {
        expect_that!(parse("."), eq(&error(".", NOTHING_AFTER_THE_DOT)));
        expect_that!(parse("  .  "), eq(&error(".", NOTHING_AFTER_THE_DOT)));
    }

    /// F. Whitespace between the dot and a word is a stray space: the fix is
    /// plain, so it is suggested.
    #[gtest]
    fn whitespace_after_the_dot_suggests_removing_it() {
        expect_that!(parse(". pdf"), eq(&did_you_mean(". pdf", ".pdf")));
        expect_that!(parse(".\tpdf"), eq(&did_you_mean(".\tpdf", ".pdf")));
        expect_that!(
            parse(".  tar.gz"),
            eq(&did_you_mean(".  tar.gz", ".tar.gz"))
        );
    }

    /// formoxus's own rule: the entry is trimmed, so an inner space is a typo.
    /// Anywhere but straight after the dot, the fix is not plain (`.tar gz`
    /// could be `.tar.gz` or `.targz`), so nothing is suggested. That includes
    /// a space after the dot when there is another one later.
    #[gtest]
    fn other_whitespace_in_an_extension_is_refused() {
        for entry in [".tar gz", ".tar\tgz", ". tar gz"] {
            expect_that!(
                parse(entry),
                eq(&error(entry, EXTENSION_WHITESPACE)),
                "{entry:?}"
            );
        }
    }

    /// A. No `/` and no leading `.`, but it would be a fine extension with one.
    /// The suggestion keeps the dev's own case: `.PDF` is valid too.
    #[gtest]
    fn an_extension_without_its_dot_suggests_the_dot() {
        for (entry, meant) in [
            ("pdf", ".pdf"),
            ("docx", ".docx"),
            ("PDF", ".PDF"),
            ("tar.gz", ".tar.gz"),
        ] {
            expect_that!(parse(entry), eq(&did_you_mean(entry, meant)), "{entry}");
        }
    }

    /// Without a `/` and without the shape of an extension, there is nothing
    /// sensible to suggest, so the reason lists the three forms. The
    /// no-break space is not ASCII whitespace, so it is not trimmed, and this
    /// entry does not start with `.`.
    #[gtest]
    fn an_entry_that_is_none_of_the_forms_lists_them() {
        for entry in ["\u{a0}.pdf", "my file", "pdf?"] {
            expect_that!(
                parse(entry),
                eq(&error(entry, NOT_ONE_OF_THE_FORMS)),
                "{entry:?}"
            );
        }
    }

    // ---- wildcards ----

    #[gtest]
    fn the_three_wildcards_are_any_of_their_type() {
        expect_that!(parse("audio/*"), eq(&Ok(any_of("audio"))));
        expect_that!(parse("video/*"), eq(&Ok(any_of("video"))));
        expect_that!(parse("image/*"), eq(&Ok(any_of("image"))));
    }

    #[gtest]
    fn a_wildcard_is_case_insensitive() {
        expect_that!(parse("IMAGE/*"), eq(&Ok(any_of("image"))));
        expect_that!(parse("Video/*"), eq(&Ok(any_of("video"))));
    }

    /// B. A wildcard's type alone. This is checked before A: `image` is far
    /// likelier to mean `image/*` than a file named `x.image`. The suggestion
    /// keeps the dev's own case, which is still a valid wildcard.
    #[gtest]
    fn a_wildcards_type_alone_suggests_the_wildcard() {
        for (entry, meant) in [
            ("image", "image/*"),
            ("audio", "audio/*"),
            ("video", "video/*"),
            ("Image", "Image/*"),
        ] {
            expect_that!(parse(entry), eq(&did_you_mean(entry, meant)), "{entry}");
        }
    }

    /// C. HTML has no "any file", because leaving `accept` off already is.
    #[gtest]
    fn a_wildcard_for_every_file_says_to_leave_accept_off() {
        for entry in ["*", "*/*"] {
            expect_that!(parse(entry), eq(&error(entry, EVERY_FILE)), "{entry}");
        }
    }

    /// D. formoxus's own rule, stricter than HTML on purpose: HTML reads these
    /// as literal MIME types that never match, so every file would be refused
    /// with no hint why. The reason names the dev's type, so they know which
    /// list to go and make.
    #[gtest]
    fn a_star_anywhere_else_in_a_mime_type_says_which_types_to_list() {
        for (entry, ty) in [
            ("text/*", "text"),
            ("image/pn*", "image"),
            ("application/*+json", "application"),
            ("Text/*", "Text"),
        ] {
            let reason = format!(
                "`*` is a wildcard only in `audio/*`, `video/*` and `image/*`; \
                 list the `{ty}/` types you want, or their extensions"
            );
            expect_that!(parse(entry), eq(&error(entry, &reason)), "{entry}");
        }
    }

    /// D, when the `*` is in the type itself: there is no type to name.
    #[gtest]
    fn a_star_in_the_type_says_to_list_the_types() {
        for entry in ["*/png", "im*/png"] {
            let reason = "`*` is a wildcard only in `audio/*`, `video/*` and `image/*`; \
                          list the types you want, or their extensions";
            expect_that!(parse(entry), eq(&error(entry, reason)), "{entry}");
        }
    }

    // ---- MIME types ----

    #[gtest]
    fn a_mime_type_is_stored_ascii_lowercase() {
        expect_that!(parse("application/pdf"), eq(&Ok(mime("application/pdf"))));
        expect_that!(parse("Text/Plain"), eq(&Ok(mime("text/plain"))));
        expect_that!(parse("IMAGE/PNG"), eq(&Ok(mime("image/png"))));
    }

    /// Only the HTTP token characters are checked, not registration: an
    /// unregistered `x-` type and the typo `pfd` are both valid HTML.
    #[gtest]
    fn a_mime_type_need_not_be_registered() {
        for entry in [
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/x-zip-compressed",
            "application/pfd",
        ] {
            expect_that!(parse(entry), eq(&Ok(mime(entry))), "{entry}");
        }
    }

    /// Every HTTP token code point but `*`, which formoxus refuses (above).
    #[gtest]
    fn a_mime_type_may_use_every_token_symbol_but_star() {
        let entry = "a0!#$%&'+-.^_`|~/z9!#$%&'+-.^_`|~";
        expect_that!(parse(entry), eq(&Ok(mime(entry))));
    }

    /// E. The suggestion is the entry up to the `;`, trimmed: what the dev
    /// meant, in their own case. `;` is checked before `*`, so a wildcard with
    /// a parameter is told about the parameter and offered the wildcard.
    #[gtest]
    fn a_mime_type_with_parameters_suggests_it_without_them() {
        for (entry, meant) in [
            ("text/plain; charset=utf-8", "text/plain"),
            ("text/plain;charset=utf-8", "text/plain"),
            ("text/plain ;", "text/plain"),
            ("Text/Plain;", "Text/Plain"),
            ("image/*;q=1", "image/*"),
        ] {
            let reason = format!("`accept` doesn't take MIME parameters; did you mean `{meant}`?");
            expect_that!(parse(entry), eq(&error(entry, &reason)), "{entry}");
        }
    }

    #[gtest]
    fn a_mime_type_cannot_contain_whitespace() {
        for entry in ["text/ plain", "text /plain", "text/pl\tain"] {
            expect_that!(
                parse(entry),
                eq(&error(entry, MIME_WHITESPACE)),
                "{entry:?}"
            );
        }
    }

    #[gtest]
    fn a_mime_type_needs_both_halves() {
        for entry in ["text/", "/plain", "/"] {
            expect_that!(parse(entry), eq(&error(entry, MISSING_HALF)), "{entry}");
        }
    }

    #[gtest]
    fn a_mime_type_has_one_slash() {
        for entry in ["a/b/c", "text//plain", "text/plain/"] {
            expect_that!(parse(entry), eq(&error(entry, SECOND_SLASH)), "{entry}");
        }
    }

    /// I. The reason names the character, so the dev need not hunt for it.
    /// `ë` must come out as `ë`: going through the entry byte by byte would
    /// name one of its two UTF-8 bytes, read as a Latin-1 character, instead.
    #[gtest]
    fn a_mime_type_names_a_character_it_cannot_hold() {
        for (entry, bad) in [
            ("text/pl@in", '@'),
            ("text/plain\"", '"'),
            ("text/(plain)", '('),
            ("text:x/plain", ':'),
            ("tëxt/plain", 'ë'),
            // Not ASCII whitespace, so neither trimmed nor "whitespace": just
            // a character HTML doesn't allow.
            ("text/\u{a0}plain", '\u{a0}'),
        ] {
            expect_that!(
                parse(entry),
                eq(&error(
                    entry,
                    &format!("`{bad}` cannot appear in a MIME type")
                )),
                "{entry:?}"
            );
        }
    }

    // ---- the whole entry ----

    /// HTML trims ASCII whitespace (tab, LF, FF, CR, space) from each entry,
    /// and the error shows the trimmed entry.
    #[gtest]
    fn an_entry_is_trimmed_of_ascii_whitespace() {
        expect_that!(parse("  .pdf  "), eq(&Ok(extension(".pdf"))));
        expect_that!(parse("\t\n\x0c\rimage/*\r\n"), eq(&Ok(any_of("image"))));
        expect_that!(parse(" text/plain\t"), eq(&Ok(mime("text/plain"))));
        expect_that!(parse("  pdf  "), eq(&did_you_mean("pdf", ".pdf")));
    }

    #[gtest]
    fn an_empty_string_is_refused() {
        for entry in ["", "   ", "\t"] {
            expect_that!(
                parse(entry),
                eq(&Err("an empty string is not a legal file type".to_string())),
                "{entry:?}"
            );
        }
    }

    /// G. HTML never hands one entry a comma, since it splits on them first.
    /// But `FromStr` is public, so `".pdf,.doc"` would otherwise parse as ONE
    /// extension that no file name ends with. The comma is checked first,
    /// before any rule that would give a vaguer reason.
    #[gtest]
    fn a_comma_says_to_use_parse_list() {
        for entry in [
            ".pdf,.doc",
            ".pdf, .doc",
            "text/plain,text/html",
            "image/*,",
        ] {
            expect_that!(parse(entry), eq(&error(entry, A_LIST)), "{entry}");
        }
    }

    #[gtest]
    fn the_error_is_a_std_error() {
        fn is_error<E: std::error::Error>() {}
        is_error::<FileTypeError>();
    }
}
