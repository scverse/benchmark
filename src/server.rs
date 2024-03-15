use anyhow::Result;
use futures::FutureExt;
use futures::{channel::mpsc::channel, TryFutureExt};
use std::future::IntoFuture;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use crate::cli::ServeArgs;
use crate::event::Event;
use crate::utils::get_credential;

mod listener;
mod octocrab_utils;
mod runner;

pub(crate) async fn serve(args: ServeArgs) -> Result<()> {
    let (sender, receiver) = channel::<Event>(32);
    // If secret has not been passed via CLI or env, get it as a credential.
    let secret_token = args
        .secret_token
        .ok_or(())
        .or_else(|()| get_credential("webhook_secret"))?;

    let service = listener::listen(sender, secret_token);
    let tcp_listener = TcpListener::bind(&args.addr).await?;
    tracing::info!("Listening on {}", args.addr);

    let mut set: JoinSet<Result<()>> = JoinSet::new();
    set.spawn(axum::serve(tcp_listener, service).into_future().err_into());
    set.spawn(runner::runner(receiver).map(Result::Ok));
    while let Some(res) = set.join_next().await {
        let _ = res?;
    }
    Ok(())
}
