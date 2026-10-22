# open-technical-drawer

`otd` is the [Drawing Format Standard](https://github.com/African-River-Corridors/technical_drawings_for_agents)
in Rust. A drawing document goes in. Normalised content comes out, and from it SVG, PDF and DXF.

- **Domain-free.** `otd` knows sheets, viewports, entities, dimensions, hatches, the title block and
  the writers. It knows nothing about what is drawn. The tool that uses it owns its own checks.
- **Pure.** The core opens no file, reads no clock and uses no network. A document that names a file
  gets a `Files` reader from the caller.
- **The same on every platform.** Every transcendental goes through `libm`.
- **Checked against the standard.** The standard's conformance suite is vendored under `standard/`,
  pinned by commit and hash in `standard/PIN.json`. Every case must pass.

It began as the `ovcd-draw` crate of
[open-vessel-concept-drawings](https://github.com/African-River-Corridors/open-vessel-concept-drawings)
(OVCD) and moved here so that more than one tool can draw through it. OVCD and
[open-waterway-designer](https://github.com/African-River-Corridors/open-waterway-designer) use it.

## Use it

`otd` is not on crates.io. Pin it by git tag:

```toml
[dependencies]
otd = { git = "https://github.com/African-River-Corridors/open-technical-drawer", tag = "v0.1.0" }
```

```rust
let result = otd::run_text(&drawing_document_json, &otd::NoFiles);
```

`run_text` and `run_value` return the normalised content, or the errors the standard defines. The
writers are in `otd::render` (`svg`, `pdf`, `dxf`).

## Standard version

| `otd` | Drawing Format Standard |
|---|---|
| 0.1.x | 1.0 (`4936998`) |
| 0.2.x, 0.3.x | 1.1 (`a2021a8`) |
| unreleased | 1.2 (`1d16f5c`) |

Each version reads documents of its own minor version and every earlier one (clause 18.2).
Standard 1.1 adds the image entity, the colour print profile and the map ratios. Standard 1.2 adds
the vertical exaggeration of a viewport.

## Build and test

```
cargo test
python3 tools/release_guard.py && python3 tools/release_guard.py --history
```

The toolchain is pinned in `rust-toolchain.toml`. To move the vendored standard to a new commit, run
`python3 tools/vendor_standard.py --from <clone of the standard> --commit <sha>`, read the diff, and
say why in the pull request.

## Licence

Apache-2.0. The vendored font under `standard/fonts/` has its own licence, beside it.
