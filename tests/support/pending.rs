//! The conformance cases this crate does not pass yet, in one place. `tests/conformance.rs` fails
//! when one of them passes, so the list can only shrink. The DXF read-back skips them too.

/// Cases not yet passing: the standard 1.1 cases (image, colour print profile, map ratios). Issue #1
/// empties this list.
pub const EXPECTED_TO_FAIL: &[&str] = &[
    "err-colour-monochrome",
    "err-img-box",
    "err-img-hash",
    "err-img-monochrome",
    "err-img-outside-frame",
    "err-img-rotated",
    "err-version-1-0-colour",
    "err-version-1-0-image",
    "err-version-1-0-map-scale",
    "img-colour-basic",
    "img-plates",
    "img-sheet-colour",
    "map-scale-explicit",
    "map-scale-fit",
];
