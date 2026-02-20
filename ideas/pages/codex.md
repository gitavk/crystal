# Hosting & Supporting the App Website via GitHub Pages

## Option 1 – Static Site with GitHub Pages + GitHub Actions build
1. Keep the website as a static site generator (Hugo, Next.js export, Astro, etc.) inside an `website/` folder in this repo.
2. Add a GitHub Actions workflow (`.github/workflows/pages.yml`) able to:
   - install the chosen toolchain, build the site into a `out/` or `dist/` directory,
   - push the build artifacts to the `gh-pages` branch (or to the repository root via the `pages` deployment). Use the official `actions/configure-pages` and `actions/upload-pages-artifact` steps for reliability.
3. Configure GitHub Pages (Settings → Pages) to serve from `gh-pages` (or `/` on the main branch if you prefer). Tie a custom domain and HTTPS certificate there.
4. Support & updates: treat the site like any other feature—change markdown, templates, or components in `website/`, push to the default branch, and let Actions rebuild. Pin dependency versions and cache npm/yarn handles to keep builds fast. Add automated checks (lint/format/test) that run before the deployment job so only green commits publish.
5. For ongoing maintenance, document the workflow in this file and, if needed, create a lightweight dashboard (issue templates, Zaps) so stakeholders can request content changes and track deployments.

## Option 2 – Content-First with GitHub Pages + Docs-Only Branch
1. Create a `docs/` folder at the repo root whose contents mirror the published site; GitHub Pages can serve `/docs` directly from the default branch, so you avoid extra branches.
2. Write or generate your marketing content (pages, assets) directly into `docs/` using Markdown, including a lightweight `README.md` as the homepage, and keep reusable assets in `docs/static/`.
3. Use a simple build step (optional) that compiles assets into `docs/` via npm scripts or a minimal toolchain run locally, then commit the generated files; no Actions workflow is required if you build before pushing.
4. To support the site, keep `docs/README.md` updated with contribution notes and use GitHub Issues for enhancement requests; pair this with a short “deploy checklist” (test locally, run lint, push) so contributors know how to preview changes via browser previews or `npm run dev` before committing.
