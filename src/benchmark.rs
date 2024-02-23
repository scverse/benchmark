use anyhow::{Context, Result};
use tokio::process::Command;

use crate::event::RunBenchmark;
use crate::repo_cache::sync_repo;
use crate::utils::PipeMap;

pub(crate) async fn sync_repo_and_run(
    RunBenchmark {
        repo,
        branch,
        run_on,
    }: RunBenchmark,
) -> Result<()> {
    let repo = tokio::task::spawn_blocking(move || sync_repo(&repo, branch.as_deref())).await??;
    tracing::info!("Synced repo to {:?}", repo.path());
    run_benchmark(repo, run_on.as_deref()).await?;
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
