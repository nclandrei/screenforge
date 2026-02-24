use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    pub scenes: Vec<SceneConfig>,
}

impl Config {
    pub fn from_path(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let parsed: Self = serde_yaml::from_str(&raw)
            .with_context(|| format!("failed to parse yaml: {}", path.display()))?;
        Ok(parsed)
    }
}

#[derive(Debug, Deserialize)]
pub struct SceneConfig {
    pub id: String,
    pub capture: CaptureConfig,
    pub output: OutputConfig,
    pub background: BackgroundConfig,
    pub phone: PhoneConfig,
    #[serde(default)]
    pub copy: Option<CopyConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub enum PhoneModel {
    #[serde(rename = "iphone_16_pro")]
    Iphone16Pro,
    #[serde(rename = "iphone_16_pro_max")]
    Iphone16ProMax,
    #[serde(rename = "iphone_17_pro")]
    Iphone17Pro,
    #[serde(rename = "iphone_17_pro_max")]
    Iphone17ProMax,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "adapter", rename_all = "snake_case")]
pub enum CaptureConfig {
    File {
        path: PathBuf,
    },
    Simctl {
        device: String,
        #[serde(default = "default_settle_ms")]
        settle_ms: u64,
    },
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub filename: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundConfig {
    #[serde(default)]
    pub template: BackgroundTemplate,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_palette")]
    pub colors: Vec<String>,
    /// When true, automatically extract colors from the screenshot
    #[serde(default)]
    pub auto_colors: bool,
    /// Strategy for generating palette from extracted colors
    #[serde(default)]
    pub auto_strategy: AutoColorStrategy,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutoColorStrategy {
    /// Darker/lighter variations of dominant color
    #[default]
    Monochromatic,
    /// Colors adjacent on the color wheel
    Analogous,
    /// Opposite on color wheel for contrast
    Complementary,
    /// Three colors equally spaced
    Triadic,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundTemplate {
    #[default]
    Mesh,
    Stripes,
}

#[derive(Debug, Deserialize)]
pub struct PhoneConfig {
    #[serde(default)]
    pub model: Option<PhoneModel>,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: u32,
    #[serde(default)]
    pub screen_padding: Insets,
    #[serde(default = "default_frame_color")]
    pub frame_color: String,
    #[serde(default = "default_frame_border_width")]
    pub frame_border_width: u32,
    #[serde(default = "default_shadow_offset_y")]
    pub shadow_offset_y: i32,
    #[serde(default = "default_shadow_alpha")]
    pub shadow_alpha: u8,
    #[serde(default)]
    pub overlay: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Insets {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Default for Insets {
    fn default() -> Self {
        Self {
            top: 28,
            right: 20,
            bottom: 28,
            left: 20,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum FontWeight {
    Regular,
    Medium,
    #[default]
    SemiBold,
    Bold,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextPosition {
    /// Text centered above the phone mockup
    #[default]
    AbovePhone,
    /// Text centered below the phone mockup
    BelowPhone,
    /// Text at top of canvas (with padding)
    Top,
    /// Text at bottom of canvas (with padding)
    Bottom,
}

#[derive(Debug, Deserialize)]
pub struct CopyConfig {
    pub headline: String,
    #[serde(default)]
    pub subheadline: String,
    #[serde(default = "default_copy_color")]
    pub color: String,
    /// Vertical position preset (default: above_phone)
    #[serde(default)]
    pub position: TextPosition,
    /// Vertical offset adjustment in pixels (positive = down, negative = up)
    #[serde(default)]
    pub y_offset: i32,
    /// Headline font size in pixels (default: 72)
    #[serde(default = "default_headline_size")]
    pub headline_size: f32,
    /// Subheadline font size in pixels (default: 36)
    #[serde(default = "default_subheadline_size")]
    pub subheadline_size: f32,
    /// Font weight for headline (default: bold)
    #[serde(default)]
    pub headline_weight: FontWeight,
    /// Font weight for subheadline (default: regular)
    #[serde(default = "default_subheadline_weight")]
    pub subheadline_weight: FontWeight,
    #[serde(default = "default_line_gap")]
    pub line_gap: u32,
    /// Maximum width for text wrapping (default: auto based on image width)
    #[serde(default)]
    pub max_width: Option<u32>,
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("./output")
}

fn default_seed() -> u64 {
    1
}

fn default_palette() -> Vec<String> {
    vec![
        "#0E1228".to_string(),
        "#1348A5".to_string(),
        "#2B8CD6".to_string(),
        "#C2E6FF".to_string(),
    ]
}

fn default_settle_ms() -> u64 {
    800
}

fn default_corner_radius() -> u32 {
    88
}

fn default_frame_color() -> String {
    "#11151B".to_string()
}

fn default_frame_border_width() -> u32 {
    8
}

fn default_shadow_offset_y() -> i32 {
    18
}

fn default_shadow_alpha() -> u8 {
    74
}

fn default_copy_color() -> String {
    "#F4F8FF".to_string()
}

fn default_headline_size() -> f32 {
    120.0
}

fn default_subheadline_size() -> f32 {
    56.0
}

fn default_subheadline_weight() -> FontWeight {
    FontWeight::Regular
}

fn default_line_gap() -> u32 {
    24
}
