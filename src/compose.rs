use std::path::Path;

use anyhow::{Context, Result, bail};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use image::imageops::{FilterType, crop_imm};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

use crate::color::parse_hex_rgba;
use crate::config::{CopyConfig, SceneConfig};
use crate::devices::{DynamicIslandSpec, resolve_phone_style};
use crate::frames::resolve_overlay_for_compose;

pub fn compose_scene(
    screenshot: &DynamicImage,
    scene: &SceneConfig,
    mut background: RgbaImage,
    config_dir: &Path,
) -> Result<RgbaImage> {
    if let Some(copy) = &scene.copy {
        draw_copy(&mut background, copy)?;
    }

    let phone = &scene.phone;
    if phone.width == 0 || phone.height == 0 {
        bail!("scene '{}' has invalid phone size", scene.id);
    }

    let style = resolve_phone_style(phone);
    let frame_color = parse_hex_rgba(&style.frame_color)?;

    let shadow_y = phone.y as i32 + style.shadow_offset_y;
    fill_rounded_rect(
        &mut background,
        phone.x as i32,
        shadow_y,
        phone.width,
        phone.height,
        style.corner_radius,
        Rgba([0, 0, 0, style.shadow_alpha]),
    );

    fill_rounded_rect(
        &mut background,
        phone.x as i32,
        phone.y as i32,
        phone.width,
        phone.height,
        style.corner_radius,
        frame_color,
    );
    draw_frame_tones(
        &mut background,
        phone.x as i32,
        phone.y as i32,
        phone.width,
        phone.height,
        style.corner_radius,
    );

    let inset_left = style
        .screen_padding
        .left
        .saturating_add(style.frame_border_width);
    let inset_right = style
        .screen_padding
        .right
        .saturating_add(style.frame_border_width);
    let inset_top = style
        .screen_padding
        .top
        .saturating_add(style.frame_border_width);
    let inset_bottom = style
        .screen_padding
        .bottom
        .saturating_add(style.frame_border_width);

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
    let screenshot_radius = style
        .corner_radius
        .saturating_sub(style.frame_border_width + 2);
    let fitted = resize_cover(screenshot, screen_w, screen_h);
    blit_rounded(
        &mut background,
        &fitted,
        screen_x as i32,
        screen_y as i32,
        screenshot_radius,
    );

    if let Some(island) = style.island {
        draw_dynamic_island(
            &mut background,
            screen_x as i32,
            screen_y as i32,
            screen_w,
            screen_h,
            island,
        );
    }

    if let Some(overlay) = resolve_overlay_for_compose(scene, config_dir) {
        apply_phone_overlay(
            &mut background,
            &overlay.path,
            phone.x as i32,
            phone.y as i32,
            phone.width,
            phone.height,
        )
        .with_context(|| {
            format!(
                "scene '{}' failed applying {} overlay {}",
                scene.id,
                overlay.source.label(),
                overlay.path.display()
            )
        })?;
    }

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

fn draw_frame_tones(image: &mut RgbaImage, x: i32, y: i32, width: u32, height: u32, radius: u32) {
    let top_h = (height / 3).max(8);
    fill_rounded_rect(
        image,
        x + 1,
        y + 1,
        width.saturating_sub(2),
        top_h,
        radius.saturating_sub(1),
        Rgba([255, 255, 255, 20]),
    );

    let bottom_y = y + ((height as i32 * 2) / 3);
    let bottom_h = height.saturating_sub((height * 2) / 3).saturating_sub(2);
    fill_rounded_rect(
        image,
        x + 1,
        bottom_y,
        width.saturating_sub(2),
        bottom_h,
        radius.saturating_sub(1),
        Rgba([0, 0, 0, 28]),
    );
}

fn draw_dynamic_island(
    image: &mut RgbaImage,
    screen_x: i32,
    screen_y: i32,
    screen_w: u32,
    screen_h: u32,
    spec: DynamicIslandSpec,
) {
    let island_w = ((screen_w as f32 * spec.width_ratio).round() as u32)
        .max(48)
        .min(screen_w.saturating_sub(4));
    let island_h = ((screen_h as f32 * spec.height_ratio).round() as u32)
        .max(18)
        .min(screen_h.saturating_sub(2));
    let island_x = screen_x + ((screen_w.saturating_sub(island_w) / 2) as i32);
    let island_y = screen_y + ((screen_h as f32 * spec.y_offset_ratio).round() as i32);

    fill_rounded_rect(
        image,
        island_x,
        island_y,
        island_w,
        island_h,
        island_h / 2,
        Rgba([0, 0, 0, 255]),
    );
    fill_rounded_rect(
        image,
        island_x + 1,
        island_y + 1,
        island_w.saturating_sub(2),
        island_h.saturating_sub(2),
        island_h / 2,
        Rgba([8, 8, 9, 255]),
    );

    let lens_size = ((island_h as f32 * spec.lens_size_ratio).round() as u32)
        .max(4)
        .min(island_h.saturating_sub(4));
    let lens_x = island_x + island_w as i32 - lens_size as i32 - (island_h as i32 / 3);
    let lens_y = island_y + (island_h.saturating_sub(lens_size) / 2) as i32;
    let lens_r = (lens_size / 2) as i32;
    fill_circle(
        image,
        lens_x + lens_r,
        lens_y + lens_r,
        lens_r,
        Rgba([20, 32, 45, 210]),
    );
    fill_circle(
        image,
        lens_x + lens_r / 2,
        lens_y + lens_r / 2,
        (lens_r / 3).max(1),
        Rgba([90, 136, 180, 120]),
    );
}

fn apply_phone_overlay(
    image: &mut RgbaImage,
    overlay_path: &Path,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<()> {
    let overlay = image::open(overlay_path)
        .with_context(|| format!("failed opening overlay {}", overlay_path.display()))?
        .resize_exact(width, height, FilterType::Lanczos3)
        .to_rgba8();

    for yy in 0..overlay.height() as i32 {
        for xx in 0..overlay.width() as i32 {
            let pixel = overlay.get_pixel(xx as u32, yy as u32);
            if pixel[3] == 0 {
                continue;
            }
            blend_pixel(image, x + xx, y + yy, *pixel);
        }
    }

    Ok(())
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

fn fill_circle(image: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    if radius <= 0 {
        return;
    }

    let r2 = radius * radius;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r2 {
                blend_pixel(image, x, y, color);
            }
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
