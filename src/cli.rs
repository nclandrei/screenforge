use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
}
