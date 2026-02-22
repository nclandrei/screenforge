use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{Config, PhoneModel, SceneConfig};

const DEFAULT_FRAMES_DIR: &str = "assets/frames";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySource {
    Explicit,
    ModelDefault,
}

impl OverlaySource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::ModelDefault => "model_default",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedOverlay {
    pub path: PathBuf,
    pub source: OverlaySource,
}

#[derive(Debug)]
pub struct VerifyIssue {
    pub scene_id: String,
    pub level: VerifyLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyLevel {
    Warning,
    Error,
}

impl VerifyLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }
}

#[derive(Debug)]
pub struct VerifySummary {
    pub scene_count: usize,
    pub checked_overlays: usize,
    pub warnings: usize,
    pub errors: usize,
    pub issues: Vec<VerifyIssue>,
}

impl VerifySummary {
    pub fn failed(&self, strict: bool) -> bool {
        self.errors > 0 || (strict && self.warnings > 0)
    }
}

#[derive(Debug)]
pub struct ImportSummary {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub imported: usize,
    pub skipped: usize,
    pub notes: Vec<String>,
}

pub fn import_frames(source: &Path, destination: &Path, overwrite: bool) -> Result<ImportSummary> {
    let source_metadata = fs::metadata(source)
        .with_context(|| format!("failed reading source {}", source.display()))?;
    if !source_metadata.is_dir() {
        anyhow::bail!("source is not a directory: {}", source.display());
    }

    fs::create_dir_all(destination)
        .with_context(|| format!("failed creating {}", destination.display()))?;

    let mut entries = fs::read_dir(source)
        .with_context(|| format!("failed reading source {}", source.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed listing files in {}", source.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut notes = Vec::new();

    for entry in entries {
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed reading file type for {}", entry.path().display()))?;
        if !file_type.is_file() {
            continue;
        }

        let src_path = entry.path();
        if !is_png_file(&src_path) {
            skipped += 1;
            notes.push(format!(
                "skip {}: only .png files are supported",
                src_path.display()
            ));
            continue;
        }

        let stem = src_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let slug = normalize_frame_slug(stem);
        if slug.is_empty() {
            skipped += 1;
            notes.push(format!(
                "skip {}: filename cannot be normalized to a slug",
                src_path.display()
            ));
            continue;
        }

        match read_overlay_meta(&src_path) {
            Ok(meta) => {
                if !meta.has_transparency {
                    skipped += 1;
                    notes.push(format!(
                        "skip {}: overlay has no transparency (frame must include transparent cutout)",
                        src_path.display()
                    ));
                    continue;
                }
            }
            Err(err) => {
                skipped += 1;
                notes.push(format!("skip {}: {}", src_path.display(), err));
                continue;
            }
        }

        let dest_path = destination.join(format!("{}.png", slug));
        if dest_path.exists() && !overwrite {
            skipped += 1;
            notes.push(format!(
                "skip {}: destination exists (use --overwrite to replace)",
                dest_path.display()
            ));
            continue;
        }

        fs::copy(&src_path, &dest_path).with_context(|| {
            format!(
                "failed copying {} -> {}",
                src_path.display(),
                dest_path.display()
            )
        })?;
        imported += 1;
        notes.push(format!(
            "import {} -> {}",
            src_path.display(),
            dest_path.display()
        ));
    }

    Ok(ImportSummary {
        source: source.to_path_buf(),
        destination: destination.to_path_buf(),
        imported,
        skipped,
        notes,
    })
}

pub fn verify_overlays(config_path: &Path) -> Result<VerifySummary> {
    let config = Config::from_path(config_path)?;
    let config_dir = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut summary = VerifySummary {
        scene_count: config.scenes.len(),
        checked_overlays: 0,
        warnings: 0,
        errors: 0,
        issues: Vec::new(),
    };

    for scene in &config.scenes {
        let Some(overlay) = resolve_overlay_for_verify(scene, &config_dir) else {
            continue;
        };
        summary.checked_overlays += 1;

        if !overlay.path.exists() {
            match overlay.source {
                OverlaySource::Explicit => {
                    push_issue(
                        &mut summary,
                        scene.id.clone(),
                        VerifyLevel::Error,
                        format!("overlay not found: {}", overlay.path.display()),
                    );
                }
                OverlaySource::ModelDefault => {
                    push_issue(
                        &mut summary,
                        scene.id.clone(),
                        VerifyLevel::Warning,
                        format!("no auto overlay for model at {}", overlay.path.display()),
                    );
                }
            }
            continue;
        }

        if !is_png_file(&overlay.path) {
            push_issue(
                &mut summary,
                scene.id.clone(),
                VerifyLevel::Warning,
                format!("overlay should be a PNG: {}", overlay.path.display()),
            );
        }

        match read_overlay_meta(&overlay.path) {
            Ok(meta) => {
                if !meta.has_transparency {
                    push_issue(
                        &mut summary,
                        scene.id.clone(),
                        VerifyLevel::Error,
                        format!("overlay has no transparency: {}", overlay.path.display()),
                    );
                }

                if meta.width != scene.phone.width || meta.height != scene.phone.height {
                    push_issue(
                        &mut summary,
                        scene.id.clone(),
                        VerifyLevel::Warning,
                        format!(
                            "overlay size {}x{} does not match phone rect {}x{} ({}).",
                            meta.width,
                            meta.height,
                            scene.phone.width,
                            scene.phone.height,
                            overlay.path.display()
                        ),
                    );
                }
            }
            Err(err) => {
                push_issue(
                    &mut summary,
                    scene.id.clone(),
                    VerifyLevel::Error,
                    format!("failed reading overlay {}: {}", overlay.path.display(), err),
                );
            }
        }
    }

    Ok(summary)
}

pub fn resolve_overlay_for_compose(
    scene: &SceneConfig,
    config_dir: &Path,
) -> Option<ResolvedOverlay> {
    if let Some(overlay) = scene.phone.overlay.as_ref().map(|path| ResolvedOverlay {
        path: resolve_path(config_dir, path),
        source: OverlaySource::Explicit,
    }) {
        return Some(overlay);
    }

    let model = scene.phone.model?;
    let path = default_model_overlay_path(config_dir, model);
    if path.exists() {
        Some(ResolvedOverlay {
            path,
            source: OverlaySource::ModelDefault,
        })
    } else {
        None
    }
}

pub fn resolve_overlay_for_verify(
    scene: &SceneConfig,
    config_dir: &Path,
) -> Option<ResolvedOverlay> {
    if let Some(overlay) = scene.phone.overlay.as_ref().map(|path| ResolvedOverlay {
        path: resolve_path(config_dir, path),
        source: OverlaySource::Explicit,
    }) {
        return Some(overlay);
    }

    let model = scene.phone.model?;
    Some(ResolvedOverlay {
        path: default_model_overlay_path(config_dir, model),
        source: OverlaySource::ModelDefault,
    })
}

pub fn model_slug(model: PhoneModel) -> &'static str {
    match model {
        PhoneModel::Iphone16Pro => "iphone_16_pro",
        PhoneModel::Iphone17Pro => "iphone_17_pro",
    }
}

fn default_model_overlay_path(config_dir: &Path, model: PhoneModel) -> PathBuf {
    config_dir
        .join(DEFAULT_FRAMES_DIR)
        .join(format!("{}.png", model_slug(model)))
}

fn resolve_path(config_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_dir.join(path)
    }
}

fn push_issue(summary: &mut VerifySummary, scene_id: String, level: VerifyLevel, message: String) {
    match level {
        VerifyLevel::Warning => summary.warnings += 1,
        VerifyLevel::Error => summary.errors += 1,
    }
    summary.issues.push(VerifyIssue {
        scene_id,
        level,
        message,
    });
}

#[derive(Debug)]
struct OverlayMeta {
    width: u32,
    height: u32,
    has_transparency: bool,
}

fn read_overlay_meta(path: &Path) -> Result<OverlayMeta> {
    let image =
        image::open(path).with_context(|| format!("failed to decode {}", path.display()))?;
    let rgba = image.to_rgba8();
    let has_transparency = rgba.pixels().any(|pixel| pixel[3] < 255);
    Ok(OverlayMeta {
        width: rgba.width(),
        height: rgba.height(),
        has_transparency,
    })
}

fn is_png_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("png"))
        .unwrap_or(false)
}

fn normalize_frame_slug(stem: &str) -> String {
    let mut out = String::new();
    let mut previous_was_sep = false;

    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            previous_was_sep = false;
        } else if !previous_was_sep {
            out.push('_');
            previous_was_sep = true;
        }
    }

    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    #[test]
    fn import_frames_only_accepts_transparent_pngs() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).expect("create source");

        write_png(&source.join("iPhone 16 Pro.png"), 20, 30, true);
        write_png(&source.join("Opaque.png"), 20, 30, false);
        fs::write(source.join("readme.txt"), "x").expect("write txt");

        let summary = import_frames(&source, &destination, false).expect("import frames");
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.skipped, 2);
        assert!(destination.join("iphone_16_pro.png").exists());
        assert!(!destination.join("opaque.png").exists());
    }

    #[test]
    fn verify_overlays_reports_missing_explicit_overlay() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("screenforge.yaml");
        fs::write(
            &config_path,
            format!(
                r#"
output_dir: ./output
scenes:
  - id: missing_explicit
    capture:
      adapter: file
      path: ./raw.png
    output:
      filename: out.png
      width: 1290
      height: 2796
    background: {{}}
    phone:
      x: 10
      y: 10
      width: 100
      height: 200
      overlay: {}
"#,
                temp.path().join("missing.png").display()
            ),
        )
        .expect("write config");

        let summary = verify_overlays(&config_path).expect("verify");
        assert_eq!(summary.checked_overlays, 1);
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.warnings, 0);
        assert!(summary.failed(false));
    }

    #[test]
    fn verify_overlays_warns_when_model_overlay_is_missing() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("screenforge.yaml");
        fs::write(
            &config_path,
            r#"
output_dir: ./output
scenes:
  - id: model_missing
    capture:
      adapter: file
      path: ./raw.png
    output:
      filename: out.png
      width: 1290
      height: 2796
    background: {}
    phone:
      model: iphone_16_pro
      x: 10
      y: 10
      width: 100
      height: 200
"#,
        )
        .expect("write config");

        let summary = verify_overlays(&config_path).expect("verify");
        assert_eq!(summary.checked_overlays, 1);
        assert_eq!(summary.errors, 0);
        assert_eq!(summary.warnings, 1);
        assert!(!summary.failed(false));
        assert!(summary.failed(true));
    }

    #[test]
    fn verify_overlays_warns_on_dimension_mismatch() {
        let temp = tempdir().expect("tempdir");
        let frames_dir = temp.path().join("assets/frames");
        fs::create_dir_all(&frames_dir).expect("frames dir");
        write_png(&frames_dir.join("iphone_17_pro.png"), 300, 600, true);

        let config_path = temp.path().join("screenforge.yaml");
        fs::write(
            &config_path,
            r#"
output_dir: ./output
scenes:
  - id: size_mismatch
    capture:
      adapter: file
      path: ./raw.png
    output:
      filename: out.png
      width: 1290
      height: 2796
    background: {}
    phone:
      model: iphone_17_pro
      x: 10
      y: 10
      width: 100
      height: 200
"#,
        )
        .expect("write config");

        let summary = verify_overlays(&config_path).expect("verify");
        assert_eq!(summary.checked_overlays, 1);
        assert_eq!(summary.errors, 0);
        assert_eq!(summary.warnings, 1);
        assert!(
            summary
                .issues
                .iter()
                .any(|issue| issue.message.contains("does not match"))
        );
    }

    fn write_png(path: &Path, width: u32, height: u32, transparent: bool) {
        let mut image = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                image.put_pixel(x, y, Rgba([60, 80, 120, 255]));
            }
        }
        if transparent {
            let max_x = width.min(10);
            let max_y = height.min(10);
            for y in 0..max_y {
                for x in 0..max_x {
                    image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            }
        }
        image.save(path).expect("save png");
    }
}
