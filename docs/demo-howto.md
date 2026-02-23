# demo.gif — Content & Recording Guide

## What to show

The sequence should tell a short story: *connect → navigate → inspect → debug*.
Target length: **45–60 seconds**. Shorter is better; stop after the strongest moment.

### Recommended script

| Step | Action | Key(s) | Why it matters |
|------|--------|--------|----------------|
| 1 | Launch `kubetile` — cluster connects, pod list loads live | — | Establishes context immediately |
| 2 | Scroll through the pod list | `j` / `k` | Shows keyboard-first feel |
| 3 | Open resource switcher, type `dep`, confirm to switch to Deployments | `:` → type → `Enter` | Highlights the command palette |
| 4 | Split the pane vertically, switch resource to Pods in the new pane | `Alt+v` → `:` | Shows the tiled layout — the visual differentiator |
| 5 | Select a pod in the Pods pane, open its logs | `Enter` or `l` | Practical daily use |
| 6 | Close the log pane, select a pod, trigger debug mode | `Alt+x` → `Ctrl+Alt+d` → confirm `y` | The strongest unique feature — save this for last |
| 7 | Exec into the now-sleeping pod | `e` | Pays off the debug mode setup |

### Tips
- Use a cluster with a few recognisable app names (not just `nginx-7d9f...`) — readable names read better on screen.
- Keep the terminal font large enough to be legible at 800px wide.
- Pause 1–2 seconds after each meaningful state change so the viewer can register it.
- End on the exec shell prompt inside the debug container — strong visual finish.

---

## Recording tools

### asciinema + asciinema.org (recommended online path)

1. Install the recorder:
   ```bash
   # macOS
   brew install asciinema
   # Debian/Ubuntu
   sudo apt install asciinema
   ```
2. Record:
   ```bash
   asciinema rec demo.cast
   # run through the script above, then Ctrl+D to stop
   ```
3. Upload to **asciinema.org** (free hosting, no account required for anonymous):
   ```bash
   asciinema upload demo.cast
   # returns a URL you can link from README or LinkedIn
   ```
4. Convert to GIF for embedding in GitHub README using **agg** (asciinema GIF generator):
   ```bash
   cargo install --git https://github.com/asciinema/agg
   agg demo.cast docs/demo.gif
   ```
   Or use the hosted converter at **https://asciinema.org** — after uploading, the share page links to rendering options.

> **asciinema.org** is the strongest online option: the hosted player is interactive (viewer can pause/copy text), shareable by URL, and embeds cleanly.

## Embedding in README

Once you have `docs/demo.gif`, uncomment the line in `README.md`:

```markdown
![KubeTile demo](docs/demo.gif)
```

For the asciinema hosted player instead of a GIF, use:

```markdown
[![asciicast](https://asciinema.org/a/YOUR_ID.svg)](https://asciinema.org/a/YOUR_ID)
```
