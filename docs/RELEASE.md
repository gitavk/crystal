# Release Instructions

These steps document how to prepare and ship a Crystal release. Follow them in order, then coordinate with the release owner (if different) before announcing or publishing the release.

1. **Confirm scope and version**
   - Collect issues/PRs that should land in this release and note the desired semantic-version bump (patch/minor/major).
   - Update `Cargo.toml` (and any workspace members) with the new `version` value and synchronize changelog entries (see `docs/CHANGELOG.md` if available).

2. **Update documentation**
   - Refresh user-facing docs/notes (`README.md`, `docs/`, etc.) to reflect new behaviors or configuration defaults.
   - Add or update release-specific notes (new page, `docs/notes/vX.Y.Z.md`, etc.) describing new features, bug fixes, and remaining known issues.

3. **Run automated verifications**
   - `cargo fmt` to ensure formatting.
   - `cargo clippy --all-targets --all-features` to surface lint issues.
   - `cargo test --all-features` (include `kube` mock/fixtures as needed) to prove functionality.
   - `cargo build --release` (optional but recommended) to guarantee the release build succeeds.
   - If affected, rerun any integration/system tests documented in `docs/tests.md`.

4. **Prepare change summary**
   - Draft a `docs/notes` entry or GitHub release draft summarizing the release highlights, breaking changes, and upgrade tips.
   - Update any release-tracking dashboards or templates (`PRD.md`, release kanban, etc.).

5. **Tagging and packaging**
   - Ensure git history is clean (rebase/squash commits as agreed) and run `git status`.
   - Commit version/docs/test updates with a clear message (e.g., “release: prepare v0.4.0”).
   - Tag the commit: `git tag -a vX.Y.Z -m "Crystal vX.Y.Z"` and push tags `git push origin vX.Y.Z`.
   - If binaries/assets must be published, build them from the release tag and upload to the artifact store (GitHub Releases, internal CDN, etc.).
