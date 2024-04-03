#![warn(clippy::pedantic)]

use anyhow::Result;
use clap::Parser;

mod benchmark;
mod cli;
mod constants;
mod event;
#[cfg(test)]
mod fixtures;
mod nightly_backports;
mod octocrab_utils;
mod repo_cache;
mod server;
mod traits;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    cli::init_tracing();

    let mut cli = cli::Cli::parse();

    octocrab::initialise(std::mem::take(&mut cli.auth).into_octocrab().await?);

    match cli.command {
        cli::Commands::Serve(args) => {
            server::serve(args).await?;
        }
        cli::Commands::Run(args) => {
            let wd = benchmark::sync_repo_and_run(&args).await?;
            // if exactly two are specified, show a comparison
            if let [before, after] = args.run_on.as_slice() {
                benchmark::AsvCompare::new(&wd, before, after).run().await?;
            }
        }
    }
    Ok(())
}
