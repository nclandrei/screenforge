use anyhow::{Result, bail};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use image::imageops::{FilterType, crop_imm};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

use crate::color::parse_hex_rgba;
use crate::config::{CopyConfig, SceneConfig};

pub fn compose_scene(
    screenshot: &DynamicImage,
    scene: &SceneConfig,
    mut background: RgbaImage,
) -> Result<RgbaImage> {
    if let Some(copy) = &scene.copy {
        draw_copy(&mut background, copy)?;
    }

    let frame_color = parse_hex_rgba(&scene.phone.frame_color)?;
    let phone = &scene.phone;

    if phone.width == 0 || phone.height == 0 {
        bail!("scene '{}' has invalid phone size", scene.id);
    }

    let shadow_y = phone.y as i32 + phone.shadow_offset_y;
    fill_rounded_rect(
        &mut background,
        phone.x as i32,
        shadow_y,
        phone.width,
        phone.height,
        phone.corner_radius,
        Rgba([0, 0, 0, phone.shadow_alpha]),
    );

    fill_rounded_rect(
        &mut background,
        phone.x as i32,
        phone.y as i32,
        phone.width,
        phone.height,
        phone.corner_radius,
        frame_color,
    );

    let inset_left = phone
        .screen_padding
        .left
        .saturating_add(phone.frame_border_width);
    let inset_right = phone
        .screen_padding
        .right
        .saturating_add(phone.frame_border_width);
    let inset_top = phone
        .screen_padding
        .top
        .saturating_add(phone.frame_border_width);
    let inset_bottom = phone
        .screen_padding
        .bottom
        .saturating_add(phone.frame_border_width);

    let screen_w = phone
        .width
        .saturating_sub(inset_left.saturating_add(inset_right));
    let screen_h = phone
        .height
        .saturating_sub(inset_top.saturating_add(inset_bottom));

    if screen_w == 0 || screen_h == 0 {
        bail!(
            "scene '{}' phone insets leave no space for screenshot",
            scene.id
        );
    }

    let screen_x = phone.x.saturating_add(inset_left);
    let screen_y = phone.y.saturating_add(inset_top);
    let screenshot_radius = phone
        .corner_radius
        .saturating_sub(phone.frame_border_width + 2);
    let fitted = resize_cover(screenshot, screen_w, screen_h);
    blit_rounded(
        &mut background,
        &fitted,
        screen_x as i32,
        screen_y as i32,
        screenshot_radius,
    );

    Ok(background)
}

fn draw_copy(image: &mut RgbaImage, copy: &CopyConfig) -> Result<()> {
    let color = parse_hex_rgba(&copy.color)?;
    let max_width = image.width().saturating_sub(copy.x.saturating_add(80));
    let used = draw_text_wrapped(
        image,
        &copy.headline,
        copy.x as i32,
        copy.y as i32,
        copy.headline_scale.max(1),
        color,
        max_width,
    );

    if !copy.subheadline.trim().is_empty() {
        let sub_y = copy.y.saturating_add(used).saturating_add(copy.line_gap);
        draw_text_wrapped(
            image,
            &copy.subheadline,
            copy.x as i32,
            sub_y as i32,
            copy.subheadline_scale.max(1),
            color,
            max_width,
        );
    }

    Ok(())
}

fn draw_text_wrapped(
    image: &mut RgbaImage,
    text: &str,
    start_x: i32,
    start_y: i32,
    scale: u32,
    color: Rgba<u8>,
    max_width: u32,
) -> u32 {
    if max_width == 0 {
        return 0;
    }

    let glyph_w = 8 * scale;
    let line_height = 10 * scale;
    let max_chars = (max_width / glyph_w.max(1)).max(1) as usize;
    let lines = wrap_text(text, max_chars);

    for (line_index, line) in lines.iter().enumerate() {
        let y = start_y + (line_index as i32 * line_height as i32);
        draw_bitmap_text(image, line, start_x, y, scale, color);
    }

    lines.len() as u32 * line_height
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    for hard_line in text.lines() {
        if hard_line.chars().count() <= max_chars {
            out.push(hard_line.to_string());
            continue;
        }

        let mut current = String::new();
        for word in hard_line.split_whitespace() {
            if current.is_empty() {
                current.push_str(word);
                continue;
            }
            let next_len = current.chars().count() + 1 + word.chars().count();
            if next_len <= max_chars {
                current.push(' ');
                current.push_str(word);
            } else {
                out.push(current);
                current = word.to_string();
            }
        }

        if !current.is_empty() {
            out.push(current);
        }
    }

    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn draw_bitmap_text(
    image: &mut RgbaImage,
    text: &str,
    start_x: i32,
    start_y: i32,
    scale: u32,
    color: Rgba<u8>,
) {
    let mut cursor_x = start_x;
    for ch in text.chars() {
        let glyph = BASIC_FONTS
            .get(ch)
            .or_else(|| BASIC_FONTS.get('?'))
            .unwrap_or([0; 8]);

        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..8 {
                if (bits >> col) & 1 == 0 {
                    continue;
                }

                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = cursor_x + ((7 - col) as i32 * scale as i32) + dx as i32;
                        let py = start_y + (row as i32 * scale as i32) + dy as i32;
                        blend_pixel(image, px, py, color);
                    }
                }
            }
        }

        cursor_x += (8 * scale + scale) as i32;
    }
}

fn resize_cover(source: &DynamicImage, target_w: u32, target_h: u32) -> RgbaImage {
    let (src_w, src_h) = source.dimensions();
    let scale = (target_w as f32 / src_w as f32).max(target_h as f32 / src_h as f32);
    let resized_w = ((src_w as f32 * scale).ceil() as u32).max(target_w);
    let resized_h = ((src_h as f32 * scale).ceil() as u32).max(target_h);

    let resized = source
        .resize_exact(resized_w, resized_h, FilterType::Lanczos3)
        .to_rgba8();
    let crop_x = (resized_w - target_w) / 2;
    let crop_y = (resized_h - target_h) / 2;
    crop_imm(&resized, crop_x, crop_y, target_w, target_h).to_image()
}

fn fill_rounded_rect(
    image: &mut RgbaImage,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: u32,
    color: Rgba<u8>,
) {
    let w = width as i32;
    let h = height as i32;

    for yy in 0..h {
        for xx in 0..w {
            if !inside_rounded_rect(xx, yy, w, h, radius as i32) {
                continue;
            }
            blend_pixel(image, x + xx, y + yy, color);
        }
    }
}

fn blit_rounded(image: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32, radius: u32) {
    let w = src.width() as i32;
    let h = src.height() as i32;
    for yy in 0..h {
        for xx in 0..w {
            if !inside_rounded_rect(xx, yy, w, h, radius as i32) {
                continue;
            }
            let pixel = src.get_pixel(xx as u32, yy as u32);
            blend_pixel(image, x + xx, y + yy, *pixel);
        }
    }
}

fn inside_rounded_rect(px: i32, py: i32, w: i32, h: i32, radius: i32) -> bool {
    if radius <= 0 {
        return true;
    }
    let r = radius.min(w / 2).min(h / 2);
    if px >= r && px < (w - r) {
        return true;
    }
    if py >= r && py < (h - r) {
        return true;
    }

    let cx = if px < r { r - 1 } else { w - r };
    let cy = if py < r { r - 1 } else { h - r };
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= r * r
}

fn blend_pixel(image: &mut RgbaImage, x: i32, y: i32, src: Rgba<u8>) {
    if x < 0 || y < 0 {
        return;
    }

    let (x, y) = (x as u32, y as u32);
    if x >= image.width() || y >= image.height() {
        return;
    }

    let dst = image.get_pixel(x, y);
    let alpha = src[3] as f32 / 255.0;
    let inv = 1.0 - alpha;
    let out = Rgba([
        (src[0] as f32 * alpha + dst[0] as f32 * inv)
            .round()
            .clamp(0.0, 255.0) as u8,
        (src[1] as f32 * alpha + dst[1] as f32 * inv)
            .round()
            .clamp(0.0, 255.0) as u8,
        (src[2] as f32 * alpha + dst[2] as f32 * inv)
            .round()
            .clamp(0.0, 255.0) as u8,
        255,
    ]);
    image.put_pixel(x, y, out);
}
