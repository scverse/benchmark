use anyhow::{Context, Result};
use tokio::process::Command;

use crate::event::RunBenchmark;
use crate::repo_cache::sync_repo;
use crate::utils::PipeMap;

pub(crate) async fn sync_repo_and_run(req: RunBenchmark) -> Result<()> {
    tracing::info!("Handling request for {req} on {:?}", req.run_on);
    let (repo, req) =
        tokio::task::spawn_blocking(move || sync_repo(&req.repo, &req.branch).map(|r| (r, req)))
            .await??;
    tracing::info!("Synced repo to {:?}", repo.path());
    run_benchmark(repo, req.run_on.as_deref()).await?;
    Ok(())
}

async fn run_benchmark(repo: git2::Repository, on: Option<&str>) -> Result<()> {
    let wd = repo.workdir().context("no workdir")?;
    let result = Command::new("asv")
        .arg("run")
        .pipe_map(on, |cmd, run_on| cmd.arg(run_on))
        .current_dir(wd)
        .spawn()?
        .wait()
        .await?;

    tracing::info!("asv exited with {result}");
    Ok(())
}
