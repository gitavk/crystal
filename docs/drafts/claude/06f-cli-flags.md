# Step 6.6 — CLI Flags for Config Management

> `feat(app): add --init-config and --print-config CLI flags`

## Goal

Add two CLI flags that let users manage their configuration without launching
the full TUI: one to generate a default config file, one to dump the current
effective config.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/main.rs` | UPDATE — add clap flags, handle before TUI startup |

## CLI Flags

### `crystal --init-config`

Generates `~/.config/crystal/config.toml` with all default values and
inline comments explaining each section. Exits after generation.

```
$ crystal --init-config
Config written to /home/user/.config/crystal/config.toml
```

Behavior:
- Creates parent directories if needed (`~/.config/crystal/`)
- If file already exists, prints a warning and does NOT overwrite
  (user must delete manually to regenerate)
- The generated file includes all sections with comments matching the
  defaults.toml structure
- Exit code 0 on success, 1 on error

### `crystal --print-config`

Dumps the current effective configuration (defaults merged with user
overrides) as TOML to stdout. Exits after printing.

```
$ crystal --print-config
[general]
tick-rate-ms = 250
default-namespace = "default"
...
```

Behavior:
- Loads user config if it exists, merges with defaults
- Outputs the merged result as valid TOML
- Useful for debugging ("what is crystal actually using?")
- Exit code 0 on success, 1 on error

## Implementation

```rust
// crates/crystal-app/src/main.rs

#[derive(Parser)]
struct Cli {
    /// Generate default config file at ~/.config/crystal/config.toml
    #[arg(long)]
    init_config: bool,

    /// Print effective config (defaults + user overrides) and exit
    #[arg(long)]
    print_config: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.init_config {
        let path = AppConfig::init_default()?;
        println!("Config written to {}", path.display());
        return Ok(());
    }

    if cli.print_config {
        let config = AppConfig::load(&AppConfig::default_path())?;
        println!("{}", toml::to_string_pretty(&config)?);
        return Ok(());
    }

    // ... normal TUI startup
}
```

## Notes

- Both flags are handled before any TUI initialization (no terminal setup,
  no K8s connection).
- `--init-config` writes a human-friendly file with comments. Since `toml`
  crate's serializer doesn't preserve comments, we use the embedded
  `defaults.toml` directly (which already has comments) rather than
  serializing the struct.
- `--print-config` uses `toml::to_string_pretty` on the merged struct,
  so it reflects actual effective values (no comments, but accurate).

## Tests

- `--init-config` creates file at expected XDG path
- `--init-config` with existing file does NOT overwrite
- `--print-config` output is valid TOML that round-trips through `AppConfig`
- Both flags exit without starting the TUI
