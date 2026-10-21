# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Drawing Format Standard 1.1 (vendored at `a2021a8`). The crate now reads documents of version 1.0
  and 1.1:
  - the `image` entity (clause 9.10): a JPEG placed in model space, checked by its frame header and
    its SHA-256, drawn first in every output. The SVG and the PDF hold the JPEG bytes as they are;
    the DXF links the file with an IMAGE and an IMAGEDEF (clause 15.4.12);
  - the `colour` print profile (clause 8.6.4) and the colour members of a layer: `print_colour`,
    `fill_opacity` and `halo_colour` (clauses 8.6.5 and 8.6.6), with white plates under the scale bar
    and north arrow of a viewport that holds an image (clause 8.6.7);
  - the map ratios, 1:200000 to 1:10000000 (clause 7.3.5), which `fit` tries last;
  - error `18.2.1` for a member of version 1.1 in a document of version 1.0.
- `Files::read_bytes`, with a default that reads the file as text. A caller that gives images must
  override it.
- `Entity::paint` (the colour members), `Shape::Image`, `SheetInfo::colour` and `Normalised::images`
  (the image bytes for the renderers, not part of the normalised content).
- `rules::ratios(minor)`, `rules::fit_scale_in` and `rules::MAP_RATIOS`. `rules::fit_scale` is
  unchanged: it tries the 1.0 ratios.
- The schema checker reads a pattern that starts with negative look-aheads (`^(?!A)REST`), which the
  image path of the 1.1 schema uses.

### Changed
- Nothing in the output of a document of version 1.0: its normalised content, SVG, PDF and DXF are
  the same bytes as in 0.1.0 (checked for the 182 documents of version 1.0 in the suite). A tool that
  matches on `Shape` without a wildcard arm must add `Shape::Image`.

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
