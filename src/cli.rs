//! Command-line argument parsing for Pulse.
//!
//! Defines the CLI interface using clap, allowing users to specify
//! configuration files and display modes.

use std::path::PathBuf;

use clap::{ArgGroup, Parser};

/// Command-line arguments for Pulse.
#[derive(Parser, Debug, Clone)]
#[command(name = "pulse")]
#[command(
    version,
    about = "A fast, configurable Rust PS1 prompt engine for modern shells"
)]
#[command(group(
    ArgGroup::new("install_ops")
        .args(["install", "uninstall"])
        .multiple(false)
))]
pub struct Args {
    /// Path to custom configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Use inline mode instead of dual-line
    #[arg(long)]
    pub inline: bool,

    /// Install Pulse to shell configuration
    #[arg(long)]
    pub install: bool,

    /// Uninstall Pulse from shell configuration
    #[arg(long, conflicts_with = "install")]
    pub uninstall: bool,

    /// Show actions without modifying files
    #[arg(long, requires = "install_ops")]
    pub dry_run: bool,

    /// Generate shell completions
    #[arg(long, value_name = "SHELL")]
    pub generate_completions: Option<String>,
}
