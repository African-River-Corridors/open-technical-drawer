//! The conformance cases this crate does not pass yet, in one place. `tests/conformance.rs` fails
//! when one of them passes, so the list can only shrink. The DXF read-back skips them too.

/// Cases not yet passing. None: every case of the pinned suite passes.
pub const EXPECTED_TO_FAIL: &[&str] = &[];
