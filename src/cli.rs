use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "screenforge",
    version,
    about = "Generate App Store-style marketing screenshots from config"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run full pipeline: capture -> background -> compose -> preview
    Run {
        /// Path to YAML config
        #[arg(short, long, default_value = "screenforge.yaml")]
        config: PathBuf,
    },
    /// List built-in phone model presets
    Devices,
    /// Import transparent PNG frame overlays into assets/frames
    ImportFrames {
        /// Source directory containing PNG frame files
        #[arg(short, long)]
        source: PathBuf,
        /// Destination directory for normalized overlays
        #[arg(long, default_value = "assets/frames")]
        dest: PathBuf,
        /// Overwrite destination overlays if they already exist
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    /// Validate overlay files referenced by config scenes
    VerifyOverlay {
        /// Path to YAML config
        #[arg(short, long, default_value = "screenforge.yaml")]
        config: PathBuf,
        /// Treat warnings as failures
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
    /// Capture and frame a screenshot from a running iOS simulator
    ///
    /// Takes a screenshot from any booted simulator, auto-detects the device
    /// model, and renders a framed App Store-style image.
    ///
    /// Examples:
    ///   screenforge snap "iPhone 16 Pro"
    ///   screenforge snap "My-Custom-Simulator" --output hero.png
    ///   screenforge snap 864E85BD-BAAF-4BB3-9D02-3D9FD0C34D4A --raw
    ///   screenforge snap --list
    #[command(verbatim_doc_comment)]
    Snap {
        /// Simulator name, partial name, or UDID. Omit to list booted simulators.
        #[arg(value_name = "SIMULATOR")]
        simulator: Option<String>,

        /// Output file path
        #[arg(short, long, default_value = "snap_output.png")]
        output: PathBuf,

        /// Capture raw screenshot without framing
        #[arg(long, default_value_t = false)]
        raw: bool,

        /// List all booted simulators and exit
        #[arg(short, long, default_value_t = false)]
        list: bool,

        /// Output format (text or json for agent consumption)
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,

        /// Override auto-detected phone model
        #[arg(long, value_enum)]
        model: Option<PhoneModelArg>,

        /// Wait time (ms) before capturing to let UI settle
        #[arg(long, default_value_t = 500)]
        settle_ms: u64,

        /// Output canvas width
        #[arg(long, default_value_t = 1290)]
        width: u32,

        /// Output canvas height
        #[arg(long, default_value_t = 2796)]
        height: u32,

        /// Headline text to render above phone
        #[arg(long)]
        headline: Option<String>,

        /// Subheadline text
        #[arg(long)]
        subheadline: Option<String>,

        /// Background template
        #[arg(long, value_enum, default_value_t = BackgroundTemplateArg::Mesh)]
        background: BackgroundTemplateArg,

        /// Background seed for deterministic generation
        #[arg(long, default_value_t = 42)]
        seed: u64,

        /// Background colors (comma-separated hex colors)
        #[arg(long, value_delimiter = ',')]
        colors: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PhoneModelArg {
    Iphone16Pro,
    Iphone16ProMax,
    Iphone17Pro,
    Iphone17ProMax,
}

impl From<PhoneModelArg> for crate::config::PhoneModel {
    fn from(arg: PhoneModelArg) -> Self {
        match arg {
            PhoneModelArg::Iphone16Pro => Self::Iphone16Pro,
            PhoneModelArg::Iphone16ProMax => Self::Iphone16ProMax,
            PhoneModelArg::Iphone17Pro => Self::Iphone17Pro,
            PhoneModelArg::Iphone17ProMax => Self::Iphone17ProMax,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BackgroundTemplateArg {
    Mesh,
    Stripes,
}

impl From<BackgroundTemplateArg> for crate::config::BackgroundTemplate {
    fn from(arg: BackgroundTemplateArg) -> Self {
        match arg {
            BackgroundTemplateArg::Mesh => Self::Mesh,
            BackgroundTemplateArg::Stripes => Self::Stripes,
        }
    }
}
