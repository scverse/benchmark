use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Start web hook server
    Serve(ServeArgs),
}

#[derive(Args)]
pub(crate) struct ServeArgs {
    /// IP and port to listen on
    #[arg(default_value = "0.0.0.0:3000")]
    pub(crate) addr: String,
    /// Webhook secret as configured on GitHub
    #[arg(long, env)]
    pub(crate) secret_token: String,
}
