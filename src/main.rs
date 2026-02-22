mod background;
mod capture;
mod cli;
mod color;
mod compose;
mod config;
mod devices;
mod frames;
mod pipeline;
mod preview;

use anyhow::{Result, bail};
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
        Commands::Devices => {
            println!("Built-in phone models:");
            for device in &devices::DEVICE_LISTINGS {
                println!("  - {} ({})", device.slug, device.display_name);
            }
        }
        Commands::ImportFrames {
            source,
            dest,
            overwrite,
        } => {
            let summary = frames::import_frames(&source, &dest, overwrite)?;
            println!("Imported frames from {}", summary.source.display());
            println!("Destination: {}", summary.destination.display());
            println!("Imported: {}", summary.imported);
            println!("Skipped: {}", summary.skipped);
            for line in summary.notes {
                println!("  - {}", line);
            }
        }
        Commands::VerifyOverlay { config, strict } => {
            let summary = frames::verify_overlays(&config)?;
            println!(
                "Overlay checks: {} scene(s), {} overlay candidate(s), {} warning(s), {} error(s)",
                summary.scene_count, summary.checked_overlays, summary.warnings, summary.errors
            );
            for issue in &summary.issues {
                println!(
                    "  [{}] {}: {}",
                    issue.level.label(),
                    issue.scene_id,
                    issue.message
                );
            }
            if summary.failed(strict) {
                if strict && summary.errors == 0 && summary.warnings > 0 {
                    bail!(
                        "overlay verification failed in strict mode (warnings treated as failures)"
                    );
                }
                bail!("overlay verification failed");
            }
        }
    }

    Ok(())
}
