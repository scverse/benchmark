use anyhow::Result;
use clap::Parser;

mod benchmark;
mod cli;
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
            let wd = benchmark::sync_repo_and_run(args.clone()).await?;
            if args.run_on.len() == 2 {
                benchmark::asv_command(&wd)
                    .args(["compare", "--only-changed"])
                    .args(&args.run_on)
                    .spawn()?
                    .wait()
                    .await?;
            }
        }
    }
    Ok(())
}
