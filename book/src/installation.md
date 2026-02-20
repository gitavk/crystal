# Installation

## From source (requires Rust ≥ 1.75)

```bash
git clone https://github.com/gitavk/KubeTile
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
