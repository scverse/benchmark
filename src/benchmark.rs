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
    let wd = {
        let wd = repo.workdir().context("no workdir")?;
        if wd.join("benchmarks").join("asv.conf.json").is_file() {
            wd.join("benchmarks")
        } else {
            wd.to_path_buf()
        }
    };

    tracing::info!("Running asv in {}", wd.display());
    let result = Command::new("asv")
        .arg("run")
        .pipe_map(on, |cmd, run_on| cmd.arg(run_on))
        .current_dir(wd)
        .spawn()
        .context("failed to spawn asv command")?
        .wait()
        .await?;

    tracing::info!("asv exited with {result}");
    Ok(())
}
