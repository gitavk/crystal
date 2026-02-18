# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Crystal is a keyboard-driven, plugin-based Kubernetes TUI IDE written in Rust. It aims to combine observability and interaction features inspired by Lens and k9s, with a terminal UI inspired by zellij.

## Tech Stack

- **Language:** Rust
- **TUI:** ratatui/crossterm (to be finalized)
- **Kubernetes client:** kube-rs
- **Plugin system:** TBD

## Build Commands

Once Cargo is set up:
```bash
cargo build              # Build the project
cargo build --release    # Release build
cargo run                # Run the application
cargo test               # Run all tests
cargo test <test_name>   # Run a single test
cargo clippy             # Lint the code
cargo fmt                # Format the code
```

## Design Principles

- 100% keyboard-first workflow - every feature must have a keyboard shortcut
- Config over magic - prefer explicit configuration
- Small context windows - keep modules focused and self-contained
- Step-by-step development - incremental, well-tested changes

## Workflow

- Keep working from current folder without unnecessary directory changes
- Avoid adding obvious or redundant comments to code
- For each task:
  1. Create a plan using TodoWrite
  2. Present it for confirmation
  3. Execute step by step, marking progress
