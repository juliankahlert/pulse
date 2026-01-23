use clap::Parser;

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
}
