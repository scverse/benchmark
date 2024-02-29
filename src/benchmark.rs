use std::borrow::Borrow;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::event::RunBenchmark;
use crate::repo_cache::sync_repo;

pub(crate) async fn sync_repo_and_run(
    RunBenchmark {
        repo,
        branch,
        run_on,
    }: RunBenchmark,
) -> Result<()> {
    let repo = tokio::task::spawn_blocking(move || sync_repo(&repo, branch.as_deref())).await??;
    tracing::info!("Synced repo to {:?}", repo.path());
    run_benchmark(repo, &run_on[..]).await?;
    Ok(())
}

async fn run_benchmark<S: Borrow<str>>(repo: git2::Repository, on: &[S]) -> Result<()> {
    let wd = {
        let wd = repo.workdir().context("no workdir")?;
        if wd.join("benchmarks").join("asv.conf.json").is_file() {
            wd.join("benchmarks")
        } else {
            wd.to_path_buf()
        }
    };

    tracing::info!("Running asv in {}", wd.display());
    let mut command = Command::new("asv");
    command.current_dir(&wd).arg("run");
    let mut child = if on.is_empty() {
        command.spawn().context("failed to spawn `asv run`")?
    } else {
        let mut child = command
            .stdin(Stdio::piped())
            .arg("HASHFILE:-")
            .spawn()
            .context("failed to spawn `asv run HASHFILE:-`")?;
        let mut stdin = child.stdin.take().context("no stdin")?;
        stdin.write_all(on.join("\n").as_bytes()).await?;
        stdin.flush().await?;
        child
    };
    let result = child.wait().await?;
    tracing::info!("asv exited with {result}");

    Ok(())
}
