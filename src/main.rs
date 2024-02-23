use anyhow::Result;
use clap::Parser;

mod benchmark;
mod cli;
mod event;
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
    }
    Ok(())
}
