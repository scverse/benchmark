use anyhow::{Context, Result};
use futures::{channel::mpsc::Receiver, StreamExt};
use tokio::process::Command;

use crate::event::{Enqueue, Event};
use crate::git::sync_repo;
use crate::utils::PipeMap;

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

    let wd = repo.workdir().context("no workdir")?;
    let result = Command::new("asv")
        .arg("run")
        .pipe_map(e.run_on, |cmd, run_on| cmd.arg(run_on))
        .current_dir(wd)
        .spawn()?
        .wait()
        .await?;

    tracing::info!("asv exited with {result}");
    Ok(())
}
