# KubeForge — Rust TUI Kubernetes IDE

## Project Vision

A terminal-based Kubernetes IDE built in Rust, inspired by k9s and Lens, with a
zellij-style pane/tab interface, full keyboard-driven UX, plugin system, and
context-aware terminals. Designed for power users who want cluster management
without leaving the terminal.

## YouTube Series Context

This project is built step-by-step as a "Learn Rust by Building" YouTube series.
Each stage maps to 2-4 video episodes. The codebase is structured so each stage
produces a working, demo-able increment.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    KubeForge TUI                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │  Tab Bar  │ │  Panes   │ │  Status  │ │  Command  │  │
│  │          │ │ (zellij)  │ │   Bar    │ │  Palette  │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │
├─────────────────────────────────────────────────────────┤
│                  Core Engine                             │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐   │
│  │  K8s    │ │  Event   │ │  Plugin  │ │  Config   │   │
│  │  Client │ │  Bus     │ │  Host    │ │  Manager  │   │
│  └─────────┘ └──────────┘ └──────────┘ └───────────┘   │
├─────────────────────────────────────────────────────────┤
│                  Data Layer                              │
│  ┌─────────────┐  ┌───────────┐  ┌──────────────────┐  │
│  │ kube-rs     │  │ Informer  │  │  Context/Config  │  │
│  │ (API calls) │  │ Cache     │  │  Store           │  │
│  └─────────────┘  └───────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Tech Stack

| Component        | Choice               | Why                                    |
|------------------|----------------------|----------------------------------------|
| Language         | Rust                 | Performance, safety, learning goal     |
| TUI Framework    | ratatui              | Active ecosystem, crossterm backend    |
| K8s Client       | kube-rs              | Native async Rust, well-maintained     |
| Async Runtime    | tokio                | De facto standard for async Rust       |
| Serialization    | serde + serde_yaml   | K8s manifests are YAML                 |
| Plugin System    | wasmtime (WASM)      | Sandboxed, polyglot plugin support     |
| Terminal Mux     | Custom (ratatui)     | Zellij-style panes within the app      |
| Config           | toml                 | Rust ecosystem standard                |
| Logging          | tracing              | Structured, async-aware                |

## Stage Index

| Stage | File                          | Focus                              | Videos |
|-------|-------------------------------|------------------------------------|--------|
| 1     | `01-project-scaffold.md`      | Cargo workspace, CI, TUI skeleton  | 2      |
| 2     | `02-k8s-core.md`              | kube-rs integration, resource list | 2-3    |
| 3     | `03-tui-layout.md`            | Zellij-style panes, navigation     | 2-3    |
| 4     | `04-resource-views.md`        | Pods, Deployments, Services views  | 3-4    |
| 5     | `05-context-terminal.md`      | Cluster-aware terminal, exec, logs | 2-3    |
| 6     | `06-config-keybindings.md`    | TOML config, custom keybindings    | 2      |
| 7     | `07-plugin-system.md`         | WASM plugin host, plugin API       | 3-4    |
| 8     | `08-advanced-resources.md`    | CRDs, RBAC, Events, Helm releases  | 2-3    |
| 9     | `09-cloud-discovery.md`       | EKS/GKE/AKS auto-detect           | 2      |
| 10    | `10-benchmarking.md`          | HTTP benchmarking, resource tuning | 2      |
| 11    | `11-xray-pulse.md`            | Dependency graph, cluster health   | 2-3    |
| 12    | `12-ai-assist.md`             | LLM integration, natural language  | 2-3    |
| 13    | `13-polish-release.md`        | Packaging, docs, v1.0              | 2      |

## How to Use This Plan with AI Tools

Each stage file is self-contained. Feed ONLY the relevant stage file to your AI
coding assistant to keep context small:

```bash
# Example: working on Stage 2
cat 02-k8s-core.md | claude-code
# or paste into Codex/Gemini context
```

Each stage file contains:
- **Goal**: What this stage achieves
- **Prerequisites**: What must be done before
- **File Tree**: Exact files to create/modify
- **Tasks**: Numbered checklist of implementation steps
- **Interfaces**: Key structs/traits with signatures
- **Tests**: What to test and how
- **Demo**: What to show in the YouTube video
- **Commit convention**: How to structure git history

## Naming Conventions

| Item            | Convention               | Example                      |
|-----------------|--------------------------|------------------------------|
| Crate names     | `kubeforge-*`            | `kubeforge-core`             |
| Module files    | `snake_case.rs`          | `resource_view.rs`           |
| Structs/Enums   | `PascalCase`             | `PodListView`                |
| Functions       | `snake_case`             | `fetch_pods`                 |
| Constants       | `SCREAMING_SNAKE`        | `DEFAULT_NAMESPACE`          |
| Config keys     | `kebab-case` in TOML     | `refresh-interval`           |
| Plugin names    | `kebab-case`             | `my-custom-plugin`           |

## Git Strategy

- Branch per stage: `stage/01-scaffold`, `stage/02-k8s-core`, ...
- Squash merge to `main` at stage completion
- Tag releases: `v0.1.0` (stage 4), `v0.2.0` (stage 7), `v1.0.0` (stage 13)
