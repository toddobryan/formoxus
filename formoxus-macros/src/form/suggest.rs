//! "did you mean" — shared by the unknown-widget and unknown-button-type
//! messages, which are the same three cases over different name tables.

/// `DatetimeLocal` -> `datetime_local`. Only good enough to recognise the
/// `WidgetType`/`InputType` spellings an author might copy from the enum.
pub(crate) fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Levenshtein distance, for "did you mean". Small inputs, so the simple
/// two-row version is plenty.
pub(crate) fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            #[allow(clippy::bool_to_int_with_if)]
            let cost = if ca == *cb { 0 } else { 1 };
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}
