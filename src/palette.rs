use image::{DynamicImage, Rgba};
use std::collections::HashMap;

use crate::color::{hsl_to_rgb, rgb_to_hsl, rgba_to_hex, Hsl};

/// Strategy for generating background palette from dominant colors
#[derive(Debug, Clone, Copy, Default)]
pub enum PaletteStrategy {
    /// Darker/lighter variations of dominant color (good for dark apps)
    #[default]
    Monochromatic,
    /// Colors adjacent on the color wheel (harmonious)
    Analogous,
    /// Opposite on color wheel (high contrast)
    Complementary,
    /// Three colors equally spaced (vibrant)
    Triadic,
}

/// Extract dominant colors from an image by sampling and clustering
pub fn extract_dominant_colors(image: &DynamicImage, count: usize) -> Vec<Rgba<u8>> {
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Sample pixels - focus on edges where background meets frame
    let mut samples = Vec::new();

    // Sample from edges (top/bottom 15%, left/right 15%)
    let edge_margin_x = (width as f32 * 0.15) as u32;
    let edge_margin_y = (height as f32 * 0.15) as u32;

    // Also sample from center for balance
    let step = 8; // Sample every 8th pixel for performance

    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let is_edge = x < edge_margin_x
                || x > width - edge_margin_x
                || y < edge_margin_y
                || y > height - edge_margin_y;

            // Weight edges more heavily (sample them)
            if is_edge || (x % 16 == 0 && y % 16 == 0) {
                let pixel = rgba.get_pixel(x, y);
                // Skip fully transparent pixels
                if pixel[3] > 128 {
                    samples.push(*pixel);
                }
            }
        }
    }

    if samples.is_empty() {
        // Fallback: dark color
        return vec![Rgba([30, 30, 40, 255])];
    }

    // Simple color quantization using histogram binning
    // Reduce color space to 32 levels per channel
    let mut histogram: HashMap<(u8, u8, u8), usize> = HashMap::new();
    for pixel in &samples {
        let key = (pixel[0] / 8, pixel[1] / 8, pixel[2] / 8);
        *histogram.entry(key).or_insert(0) += 1;
    }

    // Sort by frequency and take top colors
    let mut sorted: Vec<_> = histogram.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    // Convert back to full colors and filter similar ones
    let mut dominant = Vec::new();
    for ((r, g, b), _) in sorted {
        let color = Rgba([r * 8 + 4, g * 8 + 4, b * 8 + 4, 255]);

        // Skip if too similar to an existing dominant color
        let dominated = dominant.iter().any(|existing: &Rgba<u8>| {
            let dr = (color[0] as i32 - existing[0] as i32).abs();
            let dg = (color[1] as i32 - existing[1] as i32).abs();
            let db = (color[2] as i32 - existing[2] as i32).abs();
            dr + dg + db < 60 // Similarity threshold
        });

        if !dominated {
            dominant.push(color);
            if dominant.len() >= count {
                break;
            }
        }
    }

    // Ensure we have at least one color
    if dominant.is_empty() {
        dominant.push(Rgba([30, 30, 40, 255]));
    }

    dominant
}

/// Generate a background palette from dominant colors using the specified strategy
pub fn generate_palette(dominant: &[Rgba<u8>], strategy: PaletteStrategy) -> Vec<String> {
    if dominant.is_empty() {
        return default_palette();
    }

    // Use the most dominant color as the base
    let base = dominant[0];
    let base_hsl = rgb_to_hsl(base);

    let colors = match strategy {
        PaletteStrategy::Monochromatic => generate_monochromatic(base_hsl),
        PaletteStrategy::Analogous => generate_analogous(base_hsl),
        PaletteStrategy::Complementary => generate_complementary(base_hsl, dominant),
        PaletteStrategy::Triadic => generate_triadic(base_hsl),
    };

    colors.into_iter().map(|c| rgba_to_hex(c)).collect()
}

fn generate_monochromatic(base: Hsl) -> Vec<Rgba<u8>> {
    // Create variations in lightness and saturation
    // Start dark, end light for good gradient backgrounds
    vec![
        hsl_to_rgb(base.with_lightness(0.08).with_saturation(base.s * 0.7)),
        hsl_to_rgb(base.with_lightness(0.18).with_saturation(base.s * 0.85)),
        hsl_to_rgb(base.with_lightness(0.35).with_saturation(base.s * 1.0)),
        hsl_to_rgb(base.with_lightness(0.55).with_saturation(base.s * 0.6)),
    ]
}

fn generate_analogous(base: Hsl) -> Vec<Rgba<u8>> {
    // Colors adjacent on the color wheel (±30°, ±15°)
    vec![
        hsl_to_rgb(base.shift_hue(-25.0).with_lightness(0.12)),
        hsl_to_rgb(base.with_lightness(0.22)),
        hsl_to_rgb(base.shift_hue(25.0).with_lightness(0.38)),
        hsl_to_rgb(base.shift_hue(15.0).with_lightness(0.58)),
    ]
}

fn generate_complementary(base: Hsl, dominant: &[Rgba<u8>]) -> Vec<Rgba<u8>> {
    // Use complement (180° shift) for contrast
    let complement = base.shift_hue(180.0);

    // If we have a second dominant color, use it too
    let accent = if dominant.len() > 1 {
        rgb_to_hsl(dominant[1])
    } else {
        base.shift_hue(90.0)
    };

    vec![
        hsl_to_rgb(base.with_lightness(0.10)),
        hsl_to_rgb(complement.with_lightness(0.25).adjust_saturation(0.7)),
        hsl_to_rgb(accent.with_lightness(0.40)),
        hsl_to_rgb(base.with_lightness(0.60).adjust_saturation(0.5)),
    ]
}

fn generate_triadic(base: Hsl) -> Vec<Rgba<u8>> {
    // Three colors equally spaced (120° apart)
    vec![
        hsl_to_rgb(base.with_lightness(0.12)),
        hsl_to_rgb(base.shift_hue(120.0).with_lightness(0.28)),
        hsl_to_rgb(base.shift_hue(240.0).with_lightness(0.42)),
        hsl_to_rgb(base.with_lightness(0.55)),
    ]
}

fn default_palette() -> Vec<String> {
    vec![
        "#0E1228".to_string(),
        "#1348A5".to_string(),
        "#2B8CD6".to_string(),
        "#C2E6FF".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_returns_colors() {
        let img = DynamicImage::new_rgba8(100, 100);
        let colors = extract_dominant_colors(&img, 4);
        assert!(!colors.is_empty());
    }

    #[test]
    fn test_generate_palette_monochromatic() {
        let dominant = vec![Rgba([100, 50, 150, 255])];
        let palette = generate_palette(&dominant, PaletteStrategy::Monochromatic);
        assert_eq!(palette.len(), 4);
        assert!(palette[0].starts_with('#'));
    }
}
