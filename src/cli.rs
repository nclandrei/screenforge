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
}
