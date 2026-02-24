# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Screenforge is a Rust CLI tool for generating App Store-style marketing screenshots. It captures screenshots (from files or iOS simulators), renders deterministic backgrounds, composites phone mockups with device frames, adds text overlays, and generates an HTML preview.

## Build & Run Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Run tests
cargo test

# Run the pipeline
cargo run -- run --config ./screenforge.yaml

# Quick snap from running simulator
cargo run -- snap "iPhone 16 Pro"
cargo run -- snap --list  # List booted simulators

# List built-in device models
cargo run -- devices

# Import frame overlays
cargo run -- import-frames --source ~/Downloads/frames

# Convert white-screen mockups to transparent overlays
cargo run -- convert-frames --source ./mockups

# Validate overlay files
cargo run -- verify-overlay --config ./screenforge.yaml --strict

# Open preview after running
open ./output/index.html
```

## Architecture

**Data flow:** `config.yaml` → capture → background render → compose → preview

```
src/
├── main.rs        # CLI entry, command dispatch
├── cli.rs         # Clap command definitions
├── config.rs      # YAML config structures (Config, SceneConfig, BackgroundConfig, PhoneConfig, CopyConfig)
├── pipeline.rs    # Orchestrates full run: capture → background → compose → preview
├── capture.rs     # Screenshot adapters: File (load image) or Simctl (xcrun simctl io)
├── background.rs  # Deterministic background rendering (mesh/stripes patterns with ChaCha8Rng seeding)
├── compose.rs     # Image composition: combines screenshot + background + frame + text
├── devices.rs     # Built-in device presets (iPhone 16/17 Pro/Pro Max dimensions, corners, padding)
├── frames.rs      # Transparent PNG overlay loading, import, validation
├── snap.rs        # Quick capture command wrapper
├── simulator.rs   # iOS simulator interaction (xcrun simctl queries)
├── color.rs       # RGB↔HSL, hex parsing, color interpolation
├── palette.rs     # Auto-palette extraction from screenshots (monochromatic, analogous, complementary, triadic)
└── preview.rs     # HTML preview index generation
```

**Key patterns:**
- Adapter pattern for capture sources (`CaptureConfig::File` vs `CaptureConfig::Simctl`)
- Style resolution merges: defaults → model preset → explicit config
- Seeded RNG (`ChaCha8Rng`) ensures deterministic backgrounds from same seed
- Overlay resolution: explicit path → `assets/frames/<model>.png` fallback

## Configuration

See `screenforge.yaml` for a complete example. Key sections:
- `capture`: File adapter (path) or Simctl adapter (device, settle_ms)
- `background`: template (mesh|stripes), seed, colors[] or auto_colors with strategy
- `phone`: model, position (x,y,width,height), optional overlay path
- `copy`: headline, subheadline, color, position, scaling

## Verification

After making changes, use `/verify` to build and visually inspect output in Chrome. This runs the pipeline and opens the HTML preview for visual inspection of:
- Text rendering (readable, properly positioned)
- Phone mockup (frame visible, screenshot inside, Dynamic Island)
- Background (gradient/pattern, colors)
- Layout (no clipping or overflow)
