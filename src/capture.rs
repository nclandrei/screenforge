use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::config::{CaptureConfig, SceneConfig};

pub fn capture_scene(scene: &SceneConfig, config_dir: &Path, raw_path: &Path) -> Result<()> {
    if let Some(parent) = raw_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }

    match &scene.capture {
        CaptureConfig::File { path } => {
            let source_path = resolve_path(config_dir, path);
            let source_img = image::open(&source_path).with_context(|| {
                format!(
                    "scene '{}' failed to open source image {}",
                    scene.id,
                    source_path.display()
                )
            })?;

            source_img.save(raw_path).with_context(|| {
                format!(
                    "scene '{}' failed to save normalized raw image {}",
                    scene.id,
                    raw_path.display()
                )
            })?;
            Ok(())
        }
        CaptureConfig::Simctl { device, settle_ms } => {
            if *settle_ms > 0 {
                thread::sleep(Duration::from_millis(*settle_ms));
            }

            let status = Command::new("xcrun")
                .args(["simctl", "io", device, "screenshot"])
                .arg(raw_path)
                .status()
                .with_context(|| "failed to execute xcrun simctl")?;

            if !status.success() {
                bail!(
                    "scene '{}' simctl screenshot failed for device '{}'",
                    scene.id,
                    device
                );
            }

            Ok(())
        }
    }
}

fn resolve_path(config_dir: &Path, path: &Path) -> std::path::PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_dir.join(path)
    }
}
