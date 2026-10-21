# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
