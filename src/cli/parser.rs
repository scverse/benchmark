use clap::{Args, Parser, Subcommand};
use serde::Deserialize;

use secrecy::SecretString;
use std::fmt::Display;

use crate::constants::ORG;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Cli<T: Send + Clone + Sync> {
    #[command(subcommand)]
    pub(crate) command: Commands<T>,
    /// GitHub token used to make API requests.
    #[arg(long, short = 't', env)]
    pub(crate) github_token: Option<SecretString>,
}

#[derive(Subcommand)]
pub(crate) enum Commands<T: Send + Clone + Sync> {
    /// Start web hook server
    Serve(ServeArgs),
    /// Run a single benchmark
    Run(RunBenchmark<T>),
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
pub(crate) struct RunBenchmark<T: Send + Clone + Sync> {
    /// Repository containing ASV benchmarks (in scverse org)
    pub repo: String,
    /// Branch or commit to use benchmark configuration from
    #[arg(long, short)]
    pub config_ref: Option<String>,
    /// Which refs in the target repository to run benchmarks on (default: default branch)
    pub run_on: T,
}

impl<T: Send + Clone + Sync> Display for RunBenchmark<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{ORG}/{}", self.repo)?;
        if let Some(config_ref) = &self.config_ref {
            write!(f, "@{config_ref}")?;
        }
        Ok(())
    }
}
