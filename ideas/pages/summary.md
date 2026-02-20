# KubeTile GitHub Pages — Chosen Approach

**Decision:** `gh-pages` branch + **mdBook** (standard Rust ecosystem tooling)

All sources agree this is the best fit for a Rust project at growth stage.

---

## Setup

1. Create a `book/` directory in `main`:
   ```
   book/
     book.toml
     src/
       SUMMARY.md
       introduction.md
       installation.md
       keybindings.md
       plugins.md
   ```

2. Add `.github/workflows/pages.yml`:
   ```yaml
   on:
     push:
       branches: [main]
   jobs:
     deploy:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
         - uses: peaceiris/actions-mdbook@v2
         - run: mdbook build book
         - uses: peaceiris/actions-gh-pages@v4
           with:
             github_token: ${{ secrets.GITHUB_TOKEN }}
             publish_dir: book/book
   ```

3. In GitHub Settings → Pages: set source to `gh-pages` branch, root `/`.

---

## Why This Approach

- **Rust ecosystem standard** — same tool as The Rust Book, familiar to contributors
- **Zero manual deploys** — every push to `main` rebuilds and publishes automatically
- **Scales naturally** — add a `.md` file + one line in `SUMMARY.md` to add a page
- **Clean separation** — generated site lives in `gh-pages`, source stays in `main`
- **PR-friendly** — Markdown diffs are readable; docs can ship in the same PR as code

---

## Ongoing Maintenance

- To add a page: create `book/src/<page>.md`, add it to `SUMMARY.md`
- To update content: edit Markdown, push to `main` — CI does the rest
- Pin `peaceiris/actions-mdbook` to a specific version tag to avoid surprise breakage
- Use GitHub Issues with a `docs` label for content change requests
