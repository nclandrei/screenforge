mod background;
mod capture;
mod cli;
mod color;
mod compose;
mod config;
mod pipeline;
mod preview;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run { config } => {
            let summary = pipeline::run(&config)?;
            println!(
                "Rendered {} scene(s) into {}",
                summary.scene_count,
                summary.output_dir.display()
            );
            println!("Preview: {}", summary.preview_path.display());
        }
    }

    Ok(())
}
