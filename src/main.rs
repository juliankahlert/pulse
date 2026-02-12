//! Pulse: A fast, configurable Rust PS1 prompt engine for modern shells.
//!
//! This application generates shell prompts with support for Git repositories,
//! customizable colors, and different display modes.

use anyhow::Result;
use clap::Parser;
use log::error;

mod cli;
mod clrs;
mod config;
mod install;
mod prompt;

/// Main entry point for the Pulse application.
///
/// Initializes logging, parses command-line arguments, loads configuration,
/// generates the prompt, and prints it to stdout.
fn main() -> Result<()> {
    env_logger::init();
    let args = cli::Args::parse();

    if args.install {
        return install::install().map_err(|e| {
            error!("Failed to install: {}", e);
            e
        });
    }

    let mut config = config::Config::load().map_err(|e| {
        error!("Failed to load config: {}", e);
        e
    })?;
    if args.inline {
        config.mode = Some("Inline".to_string());
    }
    let prompt = prompt::generate_prompt(&config).map_err(|e| {
        error!("Failed to generate prompt: {}", e);
        e
    })?;
    print!("{}", prompt);
    Ok(())
}
