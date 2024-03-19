use clap::{Args, Parser, Subcommand};
use serde::Deserialize;

use anyhow::Result;
use secrecy::SecretString;
use std::fmt::Display;

use crate::{constants::ORG, utils::get_credential};

use super::octocrab_utils::auth_to_octocrab;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,

    #[command(flatten)]
    pub(crate) auth: AuthInner,
}

// https://github.com/clap-rs/clap/issues/2621
#[derive(Default, Args)]
#[group(multiple = false)]
pub(crate) struct AuthInner {
    /// GitHub RSA private key for an app.
    #[arg(long, short = 'k', env)]
    app_key: Option<SecretString>,

    /// GitHub personal access token used to make API requests.
    #[arg(long, short = 't', env)]
    github_token: Option<SecretString>,
}

impl AuthInner {
    pub(crate) async fn into_octocrab(&mut self) -> Result<octocrab::Octocrab> {
        auth_to_octocrab(self).await
    }
}

pub(crate) enum Auth {
    AppKey(SecretString),
    GitHubToken(SecretString),
}

impl TryFrom<AuthInner> for Auth {
    type Error = anyhow::Error;

    /// If app key or token has been passed via CLI or env, use it, otherwise try to get as a credential.
    fn try_from(inner: AuthInner) -> Result<Self, Self::Error> {
        Ok(if let Some(app_key) = inner.app_key {
            tracing::info!("Using app key from CLI");
            Self::AppKey(app_key)
        } else if let Some(github_token) = inner.github_token {
            tracing::info!("Using GitHub token from CLI");
            Self::GitHubToken(github_token)
        } else if let Ok(app_key) = get_credential("app_key") {
            tracing::info!("Using app key from credential store");
            Self::AppKey(app_key)
        } else if let Ok(github_token) = get_credential("github_token") {
            tracing::info!("Using GitHub token from credential store");
            Self::GitHubToken(github_token)
        } else {
            // This doesn’t happen when parsed from CLI, only when constructed using ::default()
            anyhow::bail!("No credentials found");
        })
    }
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Start web hook server
    Serve(ServeArgs),
    /// Run a single benchmark
    Run(RunBenchmark),
}

#[derive(Args)]
pub(crate) struct ServeArgs {
    /// IP and port to listen on
    #[arg(default_value = "0.0.0.0:3000")]
    pub(crate) addr: String,
    /// Webhook secret as configured on GitHub
    #[arg(long, env)]
    pub(crate) secret_token: Option<SecretString>,
}

#[derive(Args, Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct RunBenchmark {
    /// Repository containing ASV benchmarks (in scverse org)
    pub repo: String,
    /// Branch or commit to use benchmark configuration from
    #[arg(long, short)]
    pub config_ref: Option<String>,
    /// Which refs in the target repository to run benchmarks on (default: default branch)
    pub run_on: Vec<String>,
}

impl Display for RunBenchmark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{ORG}/{}", self.repo)?;
        if let Some(config_ref) = &self.config_ref {
            write!(f, "@{config_ref}")?;
        }
        Ok(())
    }
}
