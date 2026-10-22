# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.4.0]

### Added
- Drawing Format Standard 1.2 (vendored at `1d16f5c`). The crate now reads documents of version 1.0,
  1.1 and 1.2. `SUPPORTED_VERSION` is `(1, 2)`.
  - the `vertical_exaggeration` member of a viewport (clause 7.3.6): a whole factor `E` of 2 or more.
    The scale is the horizontal one, 1:N, and the vertical scale is 1:(N / E). The map of clause 7.4
    draws model height at `k_v = 1000 / (N / E)`, and fit, window fit and containment use it
    (clauses 7.5.1 and 7.5.2). `fit` tries only the ratios whose vertical ratio is a ratio;
  - the checks of an exaggerated viewport: errors `7.3.6.1` (from the schema) to `7.3.6.9`, `7.9.12`,
    and `18.2.1` for the member in a document of version 1.0 or 1.1;
  - the vertical scale bar and the words "Vertical exaggeration ×E", never initials (clauses 7.9.11
    and 16.6.5), on the origin `viewport.<id>.vertical_scale_bar`. They stay black under the colour
    profile, as the scale bar does;
  - `AS SHOWN` in the title block of a sheet with an exaggerated viewport (clause 14.4.4);
  - in the normalised content, `vertical_exaggeration` on an exaggerated viewport only (clause 16.2);
  - in the DXF, the entities of an exaggerated viewport in paper space, as drawn, and nothing of it in
    model space (clause 15.4.13). Its VIEWPORT is at the horizontal scale, with
    `vertical_exaggeration` in its extended data. The read-back test gives the same normalised
    content.
- `ViewportInfo::vertical_exaggeration` (`Option<u32>`).
- `rules::Mapper::exaggerated` and `Mapper::kv`, `rules::required_size_exaggerated`,
  `rules::window_fits_exaggerated`, `rules::fit_scale_exaggerated`, `rules::vertical_denominator`,
  `rules::vertical_bar_length`, `rules::exaggeration_words`, and the constants `VBAR_BOTTOM`,
  `VBAR_DIVISIONS`, `VBAR_LABEL_GAP`, `VBAR_RATIO_GAP`, `WORDS_BASELINE` and
  `MIN_EXAGGERATED_FRAME_HEIGHT`. `Mapper::new`, `required_size`, `window_fits`, `fit_scale` and
  `fit_scale_in` are unchanged.
- The layout review leaves out the labels of the vertical scale bar on its own hatches, as it does
  for the scale bar.

### Changed
- Nothing in the output of a document of version 1.0 or 1.1: its normalised content, SVG, PDF and
  DXF are the same bytes as in 0.3.0. Checked for the 274 documents of version 1.0 and 1.1 in the
  suite and the schema examples: 273 are the same. The other is the new case
  `err-version-1-1-exaggeration`, a 1.1 document that uses the 1.2 member, which is now error
  `18.2.1`. A tool that builds `ViewportInfo` with a struct literal must add
  `vertical_exaggeration: None`.

## [0.3.0]

### Added
- `otd-imagery`, a crate apart from the core (a workspace member in `imagery/`): the network lives
  there, not in `otd`. It fetches EOxCloudless 2016 tiles (CC BY 4.0, Web Mercator layer
  `s2cloudless_3857`) for a box in model coordinates, given the caller's map to longitude and
  latitude. It samples them by nearest pixel into an `otd::raster::Grid`, for
  `otd::raster::Backdrop::colour`.
  - 2016 only: `Source::parse` refuses every other name and year, as the mosaics from 2018 on are
    CC BY-NC-SA 4.0. `ATTRIBUTION` is the line every sheet that shows the imagery prints.
  - `Cached` keeps tiles on disk in a folder the caller names. No tile is shipped.
  - A failed, slow or bad tile, or a box of more than 400 tiles, is a `FetchError` value to fall
    back on, never a panic. The tests use a synthetic tile and no network.
- CI runs clippy and the tests over the workspace.
- `otd::review`: a layout review of normalised content, `review(&norm, &ReviewOptions) -> Vec<Finding>`.
  Each finding has a severity (error or warning), a rule name, the viewport (or none for the sheet),
  and the entities by place and by `id`. The rules:
  - `text-overlap` (error): two text boxes overlap by more than 0.1 mm;
  - `text-print-size` (error): a text below 1.8 mm high at the print scale (1 by default);
  - `text-on-hatch` (warning): a text box lies partly on a hatch area, leaving out the standard's own
    furniture and the white plates;
  - `outside-frame` (error): a point of a viewport entity lies outside its frame.
  Every sheet of the conformance suite passes with no finding.

### Changed
- No output moves. Since 0.2.0 the core gained `src/review.rs` and one line in `src/lib.rs`, and
  nothing else in `src/` or `standard/` changed, so every document gives the same normalised content,
  SVG, PDF and DXF as in 0.2.0.

## [0.2.0]

### Added
- The raster module, `otd::raster`, for an image behind a drawing (clause 9.10). The caller reads the
  file; the module opens none.
  - `Grid<T>`: cells in model space, row 0 at the top. `clip` keeps the whole cells inside a window,
    so the image lies inside the frame. `resample` covers a box exactly, by nearest cell.
  - `shaded_relief`: a grey relief of a DEM by Horn's slope, with the azimuth, the altitude and a
    vertical scale (`Light`). No data is white. Every transcendental goes through `libm`.
  - `jpeg_grey` and `jpeg_rgb`: baseline JPEG through `jpeg-encoder`, which uses integer arithmetic
    only, so a grid gives the same bytes on every platform. A test pins the SHA-256 of one.
  - `Backdrop`: the JPEG, its path and box, and the `image` entity that places it. `WithImages`
    gives the bytes to `run_text` from memory, so a tool need not write the file first.
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
