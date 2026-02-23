mod background;
mod capture;
mod cli;
mod color;
mod compose;
mod config;
mod devices;
mod frames;
mod palette;
mod pipeline;
mod preview;
mod simulator;
mod snap;

use anyhow::{Result, bail};
use clap::Parser;

use crate::cli::{Cli, Commands, OutputFormat};
use crate::snap::SnapConfig;

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
        Commands::Snap {
            simulator,
            output,
            raw,
            list,
            format,
            model,
            settle_ms,
            width,
            height,
            headline,
            subheadline,
            background,
            seed,
            colors,
        } => {
            // Handle --list flag
            if list {
                let booted = snap::list_booted()?;
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&booted)?);
                    }
                    OutputFormat::Text => {
                        if booted.is_empty() {
                            println!("No simulators are currently booted.");
                            println!("\nBoot a simulator with:");
                            println!("  xcrun simctl boot \"iPhone 16 Pro\"");
                        } else {
                            println!("Booted simulators:");
                            for sim in &booted {
                                let model_info = sim
                                    .phone_model
                                    .as_ref()
                                    .map(|m| format!(" [{}]", m))
                                    .unwrap_or_default();
                                println!("  {} ({}){}", sim.name, sim.udid, model_info);
                            }
                        }
                    }
                }
                return Ok(());
            }

            // Require simulator argument if not listing
            let query = match simulator {
                Some(q) => q,
                None => {
                    // Default to listing booted simulators when no argument given
                    let booted = snap::list_booted()?;
                    match format {
                        OutputFormat::Json => {
                            println!("{}", serde_json::to_string_pretty(&booted)?);
                        }
                        OutputFormat::Text => {
                            if booted.is_empty() {
                                println!("No simulators are currently booted.");
                                println!("\nUsage: screenforge snap <SIMULATOR> [--output <PATH>]");
                                println!("\nBoot a simulator first:");
                                println!("  xcrun simctl boot \"iPhone 16 Pro\"");
                            } else {
                                println!("Booted simulators:");
                                for sim in &booted {
                                    let model_info = sim
                                        .phone_model
                                        .as_ref()
                                        .map(|m| format!(" [{}]", m))
                                        .unwrap_or_default();
                                    println!("  {} ({}){}", sim.name, sim.udid, model_info);
                                }
                                println!("\nUsage: screenforge snap <SIMULATOR> [--output <PATH>]");
                            }
                        }
                    }
                    return Ok(());
                }
            };

            // Execute snap
            let result = if raw {
                snap::snap_raw(&query, &output, settle_ms)?
            } else {
                let config = SnapConfig {
                    width,
                    height,
                    phone_x: None,
                    phone_y: None,
                    phone_width: None,
                    phone_height: None,
                    background_template: background.into(),
                    background_seed: seed,
                    background_colors: colors.unwrap_or_else(|| {
                        vec![
                            "#0B1022".to_string(),
                            "#16479A".to_string(),
                            "#2B8CD6".to_string(),
                            "#A9E7FF".to_string(),
                        ]
                    }),
                    headline,
                    subheadline,
                    settle_ms,
                    overlay: None,
                };
                snap::snap_framed(&query, &output, &config, model.map(Into::into))?
            };

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                OutputFormat::Text => {
                    println!("✓ Captured: {}", result.simulator_name);
                    if let Some(model) = &result.device_model {
                        println!("  Model: {}", model);
                    }
                    println!(
                        "  Output: {} ({}x{})",
                        result.output_path, result.dimensions.width, result.dimensions.height
                    );
                }
            }
        }
    }

    Ok(())
}
