# KubeTile GitHub Pages — Website Approach

## Option 1: Static site in `docs/` on `main` (Simplest)

Keep a `docs/` folder in the main repo. GitHub Pages serves it directly from `main` branch.

**Structure:**
```
docs/
  index.html        # Landing page
  style.css
  assets/
    screenshot.png
    demo.gif
```

**Pros:**
- Zero extra tooling — edit HTML/CSS and push
- No CI pipeline needed
- Docs live alongside code — easy to keep in sync with releases
- Contributors can update docs in the same PR as code changes

**Maintenance:**
- Update `docs/index.html` manually on each release
- Pin version number and changelog link in the page

---

## Option 2: Separate `gh-pages` branch with mdBook (Recommended for growth)

Use [mdBook](https://rust-lang.github.io/mdBook/) — the standard Rust ecosystem docs tool — built automatically by a GitHub Actions workflow and pushed to a `gh-pages` branch.

**Structure (in `main`):**
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

**GitHub Actions workflow** (`.github/workflows/pages.yml`):
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

**Pros:**
- Full documentation site: guides, keybinding reference, plugin API
- Automatically rebuilt on every push to `main` — always up to date
- Fits the Rust ecosystem perfectly (same tool as The Rust Book)
- Markdown source is easy to edit and review in PRs

**Maintenance:**
- Add a new `.md` file and one line in `SUMMARY.md` to add a page
- No manual deploy steps — CI handles everything
