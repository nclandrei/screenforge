# screenforge

A Rust CLI MVP for generating App Store-style screenshots from config.

## What this MVP does

- Reads scenes from YAML config
- Captures raw screenshots via:
  - `file` adapter (ingest an existing image)
  - `simctl` adapter (run `xcrun simctl io <device> screenshot`)
- Renders deterministic backgrounds (`mesh`, `stripes`) with seeded variation
- Composes a phone mockup (rounded frame + screenshot + text)
- Writes outputs and an `index.html` preview

## Quick start

```bash
cargo run -- run --config ./screenforge.yaml
```

Open preview:

```bash
open ./output/index.html
```

## Config shape

```yaml
output_dir: ./output
scenes:
  - id: my_scene
    capture:
      adapter: file
      path: ./examples/input/demo.ppm
    output:
      filename: 01-home.png
      width: 1290
      height: 2796
    background:
      template: mesh # mesh | stripes
      seed: 42
      colors: ["#0B1022", "#16479A", "#2B8CD6", "#A9E7FF"]
    phone:
      x: 170
      y: 430
      width: 950
      height: 1980
      corner_radius: 96
      frame_color: "#11151B"
      frame_border_width: 10
      screen_padding: { top: 34, right: 24, bottom: 34, left: 24 }
      shadow_offset_y: 20
      shadow_alpha: 76
    copy:
      headline: "BUILD FOCUS FAST"
      subheadline: "One clean flow for capture, layout, and export."
      color: "#F4F8FF"
      x: 86
      y: 94
      headline_scale: 7
      subheadline_scale: 3
      line_gap: 14
```

## simctl example

```yaml
capture:
  adapter: simctl
  device: "booted"
  settle_ms: 1200
```

Notes:
- `device` can be `booted` or a simulator UDID.
- MVP assumes the app is already on the desired screen before capture.

## Next MVP extensions

- Device packs with per-device screen cutout metadata
- Better typography (custom fonts, kerning, multiline layout)
- Localization matrix support and per-locale text overrides
- Deterministic scene state hooks for launch/openurl/fixture setup
