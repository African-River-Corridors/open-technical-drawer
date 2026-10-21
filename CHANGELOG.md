# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0]

### Added
- The crate, moved from OVCD's `crates/ovcd-draw` (OVCD 0.2.4) and renamed `otd`. The code and the
  library API are the same; only the crate name changed (`ovcd_draw::` becomes `otd::`). It reads
  Drawing Format Standard 1.0 (vendored at `4936998`), and every result, SVG, PDF and DXF is the same
  as before.
- A test that the crate depends on no tool crate and that its source names none.
- CI: format, clippy, the tests, the WebAssembly build, and the release guard.

This repository starts with a fresh history. The crate's earlier history is in OVCD; issue numbers in
older comments refer to OVCD's issues.
