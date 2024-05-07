#![warn(clippy::pedantic)]

use anyhow::{bail, Result};
use benchmark::RunResult;
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

    // Set global octocrab instance, either using the provided auth or in --dry-run mode
    octocrab::initialise(std::mem::take(&mut cli.auth).try_into_octocrab().await?);

    match cli.command {
        cli::Commands::Serve(args) => {
            server::serve(args).await?;
        }
        cli::Commands::Run(args) => {
            let RunResult {
                success,
                wd,
                env_specs,
            } = benchmark::sync_repo_and_run(&args).await?;
            // if exactly two are specified, show a comparison
            if let [before, after] = args.run_on.as_slice() {
                benchmark::AsvCompare::new(&wd, before, after)
                    .in_envs(env_specs)
                    .run()
                    .await?;
            }
            if !success {
                bail!("Benchmark run failed");
            }
        }
    }
    Ok(())
}
