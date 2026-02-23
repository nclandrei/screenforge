use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::background::render_background;
use crate::compose::compose_scene;
use crate::config::{
    BackgroundConfig, BackgroundTemplate, CaptureConfig, CopyConfig, Insets, OutputConfig,
    PhoneConfig, PhoneModel, SceneConfig,
};
use crate::simulator::{find_booted_simulators, find_simulator};

/// Configuration for a snap operation, loaded from YAML preset or CLI flags
#[derive(Debug, Clone)]
pub struct SnapConfig {
    /// Output dimensions (default: App Store 6.7" = 1290x2796)
    pub width: u32,
    pub height: u32,

    /// Phone positioning (auto-calculated if not set)
    pub phone_x: Option<u32>,
    pub phone_y: Option<u32>,
    pub phone_width: Option<u32>,
    pub phone_height: Option<u32>,

    /// Background settings
    pub background_template: BackgroundTemplate,
    pub background_seed: u64,
    pub background_colors: Vec<String>,

    /// Optional copy/text
    pub headline: Option<String>,
    pub subheadline: Option<String>,

    /// Settle time before capture (ms)
    pub settle_ms: u64,

    /// Frame overlay path (optional)
    pub overlay: Option<PathBuf>,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            // App Store 6.7" display size
            width: 1290,
            height: 2796,
            phone_x: None,
            phone_y: None,
            phone_width: None,
            phone_height: None,
            background_template: BackgroundTemplate::Mesh,
            background_seed: 42,
            background_colors: vec![
                "#0B1022".to_string(),
                "#16479A".to_string(),
                "#2B8CD6".to_string(),
                "#A9E7FF".to_string(),
            ],
            headline: None,
            subheadline: None,
            settle_ms: 500,
            overlay: None,
        }
    }
}

/// Result of a snap operation, suitable for JSON output
#[derive(Debug, Serialize)]
pub struct SnapResult {
    pub success: bool,
    pub simulator_name: String,
    pub simulator_udid: String,
    pub device_model: Option<String>,
    pub output_path: String,
    pub raw_path: Option<String>,
    pub dimensions: Dimensions,
}

#[derive(Debug, Serialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

/// Take a raw screenshot from a simulator without framing
pub fn snap_raw(query: &str, output_path: &Path, settle_ms: u64) -> Result<SnapResult> {
    let simulator = find_simulator(query)?;

    if !simulator.is_booted() {
        bail!(
            "simulator '{}' is not booted (state: {}). Boot it first with:\n  xcrun simctl boot '{}'",
            simulator.name,
            simulator.state,
            simulator.udid
        );
    }

    // Settle time
    if settle_ms > 0 {
        thread::sleep(Duration::from_millis(settle_ms));
    }

    // Take screenshot (suppress simctl debug output)
    let output = Command::new("xcrun")
        .args(["simctl", "io", &simulator.udid, "screenshot"])
        .arg(output_path)
        .output()
        .context("failed to execute xcrun simctl")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "simctl screenshot failed for simulator '{}': {}",
            simulator.name,
            stderr.trim()
        );
    }

    // Get image dimensions
    let img = image::open(output_path)
        .with_context(|| format!("failed to open screenshot {}", output_path.display()))?;

    Ok(SnapResult {
        success: true,
        simulator_name: simulator.name,
        simulator_udid: simulator.udid,
        device_model: simulator.phone_model.map(|m| format!("{:?}", m)),
        output_path: output_path.to_string_lossy().to_string(),
        raw_path: None,
        dimensions: Dimensions {
            width: img.width(),
            height: img.height(),
        },
    })
}

/// Take a screenshot and frame it with device chrome
pub fn snap_framed(
    query: &str,
    output_path: &Path,
    config: &SnapConfig,
    model_override: Option<PhoneModel>,
) -> Result<SnapResult> {
    let simulator = find_simulator(query)?;

    if !simulator.is_booted() {
        bail!(
            "simulator '{}' is not booted (state: {}). Boot it first with:\n  xcrun simctl boot '{}'",
            simulator.name,
            simulator.state,
            simulator.udid
        );
    }

    // Determine phone model
    let phone_model = model_override.or(simulator.phone_model);

    // Create temp file for raw screenshot
    let raw_path = std::env::temp_dir().join(format!("screenforge_snap_{}.png", simulator.udid));

    // Settle time
    if config.settle_ms > 0 {
        thread::sleep(Duration::from_millis(config.settle_ms));
    }

    // Take screenshot (suppress simctl debug output)
    let output = Command::new("xcrun")
        .args(["simctl", "io", &simulator.udid, "screenshot"])
        .arg(&raw_path)
        .output()
        .context("failed to execute xcrun simctl")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "simctl screenshot failed for simulator '{}': {}",
            simulator.name,
            stderr.trim()
        );
    }

    // Load raw screenshot
    let raw_img = image::open(&raw_path)
        .with_context(|| format!("failed to open raw screenshot {}", raw_path.display()))?;

    // Calculate phone dimensions based on output size
    let (phone_width, phone_height, phone_x, phone_y) =
        calculate_phone_layout(config, &raw_img);

    // Build scene config for compose
    let scene = SceneConfig {
        id: "snap".to_string(),
        capture: CaptureConfig::File {
            path: raw_path.clone(),
        },
        output: OutputConfig {
            filename: output_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            width: config.width,
            height: config.height,
        },
        background: BackgroundConfig {
            template: config.background_template,
            seed: config.background_seed,
            colors: config.background_colors.clone(),
        },
        phone: PhoneConfig {
            model: phone_model,
            x: phone_x,
            y: phone_y,
            width: phone_width,
            height: phone_height,
            corner_radius: 88,
            screen_padding: Insets::default(),
            frame_color: "#11151B".to_string(),
            frame_border_width: 8,
            shadow_offset_y: 18,
            shadow_alpha: 74,
            overlay: config.overlay.clone(),
        },
        copy: build_copy_config(config),
    };

    // Render background
    let background = render_background(&scene.background, config.width, config.height)?;

    // Compose final image
    let final_img = compose_scene(&raw_img, &scene, background, Path::new("."))?;

    // Save output
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }

    final_img
        .save(output_path)
        .with_context(|| format!("failed to save output {}", output_path.display()))?;

    Ok(SnapResult {
        success: true,
        simulator_name: simulator.name,
        simulator_udid: simulator.udid,
        device_model: phone_model.map(|m| format!("{:?}", m)),
        output_path: output_path.to_string_lossy().to_string(),
        raw_path: Some(raw_path.to_string_lossy().to_string()),
        dimensions: Dimensions {
            width: config.width,
            height: config.height,
        },
    })
}

/// List all booted simulators (for agent discovery)
#[derive(Debug, Serialize)]
pub struct SimulatorInfo {
    pub name: String,
    pub udid: String,
    pub device_type: String,
    pub phone_model: Option<String>,
    pub runtime: String,
}

pub fn list_booted() -> Result<Vec<SimulatorInfo>> {
    let simulators = find_booted_simulators()?;

    Ok(simulators
        .into_iter()
        .map(|s| SimulatorInfo {
            name: s.name,
            udid: s.udid,
            device_type: s.device_type,
            phone_model: s.phone_model.map(|m| format!("{:?}", m)),
            runtime: s.runtime,
        })
        .collect())
}

/// Calculate phone layout to fit nicely in the output canvas
fn calculate_phone_layout(
    config: &SnapConfig,
    raw_img: &image::DynamicImage,
) -> (u32, u32, u32, u32) {
    // Use explicit config if provided
    if let (Some(w), Some(h), Some(x), Some(y)) = (
        config.phone_width,
        config.phone_height,
        config.phone_x,
        config.phone_y,
    ) {
        return (w, h, x, y);
    }

    let output_w = config.width;
    let output_h = config.height;
    let raw_w = raw_img.width();
    let raw_h = raw_img.height();

    // Calculate phone size to fill ~73% of output width, maintaining aspect ratio
    let target_phone_width = (output_w as f32 * 0.73) as u32;
    let aspect_ratio = raw_h as f32 / raw_w as f32;
    let target_phone_height = (target_phone_width as f32 * aspect_ratio) as u32;

    // Center horizontally
    let phone_x = (output_w - target_phone_width) / 2;

    // Position in lower portion of canvas (leave room for headline)
    let phone_y = if config.headline.is_some() {
        // Leave top 15% for text
        (output_h as f32 * 0.15) as u32
    } else {
        // Center vertically with slight offset down
        (output_h - target_phone_height) / 2 + (output_h as f32 * 0.05) as u32
    };

    (
        config.phone_width.unwrap_or(target_phone_width),
        config.phone_height.unwrap_or(target_phone_height),
        config.phone_x.unwrap_or(phone_x),
        config.phone_y.unwrap_or(phone_y),
    )
}

fn build_copy_config(config: &SnapConfig) -> Option<CopyConfig> {
    config.headline.as_ref().map(|headline| CopyConfig {
        headline: headline.clone(),
        subheadline: config.subheadline.clone().unwrap_or_default(),
        color: "#F4F8FF".to_string(),
        x: 84,
        y: 98,
        headline_scale: 6,
        subheadline_scale: 3,
        line_gap: 16,
    })
}
