use anyhow::Result;
use futures::{channel::mpsc::channel, TryFutureExt};
use std::future::IntoFuture;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use crate::cli::ServeArgs;
use crate::event::Event;

mod listener;
mod octocrab_utils;
mod runner;

pub(crate) async fn serve(args: ServeArgs) -> Result<()> {
    let (sender, receiver) = channel::<Event>(32);
    let service = listener::listen(sender, args.secret_token);
    let tcp_listener = TcpListener::bind(args.addr).await?;

    let mut set: JoinSet<Result<()>> = JoinSet::new();
    set.spawn(axum::serve(tcp_listener, service).into_future().err_into());
    set.spawn(runner::runner(receiver));
    while let Some(res) = set.join_next().await {
        let _ = res?;
    }
    Ok(())
}
