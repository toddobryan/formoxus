//! The icons the row buttons draw, inlined as SVG.
//!
//! From [Lucide](https://lucide.dev) (ISC license, © Lucide Contributors),
//! each copied as its `<path>` data with Lucide's own `<svg>` attributes.
//! Inlined rather than taken from an icon crate: `dioxus-free-icons`, the
//! obvious one, cost 13.8 s and 65 MB per target to build for these four
//! one-path icons.
//!
//! Every icon is `aria-hidden`. The button it sits in carries the name, as an
//! `aria-label` or as text beside the icon, so the icon has nothing to add.
//! No class: CSS reaches one as `.fx-button svg`.

use dioxus::prelude::*;

/// Lucide `chevron-up`.
pub(crate) const CHEVRON_UP: &[&str] = &["m18 15-6-6-6 6"];
/// Lucide `chevron-down`.
pub(crate) const CHEVRON_DOWN: &[&str] = &["m6 9 6 6 6-6"];
/// Lucide `x`.
pub(crate) const X: &[&str] = &["M18 6 6 18", "m6 6 12 12"];
/// Lucide `plus`.
pub(crate) const PLUS: &[&str] = &["M5 12h14", "M12 5v14"];

/// One icon, 16px, drawn in the button's text color.
pub(crate) fn icon(paths: &'static [&'static str]) -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16",
            height: "16",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            for d in paths {
                path { d: *d }
            }
        }
    }
}
