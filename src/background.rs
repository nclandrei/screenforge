use anyhow::{Context, Result, bail};
use image::{Rgba, RgbaImage};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::color::{lerp_color, parse_hex_rgba};
use crate::config::{BackgroundConfig, BackgroundTemplate};

pub fn render_background(cfg: &BackgroundConfig, width: u32, height: u32) -> Result<RgbaImage> {
    if width == 0 || height == 0 {
        bail!("invalid canvas size {}x{}", width, height);
    }

    let palette = cfg
        .colors
        .iter()
        .map(|raw| parse_hex_rgba(raw).with_context(|| format!("invalid palette color '{}'", raw)))
        .collect::<Result<Vec<_>>>()?;

    if palette.len() < 2 {
        bail!("background needs at least 2 colors");
    }

    let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed);
    let image = match cfg.template {
        BackgroundTemplate::Mesh => render_mesh(width, height, &palette, &mut rng, cfg.seed),
        BackgroundTemplate::Stripes => render_stripes(width, height, &palette, &mut rng, cfg.seed),
    };

    Ok(image)
}

fn render_mesh(
    width: u32,
    height: u32,
    palette: &[Rgba<u8>],
    rng: &mut ChaCha8Rng,
    seed: u64,
) -> RgbaImage {
    let c0 = palette[rng.gen_range(0..palette.len())];
    let c1 = palette[rng.gen_range(0..palette.len())];
    let c2 = palette[rng.gen_range(0..palette.len())];
    let c3 = palette[rng.gen_range(0..palette.len())];

    let mut out = RgbaImage::new(width, height);
    let width_f = (width.max(1) - 1) as f32;
    let height_f = (height.max(1) - 1) as f32;

    for y in 0..height {
        let fy = y as f32 / height_f.max(1.0);
        for x in 0..width {
            let fx = x as f32 / width_f.max(1.0);

            let top = lerp_color(c0, c1, fx);
            let bottom = lerp_color(c2, c3, fx);
            let mut mixed = lerp_color(top, bottom, fy);

            let dx = (fx - 0.5).abs() * 2.0;
            let dy = (fy - 0.5).abs() * 2.0;
            let vignette = ((dx + dy) * 0.12).clamp(0.0, 0.16);
            let grain = pseudo_noise(seed, x, y) * 10.0;

            for channel in 0..3 {
                let base = mixed[channel] as f32 * (1.0 - vignette) + grain;
                mixed[channel] = base.clamp(0.0, 255.0) as u8;
            }

            out.put_pixel(x, y, mixed);
        }
    }

    out
}

fn render_stripes(
    width: u32,
    height: u32,
    palette: &[Rgba<u8>],
    rng: &mut ChaCha8Rng,
    seed: u64,
) -> RgbaImage {
    let c0 = palette[rng.gen_range(0..palette.len())];
    let c1 = palette[rng.gen_range(0..palette.len())];
    let c2 = palette[rng.gen_range(0..palette.len())];
    let stripe_size = rng.gen_range(28..92) as i32;
    let drift = rng.gen_range(18..72) as i32;

    let mut out = RgbaImage::new(width, height);
    let height_f = (height.max(1) - 1) as f32;

    for y in 0..height {
        let fy = y as f32 / height_f.max(1.0);
        let row_tint = lerp_color(c2, c0, fy);
        for x in 0..width {
            let line = ((x as i32 + y as i32 + drift) / stripe_size) % 2;
            let base = if line == 0 { c0 } else { c1 };
            let mut mixed = lerp_color(base, row_tint, 0.22);
            let grain = pseudo_noise(seed.wrapping_mul(13), x, y) * 8.0;
            for channel in 0..3 {
                let value = mixed[channel] as f32 + grain;
                mixed[channel] = value.clamp(0.0, 255.0) as u8;
            }
            out.put_pixel(x, y, mixed);
        }
    }

    out
}

fn pseudo_noise(seed: u64, x: u32, y: u32) -> f32 {
    let mut v = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    v ^= v >> 30;
    v = v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    v ^= v >> 27;
    v = v.wrapping_mul(0x94D0_49BB_1331_11EB);
    v ^= v >> 31;
    let n = (v & 1023) as f32 / 1023.0;
    (n - 0.5) * 2.0
}
