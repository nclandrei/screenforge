# AGENTS.md

## Project

Screenforge is a Rust CLI for generating App Store-style App Store marketing screenshots.

## Setup

- Requires Rust/Cargo.
- `snap` uses `xcrun simctl`; Xcode command line tools and a booted iOS simulator are required for simulator capture.

## Core Commands

- Build: `cargo build`
- Release build: `cargo build --release`
- Test: `cargo test`
- Install locally: `cargo install --path .`

Run commands through Cargo in development:

- Full pipeline: `cargo run -- run --config ./screenforge.yaml`
- Quick simulator capture: `cargo run -- snap "iPhone 16 Pro"`
- List booted simulators: `cargo run -- snap --list`
- List device presets: `cargo run -- devices`
- Validate overlays: `cargo run -- verify-overlay --config ./screenforge.yaml --strict`
- Import overlays: `cargo run -- import-frames --source <dir>`
- Convert white-screen mockups: `cargo run -- convert-frames --source <dir>`

Equivalent installed CLI commands are available as `screenforge <command>`.

## CLI Help And Flags

Top-level help:

- `screenforge --help`
- `screenforge --version`
- Usage: `screenforge <COMMAND>`
- Commands: `run`, `devices`, `import-frames`, `verify-overlay`, `snap`, `convert-frames`, `help`
- Global flags:
  - `-h, --help`
  - `-V, --version`

Subcommand help:

- `screenforge run --help`
  - `-c, --config <CONFIG>` (default: `screenforge.yaml`)
- `screenforge devices --help` (no extra flags)
- `screenforge import-frames --help`
  - `-s, --source <SOURCE>` (required)
  - `--dest <DEST>` (default: `assets/frames`)
  - `--overwrite`
- `screenforge verify-overlay --help`
  - `-c, --config <CONFIG>` (default: `screenforge.yaml`)
  - `--strict`
- `screenforge convert-frames --help`
  - `-s, --source <SOURCE>` (required)
  - `--dest <DEST>` (default: `assets/frames`)
  - `--overwrite`
  - `--white-threshold <WHITE_THRESHOLD>` (default: `250`)
- `screenforge snap --help`
  - Positional: `[SIMULATOR]` (name, partial name, or UDID)
  - `-o, --output <OUTPUT>` (default: `snap_output.png`)
  - `--raw`
  - `-l, --list`
  - `--format <FORMAT>` (default: `text`; values: `text|json`)
  - `--model <MODEL>` (values: `iphone16-pro|iphone16-pro-max|iphone17-pro|iphone17-pro-max`)
  - `--settle-ms <SETTLE_MS>` (default: `500`)
  - `--width <WIDTH>` (default: `1290`)
  - `--height <HEIGHT>` (default: `2796`)
  - `--headline <HEADLINE>`
  - `--subheadline <SUBHEADLINE>`
  - `--background <BACKGROUND>` (default: `mesh`; values: `mesh|stripes`)
  - `--seed <SEED>` (default: `42`)
  - `--colors <COLORS>` (comma-separated hex colors)
  - `--auto-colors`
  - `--auto-strategy <AUTO_STRATEGY>` (default: `analogous`; values: `monochromatic|analogous|complementary|triadic`)

## Verification Workflow

- Run a pipeline config and inspect generated output:
  - `cargo run -- run --config ./screenforge.yaml`
  - `open ./output/index.html`

## CI And Review Conventions

- No CI workflow/config files are present in this repository.
- No repository-specific branch or PR convention file is present; use org/user defaults.
