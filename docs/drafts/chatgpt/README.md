# Crystal: Rust Kubernetes Management TUI
A keyboard-driven, plugin-based Kubernetes TUI IDE written in Rust.
## Goals
- 100% keyboard-first workflow
- Terminal UI inspired by zellij
- Kubernetes observability + interaction (Lens + k9s inspired)
- Context-aware internal terminal
- Extensible via plugins
- Designed with AI-assisted development in mind
## Non-Goals (for now)
- No GUI
- No YAML editor
- No Helm management
- No cluster provisioning
## Target Users
- Platform engineers
- SREs
- DevOps engineers
- Rust learners
## Core Principles
- Small context windows
- Step-by-step development
- Every feature behind a keyboard shortcut
- Config over magic
## Tech Stack (initial)
- Language: Rust
- TUI: TBD (ratatui/crossterm evaluation later)
- K8s client: kube-rs
- Plugin system: TBD 
