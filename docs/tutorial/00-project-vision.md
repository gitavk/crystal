# Chapter 0: Project Vision and Setup

## What You Will Learn

- How to start a Rust project from an idea before writing any code
- How to structure a repository for AI-assisted development
- How to use AI coding tools to turn rough ideas into a structured plan
- How to set up Claude Code skills for Rust development

## Prerequisites

- Git installed
- A GitHub account
- An AI coding assistant (Claude Code, Codex, or Gemini)
- Basic familiarity with the terminal

No Rust installation is needed yet — this chapter is entirely about planning.

---

## Step 1: Create the Repository

Start with a new GitHub repository. Initialize it with a LICENSE file.

```bash
# Create the repo on GitHub (MIT license), then clone it
git clone git@github.com:<your-username>/crystal.git
cd crystal
```

After cloning you should have:

```
crystal/
└── LICENSE
```

> **Checkpoint:** `git log --oneline` shows a single "Initial commit" with just the LICENSE file.

---

## Step 2: Write Down the Raw Idea

Before involving any AI tool or writing any code, capture the project vision in plain text. This becomes the input prompt for AI-assisted planning later.

Create the file `ideas/00-main.txt`:

```bash
mkdir -p ideas
```

Write your project vision covering:
- What you want to build (a Rust TUI for Kubernetes management)
- Why (learn Rust, share the experience as a tutorial series)
- UI inspiration (zellij-style pane layout)
- Core features: keyboard-first, plugin support, context-aware terminal
- Reference projects (Lens, k9s) for feature inspiration
- Must-have vs lower-priority features
- A note that the plan should be AI-readable with small context windows

Here is the content used in this project:

```text
I want to build the application
call it crystal.
and learn new language RUST
with best practice of the using LLM,
and share my experience by create a series of the videos for youtube.
Build step by step plan (one answer one step), what is best way to release this idea.
here  the application should be:
- on rust language
- allow to keep hands on keyboard for any operations
- use interface style like zellij https://zellij.dev/documentation/
- allow to use plugins
- UX should have:
  - Keyboard shortcuts (hide in config)
  - Help screens
- main functions should allow to implement same as:
  - https://lenshq.io/products/lens-k8s-ide
  - https://k9scli.io/
- must have features:
  - "Cluster-Aware" Internal Terminal ( it should pre-configure the environment (KUBECONFIG/Context) )
  -  "exec" and "logs" features are part of a broader Context-Aware Terminal
- lower level priority feauters:
  - benchmark HTTP services and pods directly within the UI to help adjust resource requests and limits
  - logic to automatically detect and connect to cloud-provider clusters (EKS, GKE, AKS)
  - AI-Powered Assistance
  - "XRay" and "Pulse" views to allow users to visualize dependencies and get a high-level "state of affairs" for the cluster
the plan should be well readable for AI tools like claudecode/codex/gemini
to keep small context window during development
so split the document on stages.
```

Commit:

```bash
git add ideas/00-main.txt
git commit -m "ch00: Main idea."
```

> **Why plain text?** It is unstructured on purpose. The goal is to capture intent before imposing structure. AI tools work well when given raw requirements to organize.

---

## Step 3: Configure AI-Assisted Development

If you are using Claude Code, set up project-level instructions and skills. This ensures consistent coding standards across all AI-assisted sessions.

### 3a. Create CLAUDE.md

This file tells Claude Code about the project context, tech stack, and design principles. Place it at the repository root:

```markdown
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Crystal is a keyboard-driven, plugin-based Kubernetes TUI IDE written in Rust.
It aims to combine observability and interaction features inspired by Lens and k9s,
with a terminal UI inspired by zellij.

## Tech Stack

- **Language:** Rust
- **TUI:** ratatui/crossterm (to be finalized)
- **Kubernetes client:** kube-rs
- **Plugin system:** TBD

## Build Commands

Once Cargo is set up:
cargo build              # Build the project
cargo build --release    # Release build
cargo run                # Run the application
cargo test               # Run all tests
cargo test <test_name>   # Run a single test
cargo clippy             # Lint the code
cargo fmt                # Format the code

## Design Principles

- 100% keyboard-first workflow - every feature must have a keyboard shortcut
- Config over magic - prefer explicit configuration
- Small context windows - keep modules focused and self-contained
- Step-by-step development - incremental, well-tested changes
```

### 3b. Create Claude Code Skills

Skills are reusable instruction sets that activate when relevant. Create two skills for Rust development:

```bash
mkdir -p .claude/skills/rust-developer
mkdir -p .claude/skills/rust-tester
```

**`.claude/skills/rust-developer/SKILL.md`** — Covers Rust coding standards: style, ownership, error handling, traits, concurrency, common patterns (Builder, Newtype, Typestate). This ensures AI-generated code follows idiomatic Rust.

**`.claude/skills/rust-tester/SKILL.md`** — Covers Rust testing practices: unit tests with AAA pattern, `test_case` for parameterized tests, `mockall` for mocking, integration test structure. This ensures AI-generated tests are thorough and well-structured.

> The full content of these skills is available in the repository under `.claude/skills/`. They are reference documents — read through them to understand the standards, but you don't need to memorize them. The AI tool loads them automatically when relevant.

Commit:

```bash
git add CLAUDE.md .claude/
git commit -m "ch00: Claude config."
```

---

## Step 4: Generate AI Drafts

Feed your `ideas/00-main.txt` to one or more AI tools and ask them to produce a structured project plan. The goal is to get different perspectives and then consolidate the best ideas.

In this project, two AI tools were used:

### ChatGPT Draft

Prompt ChatGPT with the raw idea and ask for a project README with architecture, tech stack, and staged development plan. Save the output:

```bash
mkdir -p docs/drafts/chatgpt
# Save ChatGPT's output as docs/drafts/chatgpt/README.md
```

### Claude Draft

Do the same with Claude. The output for this project included a detailed architecture diagram, tech stack table, 13-stage development roadmap, naming conventions, and git strategy:

```bash
mkdir -p docs/drafts/claude
# Save Claude's output as docs/drafts/claude/00-README.md
```

Commit the drafts:

```bash
git add docs/drafts/
git commit -m "ch00: AI ch00 ideas."
```

> **Why keep drafts?** They document the decision-making process. Readers can compare what each AI suggested and see how the final plan was synthesized.

---

## Step 5: Create the PRD

Use the AI drafts and the original idea to create a single Product Requirements Document. The PRD consolidates and deduplicates all sources into one authoritative reference.

Feed the drafts to your AI tool with a prompt like:

```
Review ideas/00-main.txt, use docs/drafts/claude/00-README.md as base,
enrich with information from docs/drafts/chatgpt/README.md, create PRD.md
```

The resulting `PRD.md` should contain:

1. **Executive Summary** — what Crystal is in one paragraph
2. **Problem Statement** — why existing tools are insufficient
3. **Target Users** — who this is for (SREs, DevOps, Rust learners)
4. **Goals and Non-Goals** — sharp boundaries on scope
5. **Feature Requirements (MoSCoW)** — prioritized feature list (Must/Should/Could/Won't)
6. **Architecture Overview** — three-layer diagram (TUI, Core Engine, Data Layer)
7. **Tech Stack** — finalized technology choices with rationale
8. **Development Roadmap** — 13 stages with priority and milestones
9. **Naming Conventions** — crate names, module files, structs, functions
10. **Success Metrics** — functional, code quality, and UX targets
11. **Risks and Mitigations** — what could go wrong and how to handle it
12. **Open Questions** — unresolved decisions for later stages

Commit:

```bash
git add PRD.md
git commit -m "ch00: PRD"
```

---

## Step 6: Tag the Chapter

Once all planning documents are in place, tag the repository to mark the end of Chapter 0:

```bash
git tag ch00
```

---

## Repository State After Chapter 0

```
crystal/
├── .claude/
│   └── skills/
│       ├── rust-developer/
│       │   └── SKILL.md          # Rust coding standards for AI
│       └── rust-tester/
│           └── SKILL.md          # Rust testing standards for AI
├── docs/
│   └── drafts/
│       ├── chatgpt/
│       │   └── README.md         # ChatGPT's project plan draft
│       └── claude/
│           └── 00-README.md      # Claude's project plan draft
├── ideas/
│   └── 00-main.txt              # Raw project vision
├── CLAUDE.md                     # AI assistant project config
├── LICENSE                       # MIT License
├── PRD.md                        # Product Requirements Document
└── README.md                     # Project summary
```

### Git History

```
69559f2 (tag: ch00) ch00: PRD
901bc5d ch00: AI ch00 ideas.
76388e2 ch00: Claude config.
a2d8d79 ch00: Main idea.
95893bc Initial commit
```

---

## Key Takeaways

1. **Start with the idea, not the code.** Writing down the raw vision before structuring it ensures nothing gets lost.
2. **Use multiple AI tools for drafting.** Different models produce different perspectives. Comparing outputs leads to better decisions.
3. **Consolidate into a PRD.** A single source of truth prevents conflicting documentation from drifting apart over time.
4. **Configure AI tools early.** Setting up CLAUDE.md and skills before writing code means every future AI session starts with the right context.
5. **Keep drafts in the repo.** They document the reasoning process and help readers understand why decisions were made.

---

## What's Next

In **Chapter 1** we will set up the Rust toolchain, create a Cargo workspace, configure CI, and render the first TUI frame with ratatui.
