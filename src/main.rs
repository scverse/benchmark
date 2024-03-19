#![warn(clippy::pedantic)]

use anyhow::{anyhow, Result};
use clap::Parser;
use futures::TryStreamExt;
use secrecy::ExposeSecret;

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

    let mut cli = cli::Cli::parse();

    // initialize octocrab
    match std::mem::take(&mut cli.auth).try_into()? {
        cli::Auth::AppKey(app_key) => {
            let key = jsonwebtoken::EncodingKey::from_rsa_pem(app_key.expose_secret().as_bytes())?;
            let base = octocrab::Octocrab::builder()
                .app(constants::APP_ID, key)
                .build()?;
            let insts = base
                .apps()
                .installations()
                .send()
                .await?
                .into_stream(&base)
                .try_collect::<Vec<_>>()
                .await?;
            tracing::info!("Installations: {}", serde_json5::to_string(&insts)?);
            let crab = octocrab::Octocrab::installation(&base, insts[0].id);
            octocrab::initialise(crab);
        }
        cli::Auth::GitHubToken(github_token) => {
            let crab = octocrab::Octocrab::builder()
                // https://github.com/XAMPPRocky/octocrab/issues/594
                .personal_token(github_token.expose_secret().to_owned())
                .build()?;
            octocrab::initialise(crab);
        }
    };

    // run command
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
