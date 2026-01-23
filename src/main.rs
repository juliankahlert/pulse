use anyhow::Result;
use clap::Parser;
use log::error;

mod args;
mod clrs;
mod config;
mod prompt;

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
