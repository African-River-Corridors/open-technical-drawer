# Agent instructions

## Agent skills

### Issue tracker

GitHub Issues on this repo, via the `gh` CLI. The repo is public: no confidential names or data. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default triage labels, each named as its role. See `docs/agents/triage-labels.md`.

### Build process

How ticketed work is built and merged. See `docs/agents/build-process.md`.

- **Pre-push hook.** `arc-prepush` (ARC workflows repo) runs `cargo fmt --check` on the crates a push changes, plus the release guard (tree and history), on every `git push`. Install it once per clone with `arc-prepush install` (it covers every worktree). If it blocks, run the fix it prints, commit, and push again; do not `--no-verify` around it.

## Rules for the crate

- Domain-free: no vessel, waterway or other tool vocabulary in the code. A tool's own checks live in that tool.
- Pure core: no file, clock or network access. Every transcendental goes through `libm`.
- The vendored standard changes only through `tools/vendor_standard.py`, in its own pull request.
- Run `python3 tools/release_guard.py` and `--history` before every push.
