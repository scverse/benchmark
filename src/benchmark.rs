use anyhow::{Context, Result};
use tokio::process::Command;

use crate::event::Enqueue;
use crate::repo_cache::sync_repo;
use crate::utils::PipeMap;

pub(crate) async fn sync_repo_and_run(e: Enqueue) -> Result<()> {
    tracing::info!("Handling request for {e} on {:?}", e.run_on);
    let (repo, e) =
        tokio::task::spawn_blocking(move || sync_repo(&e.repo, &e.branch).map(|r| (r, e)))
            .await??;
    tracing::info!("Synced repo to {:?}", repo.path());
    run_benchmark(e, repo).await?;
    Ok(())
}

async fn run_benchmark(e: Enqueue, repo: git2::Repository) -> Result<()> {
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
