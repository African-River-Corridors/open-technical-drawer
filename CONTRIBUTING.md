# Contributing to open-technical-drawer

Thank you for helping. Bug reports, fixes and new parts of the Drawing Format Standard are all welcome.

## Set up and test

You need the Rust toolchain (`rustup` reads `rust-toolchain.toml`) and Python 3.12 or later for the
tools.

```
git clone https://github.com/African-River-Corridors/open-technical-drawer
cd open-technical-drawer
cargo test
python3 tools/release_guard.py
```

Both must pass. CI runs them on every push and pull request.

## Rules for the crate

- `otd` implements the standard. If the standard is unclear or wrong, open an issue on the
  [standard's repository](https://github.com/African-River-Corridors/technical_drawings_for_agents);
  do not work around it here.
- Every change comes with tests. A new clause comes with its conformance cases, vendored from the
  standard.
- The crate stays domain-free and pure (see `AGENTS.md`).

## Pull requests

1. Open an issue first for anything bigger than a small fix.
2. Fork, branch from `main`, keep the change focused.
3. Make sure the tests and the release guard pass.
4. Say what changed and how you checked it.

Branch protection on `main` requires `test` and `release-guard` to pass, and a linear history (squash
merges).

## Licence of contributions

This project is licensed under Apache-2.0. Unless you say otherwise, any contribution you
intentionally submit is licensed under Apache-2.0, as set out in section 5 of the licence.

## Conduct

Please follow the [code of conduct](CODE_OF_CONDUCT.md).
