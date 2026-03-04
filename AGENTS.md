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

## Current Snap CLI Behavior

The `snap` command supports auto-generated palettes from the captured screenshot:

- `--auto-colors`
- `--auto-strategy monochromatic|analogous|complementary|triadic`

## Verification Workflow

- Run a pipeline config and inspect generated output:
  - `cargo run -- run --config ./screenforge.yaml`
  - `open ./output/index.html`

## CI And Review Conventions

- No CI workflow/config files are present in this repository.
- No repository-specific branch or PR convention file is present; use org/user defaults.
