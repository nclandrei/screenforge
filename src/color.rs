use anyhow::{Result, bail};
use image::Rgba;

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
