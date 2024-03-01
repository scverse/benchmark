/// Run ASV
use std::borrow::Borrow;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::event::RunBenchmark;
use crate::repo_cache::sync_repo;

/// Sync repo to match remote’s branch, and run ASV afterwards.
pub(crate) async fn sync_repo_and_run(
    RunBenchmark {
        repo,
        config_ref: branch,
        run_on,
    }: RunBenchmark,
) -> Result<PathBuf> {
    let repo = tokio::task::spawn_blocking(move || sync_repo(&repo, branch.as_deref())).await??;
    tracing::info!("Synced repo to {:?}", repo.path());
    let wd = run_benchmark(repo, &run_on[..]).await?;
    Ok(wd)
}

/// Create an `asv` command in the working directory
pub(crate) fn asv_command(wd: &Path) -> Command {
    let mut command = Command::new("asv");
    command.current_dir(wd);
    command
}

pub(crate) fn asv_compare_command(wd: &Path, left: &str, right: &str) -> Command {
    let mut command = asv_command(wd);
    command.args(["compare", "--only-changed", left, right]);
    command
}

async fn run_benchmark<S: Borrow<str>>(repo: git2::Repository, on: &[S]) -> Result<PathBuf> {
    let wd = {
        let wd = repo.workdir().context("no workdir")?;
        if wd.join("benchmarks").join("asv.conf.json").is_file() {
            wd.join("benchmarks")
        } else {
            wd.to_path_buf()
        }
    };

    tracing::info!("Running asv in {}", wd.display());
    let mut command = asv_command(&wd);
    command.arg("run").arg("--skip-existing-commits");
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

    Ok(wd)
}
