use anyhow::{Context, Result};
use futures::{channel::mpsc::Receiver, StreamExt};
use tokio::process::Command;

use crate::app::{Enqueue, Event};
use crate::git::sync_repo;

pub(crate) async fn runner(mut receiver: Receiver<Event>) -> Result<()> {
    // loop runs until sender disconnects
    while let Some(event) = receiver.next().await {
        match event {
            Event::Enqueue(e) => {
                tracing::info!("Handling request for {e} on {:?}", e.run_on);
                run(e).await?
            }
        }
    }
    Ok(())
}

async fn run(e: Enqueue) -> Result<()> {
    // use config from main branch
    let repo = tokio::task::spawn_blocking(move || sync_repo(&e.repo, &e.branch)).await??;
    tracing::info!("Synced repo to {:?}", repo.path());
    let mut cmd = Command::new("asv");
    cmd.arg("run");
    if let Some(run_on) = e.run_on {
        cmd.arg(run_on);
    }
    let result = cmd
        .current_dir(repo.workdir().context("no workdir")?)
        .spawn()?
        .wait()
        .await?;

    tracing::info!("asv exited with {result}");
    Ok(())
}
