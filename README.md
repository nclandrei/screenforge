# Screenforge

A Rust CLI for generating App Store-style marketing screenshots.

## Features

- Capture screenshots from files or iOS simulators
- Render deterministic backgrounds (mesh gradients, stripes)
- Auto-extract color palettes from screenshots
- Composite phone mockups with accurate device frames
- Render Dynamic Island for supported devices
- Add headline and subheadline text overlays
- Generate HTML preview gallery
- Support for transparent PNG frame overlays

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
```

## Quick Start

Run the full pipeline from a config file:

```bash
screenforge run --config screenforge.yaml
open output/index.html
```

Capture a framed screenshot from a running iOS simulator:

```bash
screenforge snap "iPhone 16 Pro" --headline "Your App" --subheadline "Tagline here"
```

List booted simulators:

```bash
screenforge snap --list
```

## Commands

### run

Execute the full pipeline: capture, background, compose, and preview.

```bash
screenforge run --config ./screenforge.yaml
```

### snap

Capture and frame a screenshot from a running iOS simulator. Auto-detects device model.

```bash
screenforge snap "iPhone 16 Pro"
screenforge snap "My-Simulator" --output hero.png
screenforge snap --raw                           # Raw screenshot without framing
screenforge snap --auto-colors --auto-strategy analogous
```

Options:
- `--output` - Output file path (default: `snap_output.png`)
- `--raw` - Capture raw screenshot without framing
- `--model` - Override auto-detected phone model
- `--headline` / `--subheadline` - Text overlays
- `--background` - Background template (`mesh` or `stripes`)
- `--seed` - Background seed for deterministic generation
- `--colors` - Comma-separated hex colors
- `--auto-colors` - Extract colors from screenshot
- `--auto-strategy` - Color strategy (`monochromatic`, `analogous`, `complementary`, `triadic`)
- `--width` / `--height` - Output canvas dimensions
- `--settle-ms` - Wait time before capture (default: 500ms)
- `--format` - Output format (`text` or `json`)

### devices

List built-in phone model presets.

```bash
screenforge devices
```

Supported models:
- `iphone_16_pro`
- `iphone_16_pro_max`
- `iphone_17_pro`
- `iphone_17_pro_max`

### import-frames

Import transparent PNG frame overlays into `assets/frames`.

```bash
screenforge import-frames --source ~/Downloads/frames
screenforge import-frames --source ./frames --dest assets/frames --overwrite
```

### convert-frames

Convert mockup frames with white screens to transparent overlays.

```bash
screenforge convert-frames --source ./mockups
screenforge convert-frames --source ./mockups --white-threshold 240
```

### verify-overlay

Validate overlay files referenced by config scenes.

```bash
screenforge verify-overlay --config screenforge.yaml
screenforge verify-overlay --config screenforge.yaml --strict
```

Checks:
- Overlay file exists
- Overlay decodes correctly
- Overlay has transparent pixels
- Overlay dimensions match phone dimensions (warning unless `--strict`)

## Configuration

### Full Example

```yaml
output_dir: ./output

scenes:
  - id: home_screen
    capture:
      adapter: file
      path: ./screenshots/home.png
    output:
      filename: 01-home.png
      width: 1290
      height: 2796
    background:
      template: mesh
      seed: 42
      colors:
        - "#0B1022"
        - "#16479A"
        - "#2B8CD6"
        - "#A9E7FF"
    phone:
      model: iphone_16_pro
      x: 170
      y: 430
      width: 950
      height: 1980
    copy:
      headline: "Your Headline"
      subheadline: "Your subheadline text here."
      color: "#FFFFFF"
      x: 86
      y: 94
      headline_size: 120
      subheadline_size: 56
      line_gap: 24
```

### Capture Adapters

**File adapter** - Load an existing image:

```yaml
capture:
  adapter: file
  path: ./screenshots/home.png
```

**Simctl adapter** - Capture from iOS simulator:

```yaml
capture:
  adapter: simctl
  device: "booted"      # or simulator UDID
  settle_ms: 1200       # wait before capture
```

### Background Options

**Mesh gradient:**

```yaml
background:
  template: mesh
  seed: 42
  colors:
    - "#0B1022"
    - "#16479A"
    - "#2B8CD6"
    - "#A9E7FF"
```

**Stripes:**

```yaml
background:
  template: stripes
  seed: 99
  colors:
    - "#04172B"
    - "#0773B8"
    - "#37C4AA"
    - "#D0FFF1"
```

**Auto-extracted colors:**

```yaml
background:
  template: mesh
  seed: 42
  auto_colors: true
  auto_strategy: analogous  # monochromatic | analogous | complementary | triadic
```

Color strategies:
- `monochromatic` - Variations of the dominant color
- `analogous` - Adjacent colors on the color wheel
- `complementary` - Opposite colors for high contrast
- `triadic` - Three equally spaced colors

### Phone Configuration

```yaml
phone:
  model: iphone_16_pro       # optional, enables preset styling
  x: 170                     # horizontal position
  y: 430                     # vertical position
  width: 950                 # phone width
  height: 1980               # phone height
  corner_radius: 116         # optional, defaults from model
  frame_color: "#7A7F89"     # optional, defaults from model
  frame_border_width: 13     # optional, defaults from model
  shadow_offset_y: 24        # optional, defaults from model
  shadow_alpha: 82           # optional, defaults from model
  overlay: ./frames/custom.png  # optional transparent frame overlay
```

If `overlay` is not specified, Screenforge looks for `assets/frames/<model>.png`.

### Text Configuration

```yaml
copy:
  headline: "Your Headline"
  subheadline: "Supporting text"
  color: "#FFFFFF"
  x: 86
  y: 94
  headline_size: 120           # font size in pixels
  subheadline_size: 56
  headline_weight: bold        # regular | medium | semi_bold | bold
  subheadline_weight: regular
  line_gap: 24                 # gap between headline and subheadline
  max_width: 1000              # optional, for text wrapping
```

## Frame Overlays

For pixel-perfect device frames, use transparent PNG overlays:

1. Import existing overlays:
   ```bash
   screenforge import-frames --source ~/Downloads/frames
   ```

2. Or convert white-screen mockups:
   ```bash
   screenforge convert-frames --source ./mockups
   ```

3. Validate overlays:
   ```bash
   screenforge verify-overlay --config screenforge.yaml --strict
   ```

Overlays are auto-loaded from `assets/frames/<model>.png` when available.

## Output

Running the pipeline generates:
- Individual PNG files in `output_dir`
- `index.html` preview gallery

## License

MIT
