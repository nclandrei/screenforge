# screenforge

A Rust CLI MVP for generating App Store-style screenshots from config.

## What this MVP does

- Reads scenes from YAML config
- Captures raw screenshots via:
  - `file` adapter (ingest an existing image)
  - `simctl` adapter (run `xcrun simctl io <device> screenshot`)
- Renders deterministic backgrounds (`mesh`, `stripes`) with seeded variation
- Composes phone mockups with built-in device presets:
  - `iphone_16_pro`
  - `iphone_16_pro_max`
  - `iphone_17_pro`
  - `iphone_17_pro_max`
- Supports optional transparent PNG overlays for exact hardware chrome
- Writes outputs and an `index.html` preview

## Quick start

```bash
cargo run -- run --config ./screenforge.yaml
```

Open preview:

```bash
open ./output/index.html
```

List supported built-in devices:

```bash
cargo run -- devices
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
      model: iphone_16_pro # iphone_16_pro | iphone_16_pro_max | iphone_17_pro | iphone_17_pro_max
      x: 170
      y: 430
      width: 950
      height: 1980
      # Optional high-fidelity transparent overlay frame.
      # overlay: ./assets/frames/iphone_16_pro_overlay.png
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

## Frame Strategy

- Use built-in model presets for fast output (`model: iphone_16_pro` or `model: iphone_17_pro`).
- Available built-in models: `iphone_16_pro`, `iphone_16_pro_max`, `iphone_17_pro`, `iphone_17_pro_max`.
- For exact industrial design, export a transparent overlay PNG from your frame source and set `phone.overlay`.
- Overlay image is resized to `phone.width` x `phone.height` and blended on top of the generated phone.

## Next MVP extensions

- Better typography (custom fonts, kerning, multiline layout)
- Localization matrix support and per-locale text overrides
- Deterministic scene state hooks for launch/openurl/fixture setup
