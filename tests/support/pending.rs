//! The conformance cases this crate does not pass yet, in one place. `tests/conformance.rs` fails
//! when one of them passes, so the list can only shrink. The DXF read-back skips them too.

/// Cases not yet passing: the standard 1.2 cases (vertical exaggeration). Issue #4 empties this
/// list.
pub const EXPECTED_TO_FAIL: &[&str] = &[
    "err-exaggeration-circle-arc",
    "err-exaggeration-dimension-vertical",
    "err-exaggeration-frame-low",
    "err-exaggeration-image",
    "err-exaggeration-north",
    "err-exaggeration-no-scale-bar",
    "err-exaggeration-rotated",
    "err-exaggeration-symbol",
    "err-exaggeration-vertical-ratio",
    "err-version-1-1-exaggeration",
    "exaggeration-dimension-leader",
    "exaggeration-fit",
    "exaggeration-long-section",
    "exaggeration-title-block-as-shown",
];
