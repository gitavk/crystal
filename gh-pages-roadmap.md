# KubeTile GitHub Pages — Implementation Roadmap

**Strategy: always-live, incremental delivery.**
Each phase ships a complete, working site. No phase leaves the site broken.
Work is done in `main`; GitHub Actions publishes to `gh-pages` automatically.

---

## Context

- **Repo:** KubeTile (Rust Kubernetes TUI)
- **Tool:** mdBook — the Rust ecosystem standard for project documentation
- **Branch model:** source in `main`, built site in `gh-pages`
- **Existing:** `.github/workflows/ci.yml` (Rust CI), `pages/` dir with draft HTML (on gh-pages branch, safe to remove once mdBook is live)
- **Current branch for docs work:** `main`

---

## Phase 0 — Bootstrap: Site goes live (minimal landing page)

**Goal:** A real URL, a real page. Scaffold only; content is placeholder text.

### Step 0.1 — Create the book directory structure

In `main`, create the following files:

**`book/book.toml`**
```toml
[book]
title = "KubeTile"
authors = ["KubeTile contributors"]
language = "en"
multilingual = false
src = "src"

[output.html]
git-repository-url = "https://github.com/YOUR_USERNAME/KubeTile"
edit-url-template = "https://github.com/YOUR_USERNAME/KubeTile/edit/main/book/src/{path}"
```
> Replace `YOUR_USERNAME` with the actual GitHub username/org.

**`book/src/SUMMARY.md`**
```markdown
# Summary

- [Introduction](introduction.md)
```

**`book/src/introduction.md`**
```markdown
# KubeTile

KubeTile is a keyboard-driven, plugin-based Kubernetes TUI written in Rust.

> Documentation is a work in progress. More pages coming soon.
```

### Step 0.2 — Add the GitHub Actions deploy workflow

Create **`.github/workflows/pages.yml`**:
```yaml
name: Deploy Docs

on:
  push:
    branches: [main]
    paths:
      - 'book/**'
      - '.github/workflows/pages.yml'

permissions:
  contents: write

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup mdBook
        uses: peaceiris/actions-mdbook@v2
        with:
          mdbook-version: '0.4.40'

      - name: Build docs
        run: mdbook build book

      - name: Deploy to gh-pages
        uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: book/book
          force_orphan: false
```

> `paths:` filter means this workflow only fires when docs actually change,
> not on every Rust commit.

### Step 0.3 — Configure GitHub Pages source

In **GitHub → Settings → Pages**:
- Source: `Deploy from a branch`
- Branch: `gh-pages`, folder: `/ (root)`

### Step 0.4 — Commit and verify

```bash
git add book/ .github/workflows/pages.yml
git commit -m "docs: bootstrap mdBook skeleton"
git push origin main
```

**Verify:** GitHub Actions tab → `Deploy Docs` workflow runs green. Site is live at `https://YOUR_USERNAME.github.io/KubeTile/`.

---

## Phase 1 — Core Content: Installation & Keybindings

**Goal:** Users can find the two most critical pieces of information.

### Step 1.1 — Add installation page

Create **`book/src/installation.md`**:
```markdown
# Installation

## From source (requires Rust ≥ 1.75)

```bash
git clone https://github.com/YOUR_USERNAME/KubeTile
cd KubeTile
cargo build --release
# Binary is at target/release/kubetile
```

## Requirements

- A valid `~/.kube/config` pointing at a reachable cluster
- A terminal with 256-color support

## Running

```bash
./target/release/kubetile
```
```

### Step 1.2 — Add keybindings reference page

Create **`book/src/keybindings.md`**:
```markdown
# Keybindings

All keybindings are configurable. These are the defaults.

## Navigation

| Key | Action |
|-----|--------|
| `h j k l` / arrows | Move focus between panes |
| `Tab` | Cycle to next pane |
| `1`–`9` | Jump to tab by number |
| `gt` / `gT` | Next / previous tab |

## Pane management

| Key | Action |
|-----|--------|
| `\|` | Split pane vertically |
| `-` | Split pane horizontally |
| `x` | Close focused pane |
| `z` | Toggle fullscreen on focused pane |

## Resource list

| Key | Action |
|-----|--------|
| `/` | Filter |
| `s` | Sort by column |
| `A` | Toggle all-namespaces |
| `Enter` | Open detail / YAML |
| `l` | Open logs |
| `e` | Exec into pod |

## Application

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `q` | Quit |
| `:` | Command palette |
```

### Step 1.3 — Update SUMMARY.md

Replace `book/src/SUMMARY.md` with:
```markdown
# Summary

- [Introduction](introduction.md)
- [Installation](installation.md)
- [Keybindings](keybindings.md)
```

### Step 1.4 — Commit

```bash
git add book/src/
git commit -m "docs: add installation and keybindings pages"
git push origin main
```

---

## Phase 2 — Configuration & Features

**Goal:** Cover the config file, available views, and how the UI is structured.

### Step 2.1 — Add configuration page

Create **`book/src/configuration.md`**:
```markdown
# Configuration

KubeTile reads `~/.config/kubetile/config.toml` on startup.

## Example config

```toml
[general]
refresh_interval_ms = 2000
default_namespace = "default"

[keybindings]
# Override individual keys here (see Keybindings reference)
```

## Hot reload

Changes to the config file take effect after restarting KubeTile.
Hot reload is planned for a future release.
```

### Step 2.2 — Add views overview page

Create **`book/src/views.md`**:
```markdown
# Views

## Resource list

The default view when opening a pane. Shows a live-updating table of
Kubernetes resources filtered to the selected namespace.

Supported resource kinds: Pods, Deployments, Services, StatefulSets,
DaemonSets, Jobs, CronJobs, ConfigMaps, Secrets, Ingresses, Nodes,
Namespaces, PVs, PVCs.

## YAML view

Press `Enter` on any resource to open its full YAML in a dedicated pane.

## Describe view

Available from the command palette. Shows `kubectl describe` output.

## Logs view

Press `l` on a Pod row to stream its logs in a new pane.

## Exec view

Press `e` on a Pod row to open an interactive shell inside the container.

## Terminal

A general-purpose terminal pane. Open from the command palette.
```

### Step 2.3 — Update SUMMARY.md

```markdown
# Summary

- [Introduction](introduction.md)
- [Installation](installation.md)
- [Keybindings](keybindings.md)
- [Configuration](configuration.md)
- [Views](views.md)
```

### Step 2.4 — Commit

```bash
git add book/src/
git commit -m "docs: add configuration and views pages"
git push origin main
```

---

## Phase 3 — Polish: Theme, Search, and Navigation

**Goal:** The site looks intentional and is easy to explore.

### Step 3.1 — Add custom CSS (optional, low-risk)

Create **`book/theme/custom.css`**:
```css
:root {
  --sidebar-width: 260px;
}
/* Tighten up table rendering for keybinding tables */
table td:first-child {
  font-family: monospace;
  white-space: nowrap;
}
```

Update `book/book.toml` to reference it:
```toml
[output.html]
additional-css = ["theme/custom.css"]
```

### Step 3.2 — Enable full-text search

mdBook ships search by default. Verify it is not disabled in `book.toml`:
```toml
[output.html.search]
enable = true
```

### Step 3.3 — Add a 404 page (GitHub Pages)

Create **`book/src/404.md`**:
```markdown
# Page not found

The page you are looking for does not exist.

[Return to the introduction](introduction.md)
```

Add to `SUMMARY.md` at the bottom (not in the TOC, just as a standalone file):
```markdown
[404](404.md)
```

Then in `book/book.toml` set:
```toml
[output.html]
input-404 = "404.md"
```

### Step 3.4 — Commit

```bash
git add book/
git commit -m "docs: theme, search, and 404 page"
git push origin main
```

---

## Phase 4 — Future Pages (open-ended backlog)

Add each as its own commit following the same pattern: new `.md` file + one line in `SUMMARY.md`.

| Page | File | Notes |
|------|------|-------|
| Plugin system | `plugins.md` | When plugin API is designed |
| Architecture | `architecture.md` | Crate structure, module map |
| Contributing | `contributing.md` | PR guide, dev setup |
| Changelog | `changelog.md` | Link to GitHub Releases |
| Roadmap | `roadmap.md` | Pull from PRD.md |

---

## Maintenance Rules

| Task | Action |
|------|--------|
| Add a page | Create `book/src/<page>.md`, add one line to `SUMMARY.md` |
| Update content | Edit Markdown, push to `main` — CI does the rest |
| Pin mdBook version | Already pinned in `pages.yml` (`mdbook-version: '0.4.40'`) — update deliberately |
| Content requests | GitHub Issues with label `docs` |
| Never break the live site | Each commit to `main` that touches `book/` must leave `SUMMARY.md` valid |

---

## Checklist Summary

```
Phase 0 — Bootstrap
  [ ] book/book.toml created (replace YOUR_USERNAME)
  [ ] book/src/SUMMARY.md created
  [ ] book/src/introduction.md created
  [ ] .github/workflows/pages.yml created
  [ ] GitHub Settings → Pages source set to gh-pages branch
  [ ] Push to main, workflow runs green, URL is live

Phase 1 — Core content
  [ ] book/src/installation.md created
  [ ] book/src/keybindings.md created
  [ ] SUMMARY.md updated with both pages
  [ ] Push to main, site updated

Phase 2 — Configuration & views
  [ ] book/src/configuration.md created
  [ ] book/src/views.md created
  [ ] SUMMARY.md updated
  [ ] Push to main, site updated

Phase 3 — Polish
  [ ] book/theme/custom.css created
  [ ] book.toml updated for custom CSS
  [ ] Search confirmed enabled
  [ ] 404 page created
  [ ] Push to main, site updated
```
