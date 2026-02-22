use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::background::render_background;
use crate::capture::capture_scene;
use crate::compose::compose_scene;
use crate::config::Config;
use crate::preview::{PreviewItem, write_index};

pub struct RunSummary {
    pub scene_count: usize,
    pub output_dir: PathBuf,
    pub preview_path: PathBuf,
}

pub fn run(config_path: &Path) -> Result<RunSummary> {
    let config = Config::from_path(config_path)?;
    if config.scenes.is_empty() {
        bail!("config has no scenes");
    }

    let config_dir = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let output_root = resolve_path(&config_dir, &config.output_dir);
    let raw_dir = output_root.join("raw");
    let final_dir = output_root.join("final");
    let preview_path = output_root.join("index.html");

    fs::create_dir_all(&raw_dir)
        .with_context(|| format!("failed creating {}", raw_dir.display()))?;
    fs::create_dir_all(&final_dir)
        .with_context(|| format!("failed creating {}", final_dir.display()))?;

    let mut seen_ids = HashSet::new();
    let mut preview_items = Vec::with_capacity(config.scenes.len());

    for scene in &config.scenes {
        if !seen_ids.insert(scene.id.clone()) {
            bail!("duplicate scene id '{}'", scene.id);
        }

        let raw_path = raw_dir.join(format!("{}.png", scene.id));
        capture_scene(scene, &config_dir, &raw_path)?;

        let raw_img = image::open(&raw_path)
            .with_context(|| format!("failed opening raw screenshot {}", raw_path.display()))?;
        let background =
            render_background(&scene.background, scene.output.width, scene.output.height)?;
        let final_img = compose_scene(&raw_img, scene, background)?;

        let final_path = final_dir.join(&scene.output.filename);
        final_img
            .save(&final_path)
            .with_context(|| format!("failed writing {}", final_path.display()))?;

        preview_items.push(PreviewItem {
            scene_id: scene.id.clone(),
            raw_rel: format!("raw/{}.png", scene.id),
            final_rel: format!("final/{}", scene.output.filename),
        });
    }

    write_index(&preview_path, &preview_items)?;

    Ok(RunSummary {
        scene_count: preview_items.len(),
        output_dir: output_root,
        preview_path,
    })
}

fn resolve_path(config_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_dir.join(path)
    }
}
