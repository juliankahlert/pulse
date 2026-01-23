use anyhow::Result;
use clap::Parser;

use crate::args::Args;

#[derive(Parser, Debug)]
#[command(name = "pulse")]
#[command(author = "Julian Kahlert")]
#[command(version = "0.1.0")]
#[command(about = "A fast, configurable Rust PS1 prompt engine for modern shells.", long_about = None)]
pub struct Cli {
	// PLAHOLDER
}


impl Cli {
    pub fn into_args(self) -> Result<Args> {
	Ok(Args{})
    }
}
