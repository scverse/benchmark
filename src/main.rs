#![warn(clippy::pedantic)]

use anyhow::{anyhow, Result};
use clap::Parser;

mod benchmark;
mod cli;
mod constants;
mod event;
#[cfg(test)]
mod fixtures;
mod repo_cache;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    cli::init_tracing();

    let cli = cli::Cli::parse();
    match cli.command {
        cli::Commands::Serve(args) => {
            server::serve(args).await?;
        }
        cli::Commands::Run(args) => {
            let wd = benchmark::sync_repo_and_run(&args).await?;
            // if exactly two are specified, show a comparison
            if let [before, after] = args.run_on.as_slice() {
                benchmark::asv_compare_command(&wd, before, after)
                    .spawn()?
                    .wait()
                    .await?
                    .success()
                    .then_some(())
                    .ok_or_else(|| anyhow!("asv compare failed"))?;
            }
        }
    }
    Ok(())
}
