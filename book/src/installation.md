# Installation

KubeTile can be installed by downloading a pre-built binary or by building from source.

## Binary Releases

Pre-built binaries for Linux and macOS are available on the [GitHub Releases](https://github.com/gitavk/KubeTile/releases) page. We provide:
- `.tar.gz` archives for Apple aarch64 as well for linux x86_64 and aarch64
- `.deb` packages for Debian/Ubuntu
- `.rpm` packages for Fedora/RHEL

## From source (requires Rust ≥ 1.75)

```bash
git clone https://github.com/gitavk/KubeTile
cd KubeTile
cargo build --release
# Binary will be available at target/release/kubetile
```

To install the binary to your `PATH`, you can run:
```bash
cargo install --path crates/kubetile-app
```

## Requirements

- **`kubectl`**: KubeTile uses `kubectl` for exec sessions and some describe operations. It must be installed and available in your `PATH`.
- **Kubeconfig**: A valid `~/.kube/config` pointing at a reachable cluster.
- **Terminal**: A terminal with 256-color support and a modern font (e.g., Nerd Fonts for better icon support if applicable).

## Running

```bash
./target/release/kubetile
```
