use std::collections::VecDeque;
use std::path::Path;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use anyhow::{Context, Result, bail};
use image::imageops::{FilterType, crop_imm};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

use crate::color::parse_hex_rgba;
use crate::config::{CopyConfig, FontWeight, PhoneConfig, SceneConfig, TextPosition};
use crate::devices::{DynamicIslandSpec, resolve_phone_style};
use crate::frames::resolve_overlay_for_compose;

// Embed Geist fonts directly in the binary
static GEIST_REGULAR: &[u8] = include_bytes!("../assets/fonts/Geist-Regular.ttf");
static GEIST_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Geist-Medium.ttf");
static GEIST_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Geist-SemiBold.ttf");
static GEIST_BOLD: &[u8] = include_bytes!("../assets/fonts/Geist-Bold.ttf");
const OVERLAY_INNER_INSET_PX: u32 = 0;
const OVERLAY_CUTOUT_ALPHA_MAX: u8 = 254;
const OVERLAY_CUTOUT_GUARD_PX: i32 = 0;
const OVERLAY_SEMITRANSPARENT_EXPAND_STEPS: usize = 0;
const OVERLAY_SEMITRANSPARENT_LUMA_MAX: u16 = 30;

pub fn compose_scene(
    screenshot: &DynamicImage,
    scene: &SceneConfig,
    mut background: RgbaImage,
    config_dir: &Path,
) -> Result<RgbaImage> {
    if let Some(copy) = &scene.copy {
        draw_copy(&mut background, copy, &scene.phone)?;
    }

    let phone = &scene.phone;
    if phone.width == 0 || phone.height == 0 {
        bail!("scene '{}' has invalid phone size", scene.id);
    }

    let style = resolve_phone_style(phone);
    let overlay = resolve_overlay_for_compose(scene, config_dir);

    // Only draw programmatic frame if no overlay is provided
    if overlay.is_none() {
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
    }

    let overlay_screen = overlay
        .as_ref()
        .map(|ov| detect_overlay_screen_region(&ov.path))
        .transpose()
        .with_context(|| {
            format!(
                "scene '{}' failed detecting overlay screen region",
                scene.id
            )
        })?
        .flatten();

    let (screen_x, screen_y, screen_w, screen_h) = if let Some(region) = overlay_screen {
        // Derive the display area from the actual transparent cutout in the overlay.
        let sx = phone.width as f32 / region.overlay_width as f32;
        let sy = phone.height as f32 / region.overlay_height as f32;

        let mut screen_x = phone
            .x
            .saturating_add((region.x as f32 * sx).round() as u32);
        let mut screen_y = phone
            .y
            .saturating_add((region.y as f32 * sy).round() as u32);
        let mut screen_w = ((region.width as f32 * sx).round() as u32).max(1);
        let mut screen_h = ((region.height as f32 * sy).round() as u32).max(1);

        // Keep the screenshot a few pixels inside the cutout to avoid a "glued" edge look.
        let inset = OVERLAY_INNER_INSET_PX
            .min(screen_w.saturating_sub(1) / 2)
            .min(screen_h.saturating_sub(1) / 2);
        if inset > 0 {
            screen_x = screen_x.saturating_add(inset);
            screen_y = screen_y.saturating_add(inset);
            screen_w = screen_w.saturating_sub(inset.saturating_mul(2));
            screen_h = screen_h.saturating_sub(inset.saturating_mul(2));
        }

        (screen_x, screen_y, screen_w, screen_h)
    } else {
        // Fall back to model-based insets when no transparent overlay cutout is available.
        let (inset_adjust_top, inset_adjust_side) = if overlay.is_some() {
            use crate::config::PhoneModel;
            match phone.model {
                Some(PhoneModel::Iphone16ProMax) => (12, 6),
                Some(PhoneModel::Iphone17ProMax) => (10, 5),
                _ => (0, 0),
            }
        } else {
            (0, 0)
        };

        let inset_left = style
            .screen_padding
            .left
            .saturating_add(style.frame_border_width)
            .saturating_sub(inset_adjust_side);
        let inset_right = style
            .screen_padding
            .right
            .saturating_add(style.frame_border_width)
            .saturating_sub(inset_adjust_side);
        let inset_top = style
            .screen_padding
            .top
            .saturating_add(style.frame_border_width)
            .saturating_sub(inset_adjust_top);
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
        (screen_x, screen_y, screen_w, screen_h)
    };

    // When using overlay, use corner radius that fits within the frame's screen cutout
    // Each device model has a different frame geometry requiring a specific radius
    // Pro Max frames (1520x3068) have different geometry than Pro frames (1406x2822)
    let screenshot_radius = if overlay.is_some() {
        use crate::config::PhoneModel;
        let ratio = match phone.model {
            Some(PhoneModel::Iphone16Pro) => 0.16,
            Some(PhoneModel::Iphone17Pro) => 0.145,
            Some(PhoneModel::Iphone16ProMax) => 0.16,
            Some(PhoneModel::Iphone17ProMax) => 0.155,
            _ => 0.145,
        };
        (phone.width as f32 * ratio).round() as u32
    } else {
        style
            .corner_radius
            .saturating_sub(style.frame_border_width + 2)
    };

    let fitted = resize_cover(screenshot, screen_w, screen_h);
    if let Some(ref ov) = overlay {
        if overlay_screen.is_some() {
            let overlay_mask = image::open(&ov.path)
                .with_context(|| format!("failed opening overlay {}", ov.path.display()))?
                .resize_exact(phone.width, phone.height, FilterType::Lanczos3)
                .to_rgba8();
            let cutout_mask = build_inner_cutout_mask(&overlay_mask);
            blit_with_overlay_cutout(
                &mut background,
                &fitted,
                screen_x as i32,
                screen_y as i32,
                phone.x as i32,
                phone.y as i32,
                &cutout_mask,
                overlay_mask.width(),
                overlay_mask.height(),
            );
        } else {
            blit_rounded(
                &mut background,
                &fitted,
                screen_x as i32,
                screen_y as i32,
                screenshot_radius,
            );
        }
    } else {
        blit_rounded(
            &mut background,
            &fitted,
            screen_x as i32,
            screen_y as i32,
            screenshot_radius,
        );
    }

    if let Some(ref ov) = overlay {
        // Use the overlay PNG for the frame
        apply_phone_overlay(
            &mut background,
            &ov.path,
            phone.x as i32,
            phone.y as i32,
            phone.width,
            phone.height,
        )
        .with_context(|| {
            format!(
                "scene '{}' failed applying {} overlay {}",
                scene.id,
                ov.source.label(),
                ov.path.display()
            )
        })?;
    } else if let Some(island) = style.island {
        // Only draw programmatic dynamic island if no overlay
        draw_dynamic_island(
            &mut background,
            screen_x as i32,
            screen_y as i32,
            screen_w,
            screen_h,
            island,
        );
    }

    Ok(background)
}

fn get_font(weight: FontWeight) -> Result<FontRef<'static>> {
    let data = match weight {
        FontWeight::Regular => GEIST_REGULAR,
        FontWeight::Medium => GEIST_MEDIUM,
        FontWeight::SemiBold => GEIST_SEMIBOLD,
        FontWeight::Bold => GEIST_BOLD,
    };
    FontRef::try_from_slice(data).context("failed to load embedded Geist font")
}

fn draw_copy(image: &mut RgbaImage, copy: &CopyConfig, phone: &PhoneConfig) -> Result<()> {
    let color = parse_hex_rgba(&copy.color)?;
    let image_width = image.width();
    let image_height = image.height();

    // Default max_width to 80% of image width for centered text
    let max_width = copy
        .max_width
        .unwrap_or_else(|| (image_width as f32 * 0.8) as u32);

    // Pre-calculate text dimensions to determine total height
    let headline_font = get_font(copy.headline_weight)?;
    let headline_scale = PxScale::from(copy.headline_size);
    let headline_scaled = headline_font.as_scaled(headline_scale);
    let headline_lines = wrap_text_by_width(&copy.headline, &headline_scaled, max_width as f32);
    let headline_line_height = (headline_scaled.height() * 1.2).ceil() as u32;
    let headline_total_height = headline_lines.len() as u32 * headline_line_height;

    let (subheadline_lines, subheadline_total_height) = if !copy.subheadline.trim().is_empty() {
        let subheadline_font = get_font(copy.subheadline_weight)?;
        let sub_scale = PxScale::from(copy.subheadline_size);
        let sub_scaled = subheadline_font.as_scaled(sub_scale);
        let lines = wrap_text_by_width(&copy.subheadline, &sub_scaled, max_width as f32);
        let line_height = (sub_scaled.height() * 1.2).ceil() as u32;
        let total = lines.len() as u32 * line_height;
        (lines, total)
    } else {
        (vec![], 0)
    };

    let total_text_height = headline_total_height
        + if subheadline_total_height > 0 {
            copy.line_gap + subheadline_total_height
        } else {
            0
        };

    // Calculate base Y position based on TextPosition preset
    let padding = 60u32; // Default padding from edges
    let base_y = match copy.position {
        TextPosition::AbovePhone => {
            // Center text in the space above the phone
            let space_above = phone.y;
            if space_above > total_text_height {
                ((space_above - total_text_height) / 2) as i32
            } else {
                padding as i32
            }
        }
        TextPosition::BelowPhone => {
            // Center text in the space below the phone
            let phone_bottom = phone.y + phone.height;
            let space_below = image_height.saturating_sub(phone_bottom);
            if space_below > total_text_height {
                (phone_bottom + (space_below - total_text_height) / 2) as i32
            } else {
                (phone_bottom + padding) as i32
            }
        }
        TextPosition::Top => padding as i32,
        TextPosition::Bottom => {
            (image_height
                .saturating_sub(total_text_height)
                .saturating_sub(padding)) as i32
        }
    };

    // Apply user's y_offset adjustment
    let final_y = (base_y + copy.y_offset).max(0) as u32;

    // Draw headline lines centered
    let mut current_y = final_y;
    for line in &headline_lines {
        let line_width = measure_text_width(line, &headline_scaled);
        let x = ((image_width as f32 - line_width) / 2.0).max(0.0) as i32;
        draw_text_line(image, line, x, current_y as i32, &headline_scaled, color);
        current_y += headline_line_height;
    }

    // Draw subheadline lines centered
    if !subheadline_lines.is_empty() {
        current_y += copy.line_gap;
        let subheadline_font = get_font(copy.subheadline_weight)?;
        let sub_scale = PxScale::from(copy.subheadline_size);
        let sub_scaled = subheadline_font.as_scaled(sub_scale);
        let sub_line_height = (sub_scaled.height() * 1.2).ceil() as u32;

        for line in &subheadline_lines {
            let line_width = measure_text_width(line, &sub_scaled);
            let x = ((image_width as f32 - line_width) / 2.0).max(0.0) as i32;
            draw_text_line(image, line, x, current_y as i32, &sub_scaled, color);
            current_y += sub_line_height;
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct OverlayScreenRegion {
    overlay_width: u32,
    overlay_height: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn detect_overlay_screen_region(overlay_path: &Path) -> Result<Option<OverlayScreenRegion>> {
    let overlay = image::open(overlay_path)
        .with_context(|| format!("failed opening overlay {}", overlay_path.display()))?
        .to_rgba8();
    let width = overlay.width() as usize;
    let height = overlay.height() as usize;
    if width == 0 || height == 0 {
        return Ok(None);
    }

    let data = overlay.as_raw();
    let mut visited = vec![false; width * height];
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    let mut best: Option<(usize, usize, usize, usize, usize)> = None;

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;
            if data[idx * 4 + 3] != 0 {
                continue;
            }

            queue.clear();
            queue.push_back((x, y));

            let mut touches_edge = x == 0 || y == 0 || x == width - 1 || y == height - 1;
            let mut count = 0usize;
            let mut min_x = x;
            let mut min_y = y;
            let mut max_x = x;
            let mut max_y = y;

            while let Some((cx, cy)) = queue.pop_front() {
                count += 1;
                if cx < min_x {
                    min_x = cx;
                }
                if cy < min_y {
                    min_y = cy;
                }
                if cx > max_x {
                    max_x = cx;
                }
                if cy > max_y {
                    max_y = cy;
                }
                if cx == 0 || cy == 0 || cx == width - 1 || cy == height - 1 {
                    touches_edge = true;
                }

                let neighbors = [
                    (cx.wrapping_sub(1), cy, cx > 0),
                    (cx + 1, cy, cx + 1 < width),
                    (cx, cy.wrapping_sub(1), cy > 0),
                    (cx, cy + 1, cy + 1 < height),
                ];

                for (nx, ny, valid) in neighbors {
                    if !valid {
                        continue;
                    }
                    let nidx = ny * width + nx;
                    if visited[nidx] {
                        continue;
                    }
                    visited[nidx] = true;
                    if data[nidx * 4 + 3] == 0 {
                        queue.push_back((nx, ny));
                    }
                }
            }

            if !touches_edge
                && best
                    .map(|(best_count, _, _, _, _)| count > best_count)
                    .unwrap_or(true)
            {
                best = Some((count, min_x, min_y, max_x, max_y));
            }
        }
    }

    let Some((_, min_x, min_y, max_x, max_y)) = best else {
        return Ok(None);
    };

    Ok(Some(OverlayScreenRegion {
        overlay_width: width as u32,
        overlay_height: height as u32,
        x: min_x as u32,
        y: min_y as u32,
        width: (max_x - min_x + 1) as u32,
        height: (max_y - min_y + 1) as u32,
    }))
}

fn wrap_text_by_width<F: Font>(
    text: &str,
    font: &ab_glyph::PxScaleFont<&F>,
    max_width: f32,
) -> Vec<String> {
    let mut out = Vec::new();

    for hard_line in text.lines() {
        let line_width = measure_text_width(hard_line, font);
        if line_width <= max_width {
            out.push(hard_line.to_string());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0.0f32;

        for word in hard_line.split_whitespace() {
            let word_width = measure_text_width(word, font);
            let space_width = if current.is_empty() {
                0.0
            } else {
                measure_text_width(" ", font)
            };

            if current_width + space_width + word_width <= max_width {
                if !current.is_empty() {
                    current.push(' ');
                    current_width += space_width;
                }
                current.push_str(word);
                current_width += word_width;
            } else {
                if !current.is_empty() {
                    out.push(current);
                }
                current = word.to_string();
                current_width = word_width;
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

fn measure_text_width<F: Font>(text: &str, font: &ab_glyph::PxScaleFont<&F>) -> f32 {
    let mut width = 0.0f32;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;

    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        if let Some(prev) = prev_glyph {
            width += font.kern(prev, glyph_id);
        }
        width += font.h_advance(glyph_id);
        prev_glyph = Some(glyph_id);
    }

    width
}

fn draw_text_line<F: Font>(
    image: &mut RgbaImage,
    text: &str,
    start_x: i32,
    start_y: i32,
    font: &ab_glyph::PxScaleFont<&F>,
    color: Rgba<u8>,
) {
    let mut cursor_x = start_x as f32;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;

    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);

        if let Some(prev) = prev_glyph {
            cursor_x += font.kern(prev, glyph_id);
        }

        let glyph = glyph_id.with_scale_and_position(
            font.scale(),
            ab_glyph::point(cursor_x, start_y as f32 + font.ascent()),
        );

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                let alpha = (coverage * color[3] as f32).round().clamp(0.0, 255.0) as u8;
                if alpha > 0 {
                    blend_pixel(image, px, py, Rgba([color[0], color[1], color[2], alpha]));
                }
            });
        }

        cursor_x += font.h_advance(glyph_id);
        prev_glyph = Some(glyph_id);
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

fn blit_with_overlay_cutout(
    image: &mut RgbaImage,
    src: &RgbaImage,
    screen_x: i32,
    screen_y: i32,
    phone_x: i32,
    phone_y: i32,
    cutout_mask: &[bool],
    overlay_w: u32,
    overlay_h: u32,
) {
    let src_w = src.width() as i32;
    let src_h = src.height() as i32;
    let mask_width = overlay_w as usize;
    for yy in 0..src_h {
        for xx in 0..src_w {
            let dst_x = screen_x + xx;
            let dst_y = screen_y + yy;
            let ov_x = dst_x - phone_x;
            let ov_y = dst_y - phone_y;
            if ov_x < 0 || ov_y < 0 {
                continue;
            }
            let ov_x = ov_x as u32;
            let ov_y = ov_y as u32;
            if ov_x >= overlay_w || ov_y >= overlay_h {
                continue;
            }
            let mask_idx = ov_y as usize * mask_width + ov_x as usize;
            if !cutout_mask[mask_idx] {
                continue;
            }
            let pixel = src.get_pixel(xx as u32, yy as u32);
            blend_pixel(image, dst_x, dst_y, *pixel);
        }
    }
}

fn build_inner_cutout_mask(overlay: &RgbaImage) -> Vec<bool> {
    let w = overlay.width() as usize;
    let h = overlay.height() as usize;
    let mut outside = vec![false; w * h];
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();

    let is_transparent = |x: usize, y: usize| -> bool {
        overlay.get_pixel(x as u32, y as u32)[3] <= OVERLAY_CUTOUT_ALPHA_MAX
    };

    let mark_outside = |x: usize, y: usize, outside: &mut [bool], queue: &mut VecDeque<(usize, usize)>| {
        let idx = y * w + x;
        if !outside[idx] && is_transparent(x, y) {
            outside[idx] = true;
            queue.push_back((x, y));
        }
    };

    for x in 0..w {
        mark_outside(x, 0, &mut outside, &mut queue);
        mark_outside(x, h - 1, &mut outside, &mut queue);
    }
    for y in 0..h {
        mark_outside(0, y, &mut outside, &mut queue);
        mark_outside(w - 1, y, &mut outside, &mut queue);
    }

    while let Some((x, y)) = queue.pop_front() {
        let neighbors = [
            (x.wrapping_sub(1), y, x > 0),
            (x + 1, y, x + 1 < w),
            (x, y.wrapping_sub(1), y > 0),
            (x, y + 1, y + 1 < h),
        ];
        for (nx, ny, valid) in neighbors {
            if !valid {
                continue;
            }
            let idx = ny * w + nx;
            if outside[idx] || !is_transparent(nx, ny) {
                continue;
            }
            outside[idx] = true;
            queue.push_back((nx, ny));
        }
    }

    let mut inside = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            if is_transparent(x, y) && !outside[idx] {
                inside[idx] = true;
            }
        }
    }

    // Fill a one-pixel dark semi-transparent fringe around the cutout so
    // anti-aliased overlay pixels blend over screenshot (not background).
    for _ in 0..OVERLAY_SEMITRANSPARENT_EXPAND_STEPS {
        let prev_inside = inside.clone();
        let mut expanded = prev_inside.clone();
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                if prev_inside[idx] {
                    continue;
                }
                if !is_dark_semisolid(overlay, x as u32, y as u32) {
                    continue;
                }

                let mut touches_inside = false;
                let mut touches_outside = false;
                let y0 = y.saturating_sub(1);
                let y1 = (y + 1).min(h - 1);
                let x0 = x.saturating_sub(1);
                let x1 = (x + 1).min(w - 1);
                for ny in y0..=y1 {
                    for nx in x0..=x1 {
                        let nidx = ny * w + nx;
                        if prev_inside[nidx] {
                            touches_inside = true;
                        }
                        if outside[nidx] {
                            touches_outside = true;
                        }
                    }
                }

                // Only grow into dark anti-aliased pixels that border the inner cutout.
                // Exclude anything that also touches the external transparent area.
                if touches_inside && !touches_outside {
                    expanded[idx] = true;
                }
            }
        }
        inside = expanded;
    }

    if OVERLAY_CUTOUT_GUARD_PX <= 0 {
        return inside;
    }

    let mut eroded = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            if !inside[idx] {
                continue;
            }
            let mut ok = true;
            for dy in -OVERLAY_CUTOUT_GUARD_PX..=OVERLAY_CUTOUT_GUARD_PX {
                for dx in -OVERLAY_CUTOUT_GUARD_PX..=OVERLAY_CUTOUT_GUARD_PX {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        ok = false;
                        break;
                    }
                    let nidx = ny as usize * w + nx as usize;
                    if !inside[nidx] {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    break;
                }
            }
            eroded[idx] = ok;
        }
    }

    eroded
}

fn is_dark_semisolid(overlay: &RgbaImage, x: u32, y: u32) -> bool {
    let p = overlay.get_pixel(x, y);
    let a = p[3];
    if a == 0 || a == 255 {
        return false;
    }
    let luma =
        (p[0] as u32 * 2126 + p[1] as u32 * 7152 + p[2] as u32 * 722) / 10_000;
    luma <= OVERLAY_SEMITRANSPARENT_LUMA_MAX as u32
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
