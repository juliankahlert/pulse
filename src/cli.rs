//! Command-line argument parsing for Pulse.
//!
//! Defines the CLI interface using clap, allowing users to specify
//! configuration files and display modes.

use clap::Parser;

/// Command-line arguments for Pulse.
#[derive(Parser, Debug, Clone)]
#[command(name = "pulse")]
#[command(
    version,
    about = "A fast, configurable Rust PS1 prompt engine for modern shells"
)]
pub struct Args {
    /// Path to custom configuration file
    #[arg(short, long)]
    pub config: Option<String>,

    /// Use inline mode instead of dual-line
    #[arg(long)]
    pub inline: bool,

    /// Install Pulse to shell configuration
    #[arg(long)]
    pub install: bool,

    /// Generate shell completions
    #[arg(long, value_name = "SHELL")]
    pub generate_completions: Option<String>,
}
