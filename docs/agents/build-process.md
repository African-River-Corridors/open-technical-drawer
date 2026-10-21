# Build process

1. **Read** the ticket and its comments (`gh issue view <n> --comments`).
2. **Build** on a branch `ticket/<n>-<slug>` from an up-to-date `origin/main`, test first. One pull
   request closes one ticket. Each ticked acceptance criterion links to its evidence: a test in the
   diff, a command and its output in the body, or a CI job.
3. **Local checks** before a push: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test`, `python3 tools/release_guard.py` and `--history`, and the embargo check of the pull
   request body (`issue-tracker.md`).
4. **Merge** with a squash when `test` and `release-guard` are green. Commit with the repo-local
   address in `git config user.email`, not a personal one.
5. **Release.** A version is a tag `vX.Y.Z` on `main` with its CHANGELOG section and a GitHub
   release. The maintainer approves each tag. Tools that use `otd` pin the tag.

A change that moves any normalised content, SVG, PDF or DXF output says so in the CHANGELOG, as the
tools that pin `otd` must re-check their drawings when they move to the new tag.
