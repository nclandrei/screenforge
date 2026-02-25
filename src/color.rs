use anyhow::{Result, bail};
use image::Rgba;

/// HSL color representation (hue: 0-360, saturation: 0-1, lightness: 0-1)
#[derive(Debug, Clone, Copy)]
pub struct Hsl {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

impl Hsl {
    pub fn new(h: f32, s: f32, l: f32) -> Self {
        Self {
            h: h % 360.0,
            s: s.clamp(0.0, 1.0),
            l: l.clamp(0.0, 1.0),
        }
    }

    /// Shift hue by degrees (wraps around 360)
    pub fn shift_hue(self, degrees: f32) -> Self {
        Self::new((self.h + degrees + 360.0) % 360.0, self.s, self.l)
    }

    /// Adjust saturation by a factor (clamped 0-1)
    pub fn adjust_saturation(self, factor: f32) -> Self {
        Self::new(self.h, (self.s * factor).clamp(0.0, 1.0), self.l)
    }

    /// Set lightness to a specific value
    pub fn with_lightness(self, l: f32) -> Self {
        Self::new(self.h, self.s, l)
    }

    /// Set saturation to a specific value
    pub fn with_saturation(self, s: f32) -> Self {
        Self::new(self.h, s, self.l)
    }
}

/// Convert RGB to HSL
pub fn rgb_to_hsl(rgba: Rgba<u8>) -> Hsl {
    let r = rgba[0] as f32 / 255.0;
    let g = rgba[1] as f32 / 255.0;
    let b = rgba[2] as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return Hsl::new(0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f32::EPSILON {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
    } else if (max - g).abs() < f32::EPSILON {
        ((b - r) / d + 2.0) * 60.0
    } else {
        ((r - g) / d + 4.0) * 60.0
    };

    Hsl::new(h, s, l)
}

/// Convert HSL to RGB
pub fn hsl_to_rgb(hsl: Hsl) -> Rgba<u8> {
    let Hsl { h, s, l } = hsl;

    if s.abs() < f32::EPSILON {
        let v = (l * 255.0).round() as u8;
        return Rgba([v, v, v, 255]);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let hue_to_rgb = |t: f32| -> f32 {
        let t = if t < 0.0 {
            t + 1.0
        } else if t > 1.0 {
            t - 1.0
        } else {
            t
        };
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };

    let h_norm = h / 360.0;
    let r = (hue_to_rgb(h_norm + 1.0 / 3.0) * 255.0).round() as u8;
    let g = (hue_to_rgb(h_norm) * 255.0).round() as u8;
    let b = (hue_to_rgb(h_norm - 1.0 / 3.0) * 255.0).round() as u8;

    Rgba([r, g, b, 255])
}

/// Convert RGBA to hex string
pub fn rgba_to_hex(rgba: Rgba<u8>) -> String {
    format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2])
}

pub fn parse_hex_rgba(input: &str) -> Result<Rgba<u8>> {
    let value = input.trim();
    let hex = value.strip_prefix('#').unwrap_or(value);

    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            Ok(Rgba([r, g, b, 255]))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            let a = u8::from_str_radix(&hex[6..8], 16)?;
            Ok(Rgba([r, g, b, a]))
        }
        _ => bail!("invalid color '{}': expected #RRGGBB or #RRGGBBAA", input),
    }
}

pub fn lerp_color(a: Rgba<u8>, b: Rgba<u8>, t: f32) -> Rgba<u8> {
    let clamped = t.clamp(0.0, 1.0);
    Rgba([
        lerp_channel(a[0], b[0], clamped),
        lerp_channel(a[1], b[1], clamped),
        lerp_channel(a[2], b[2], clamped),
        lerp_channel(a[3], b[3], clamped),
    ])
}

fn lerp_channel(a: u8, b: u8, t: f32) -> u8 {
    ((a as f32) + ((b as f32) - (a as f32)) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}
