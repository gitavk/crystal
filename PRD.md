# KubeTile — Product Requirements Document

| Field       | Value                                      |
|-------------|--------------------------------------------|
| Version     | 0.1.0                                      |
| Status      | Draft                                      |
| Updated     | 2026-02-19                                 |
| Name Note   | **KubeTile** is the canonical project name |

---

## 1. Executive Summary

Crystal is a terminal-based Kubernetes IDE built in Rust, inspired by the observability of Lens, the speed of k9s, and the pane/tab UX of zellij. It provides a 100% keyboard-driven workflow for managing Kubernetes clusters — resource views, context-aware terminals, pod exec, log streaming, and a WASM-based plugin system — all without leaving the terminal. Crystal serves a dual purpose: a real tool for platform engineers and SREs, and a "Learn Rust by Building" tutorial series where each development stage produces a working, self-contained increment.

---

## 2. Problem Statement

- **k9s** is powerful but not extensible and lacks multi-pane management
- **Lens** provides great observability but is a heavy Electron GUI — not terminal-native
- No existing tool combines zellij-style pane layout + K8s resource views + context-aware terminal + plugin system in a single TUI
- Engineers who work in the terminal must context-switch between multiple tools: k9s for resources, a shell for kubectl, a browser for Lens dashboards
- There is no high-quality "build a Rust TUI" tutorial that produces a real-world tool

---

## 3. Target Users

| Persona | Description | Primary Need |
|---------|-------------|--------------|
| **Platform Engineer / SRE** | Manages multiple K8s clusters daily. Lives in the terminal. | Fast context switching, log tailing, exec into pods, resource monitoring |
| **DevOps Engineer** | Deploys and debugs workloads. | Quick resource views, event monitoring, pod health inspection |
| **Rust Learner** | Following the tutorial series. | Clear, incremental code demonstrating real-world Rust patterns |

---

## 4. Goals and Non-Goals

### Goals

1. 100% keyboard-first workflow — every feature accessible via keyboard shortcut
2. Zellij-inspired TUI with tabs, panes, and a status bar
3. Kubernetes observability: list/describe/watch pods, deployments, services, events, CRDs
4. Context-aware internal terminal: auto-configures KUBECONFIG and namespace per pane
5. Exec into pods and tail logs directly within the TUI
6. WASM-based plugin system for extending resource views and adding custom actions
7. TOML-based user configuration with customizable keybindings
8. AI-tool-friendly codebase: small, self-contained modules; stage-based planning documents
9. Each development stage produces a shippable, demo-able artifact

### Non-Goals

1. No GUI / graphical frontend — terminal only
2. No built-in YAML editor — users use their own editor
3. No Helm chart management (inspecting Helm releases is in scope; install/upgrade is not)
4. No cluster provisioning (no terraform, eksctl, etc.)
5. No multi-user / server mode — single-user local tool
6. No Windows support in initial releases (Linux and macOS first)

---

## 5. Feature Requirements (MoSCoW)

### Must Have (P0)

| Feature | Description | Stage |
|---------|-------------|-------|
| TUI skeleton | ratatui + crossterm app loop with event handling | 1 |
| K8s client integration | Connect to clusters via kube-rs, list namespaces and pods | 2 |
| Zellij-style pane layout | Tabs, split panes, pane navigation via keyboard | 3 |
| Resource views | Pods, Deployments, Services — list, detail, describe | 4 |
| Context-aware terminal | Internal terminal with pre-set KUBECONFIG/context/namespace | 5 |
| Pod exec & log streaming | Shell into running pods; tail/follow logs | 5 |
| Keybinding configuration | TOML config file for all keyboard shortcuts | 6 |
| Help screens | Discoverable shortcut overlays | 6 |

### Should Have (P1)

| Feature | Description | Stage |
|---------|-------------|-------|
| WASM plugin system | wasmtime-based host; plugin API for custom views and actions | 7 |
| Advanced resources | CRDs, RBAC views, Events, Helm release inspection | 8 |
| Command palette | Fuzzy-search command launcher | 3–6 |
| Status bar | Cluster connection state, current context, namespace | 1–3 |

### Could Have (P2)

| Feature | Description | Stage |
|---------|-------------|-------|
| Cloud provider auto-detect | EKS/GKE/AKS cluster discovery | 9 |
| HTTP benchmarking | Benchmark services/pods to tune resource requests/limits | 10 |
| XRay view | Dependency graph visualization for cluster resources | 11 |
| Pulse view | High-level cluster health dashboard | 11 |

### Won't Have (v1.0)

| Feature | Reason |
|---------|--------|
| AI-powered assistance | Deferred to post-v1.0; should be a plugin, not core |
| GUI frontend | Conflicts with core identity as a TUI tool |
| Built-in YAML editor | Out of scope |
| Helm install/upgrade | Only inspection is in scope |
| Cluster provisioning | Out of scope |

---

## 6. Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Crystal TUI                           │
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

### Key Architectural Decisions

- **Event bus pattern** for decoupling TUI from data layer
- **Informer cache** via kube-rs watcher to avoid repeated API calls
- **WASM runtime** (wasmtime) for sandboxed, polyglot plugin support
- **Cargo workspace** with multiple crates (`kubetile-core`, `crystal-tui`, `crystal-plugins`, etc.)

---

## 7. Tech Stack

| Component | Decision | Rationale |
|-----------|----------|-----------|
| Language | Rust | Performance, safety, learning goal |
| TUI framework | ratatui + crossterm | Active ecosystem, crossterm backend |
| K8s client | kube-rs | Native async Rust, well-maintained |
| Async runtime | tokio | De facto standard; required by kube-rs |
| Serialization | serde + serde_yaml | K8s manifests are YAML |
| Plugin runtime | wasmtime (WASM) | Sandboxed, polyglot |
| Config format | TOML | Rust ecosystem standard |
| Logging | tracing | Structured, async-aware |

---

## 8. Development Roadmap

### Stages

| Stage | Focus | Priority | Chapters | Milestone |
|-------|-------|----------|----------|-----------|
| 1 | Project scaffold: Cargo workspace, CI, TUI skeleton | P0 | 2 | First render |
| 2 | K8s core: kube-rs integration, resource listing | P0 | 2–3 | First cluster connection |
| 3 | TUI layout: zellij-style panes, keyboard navigation | P0 | 2–3 | Multi-pane navigation |
| 4 | Resource views: Pods, Deployments, Services | P0 | 3–4 | **v0.1.0** |
| 5 | Context-aware terminal: exec, logs | P0 | 2–3 | Terminal integration |
| 6 | Config & keybindings: TOML config, help screens | P0 | 2 | User customization |
| 7 | Plugin system: WASM host, plugin API | P1 | 3–4 | **v0.2.0** |
| 8 | Advanced resources: CRDs, RBAC, Events, Helm | P1 | 2–3 | Full resource coverage |
| 9 | Cloud discovery: EKS/GKE/AKS auto-detect | P2 | 2 | Cloud integration |
| 10 | Benchmarking: HTTP bench, resource tuning | P2 | 2 | Performance tools |
| 11 | XRay & Pulse: dependency graph, cluster health | P2 | 2–3 | Visualization |
| 12 | AI assistance: LLM integration | P3 | 2–3 | Post-v1.0 |
| 13 | Polish & release: packaging, docs | P0 | 2 | **v1.0.0** |

**Estimated total: ~30–38 tutorial chapters across 13 stages.**

### Stage File Format

Each stage has a self-contained markdown file containing:
- **Goal** — what this stage achieves
- **Prerequisites** — what must be done before
- **File Tree** — exact files to create/modify
- **Tasks** — numbered implementation checklist
- **Interfaces** — key structs/traits with signatures
- **Tests** — what to test and how
- **Demo** — what to demonstrate at the end of the tutorial chapter
- **Commit convention** — how to structure git history

### Git Strategy

- Branch per stage: `stage/01-scaffold`, `stage/02-k8s-core`, ...
- Squash merge to `main` at stage completion
- Tagged releases: `v0.1.0` (stage 4), `v0.2.0` (stage 7), `v1.0.0` (stage 13)

---

## 9. Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Crate names | `kubetile-*` | `kubetile-core` |
| Module files | `snake_case.rs` | `resource_view.rs` |
| Structs/Enums | `PascalCase` | `PodListView` |
| Functions | `snake_case` | `fetch_pods` |
| Constants | `SCREAMING_SNAKE` | `DEFAULT_NAMESPACE` |
| Config keys | `kebab-case` in TOML | `refresh-interval` |
| Plugin names | `kebab-case` | `my-custom-plugin` |

---

## 10. Success Metrics

### Functional

- All P0 features working by Stage 6 completion
- Plugin system supporting at least 1 third-party plugin by Stage 7
- v1.0.0 released by Stage 13 with all P0 and P1 features

### Code Quality

- Zero clippy warnings across the workspace
- Test coverage: 70%+ for core engine, 50%+ for TUI layer
- CI passing on every merge to main

### User Experience

- Time to first useful action (list pods) under 3 seconds after launch
- Every feature accessible via keyboard without mouse
- Startup time under 500ms on a modern machine

### Tutorial / Community

- Each stage produces 2–4 publishable tutorial chapters
- Each stage has a working demo readers can reproduce
- Documentation sufficient for building from source and following along

---

## 11. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| ratatui limitations for complex pane management | Medium | High | Prototype zellij-style panes early in Stage 3; evaluate alternatives before Stage 4 |
| Embedded terminal complexity (PTY in TUI) | High | High | Stage 5 is the hardest; allocate extra time; evaluate `portable-pty` crate |
| Scope creep from P2/P3 features | High | High | Strict MoSCoW enforcement; P2 features only after v0.2.0 |
| kube-rs breaking changes | Low | Medium | Pin versions; use stable informer/watcher patterns |
| WASM plugin API design instability | Medium | Medium | Keep plugin API surface minimal in v0.2.0; iterate based on usage |
| Learning Rust while building production features | Medium | Medium | Stages 1–2 are simpler for learning curve; refactor aggressively |
| Tutorial writing overhead slowing development | Medium | Medium | Write tutorials in batches; keep chapter scope aligned with stage scope |

---

## 12. Open Questions

1. **Embedded terminal crate:** Which crate for PTY management — `portable-pty`, `vt100`, or custom?
2. **Plugin system approach:** WASM from the start (Stage 7), or a simpler Lua/script-based system first?
3. **Distribution strategy:** Homebrew? `cargo install`? AppImage? Decide before Stage 13.
4. **License type:** Confirm license choice (currently has a LICENSE file — verify type).
