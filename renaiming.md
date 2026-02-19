# KubeTile Rename Plan

1. Update the root manifest and metadata to the new application name:
   - change the `[package]` name (if present) from `crystal` to `kubetile`, adjust the description/title to mention KubeTile, and update any `package.metadata` fields that echo the old name.
   - refresh the `[workspace]` `members` list so it points to the soon-to-be-renamed directories, and document the new crate-naming convention (e.g., `kubetile-*`).

2. Rename `crystal-core` to `kubetile-core`:
   - rename the directory under `crates/`, update its `Cargo.toml` `name` field, and review its `dependencies`/`dev-dependencies` for other `crystal-*` references that need rewriting.
   - search the rest of the repo for references to `crystal-core` (module imports, feature flags, docs) and plan targeted replacements.

3. Rename `crystal-config` to `kubetile-config` following the same sub-steps as above for directory, `Cargo.toml`, and cross-crate references.

4. Rename `crystal-app` to `kubetile-app` with the same method plus checking binary targets, runtime assets, and entry-point names.

5. Rename `crystal-terminal` to `kubetile-terminal`, ensuring any terminal-related scripts or aliases update to the new crate name.

6. Rename `crystal-tui` to `kubetile-tui`, updating its dependencies and document references (e.g., CLI help text) to mention KubeTile.

7. Once all crate names are adjusted, search and replace lingering literal strings like `Crystal`, `crystal`, or `CRYSTAL` in documentation (`README.md`, `docs/`, `pages/`), configuration files, and scripts so they reference KubeTile instead.

8. Update supporting tooling such as `Makefile`, CI configs, `PRD.md`, and `demo-stand` instructions to reflect the new name.

9. Regenerate `Cargo.lock`, re-run `cargo fmt`, and run the primary smoke test suite (or `cargo test`) to confirm workspace-wide consistency after the rename.
