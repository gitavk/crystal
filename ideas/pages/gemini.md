# Hosting & Supporting the KubeTile Website via GitHub Pages

Here are the two most effective ways to manage and support the website for KubeTile on GitHub Pages.

## Option 1: Static Site Generator (Zola/Hugo) + GitHub Actions (Recommended)
This approach is ideal for a Rust project, offering high performance and easy maintenance.
1. **Setup:** Use a Rust-based Static Site Generator like **Zola** or **Hugo** in a `website/` directory.
2. **Automation:** Use a GitHub Action (`.github/workflows/deploy.yml`) to automatically build and deploy the site to the `gh-pages` branch on every push to `main`.
3. **Support:** Use Markdown for documentation and blog posts. This makes it easy for contributors to submit PRs for content updates.
4. **Maintenance:** The build process is fully automated, ensuring the live site always reflects the latest documentation in the repository.

## Option 2: Simple Static HTML in `/docs` Folder (Low Friction)
This is the simplest way to get started without needing a build pipeline.
1. **Setup:** Move your existing HTML/CSS files from the `pages/` directory into a `/docs` directory at the root of the repository.
2. **Configuration:** In GitHub Repository Settings, set the GitHub Pages source to the `/docs` folder on the `main` branch.
3. **Support:** Directly edit the HTML/CSS files in the `/docs` folder. No compilation or extra tools are required.
4. **Maintenance:** Updates are live as soon as changes are merged into the `main` branch. This is perfect for a lightweight landing page and basic documentation.
