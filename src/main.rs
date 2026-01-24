//! Pulse: A fast, configurable Rust PS1 prompt engine for modern shells.
//!
//! This application generates shell prompts with support for Git repositories,
//! customizable colors, and different display modes.

use anyhow::Result;
use clap::Parser;
use log::error;

mod args;
mod clrs;
mod config;
mod prompt;

/// Main entry point for the Pulse application.
///
/// Initializes logging, parses command-line arguments, loads configuration,
/// generates the prompt, and prints it to stdout.
fn main() -> Result<()> {
    env_logger::init();
    let _args = args::Args::parse();
    let _config = config::Config::load().map_err(|e| {
        error!("Failed to load config: {}", e);
        e
    })?;
    let prompt = prompt::generate_prompt(&_config).map_err(|e| {
        error!("Failed to generate prompt: {}", e);
        e
    })?;
    print!("{}", prompt);
    Ok(())
}
