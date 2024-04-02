/// Run ASV
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::repo_cache::sync_repo;
use crate::traits::RunConfig;

/// Sync repo to match remote’s branch, and run ASV afterwards.
pub(crate) async fn sync_repo_and_run<R>(req: &R) -> Result<PathBuf>
where
    R: RunConfig + Send + Sync + Clone,
{
    let (repo, config_ref) = {
        // clone data used in the thread
        let repo = req.repo().to_owned();
        let config_ref = req.config_ref().map(str::to_owned);
        tokio::task::spawn_blocking(move || sync_repo(&repo, config_ref.as_deref())).await??
    };
    tracing::info!("Synced config repo to {:?} @ {config_ref}", repo.path());
    let wd = run_benchmark(repo, req.run_on()).await?;
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

async fn run_benchmark(repo: git2::Repository, on: &[String]) -> Result<PathBuf> {
    let wd = {
        let on = on.to_owned();
        tokio::task::spawn_blocking(move || fetch_configured_refs(&repo, &on)).await??
    };

    tracing::info!("Re-discovering benchmarks in {}", wd.display());
    let result = asv_command(&wd)
        .args(["run", "--bench=just-discover"])
        .args(on.iter().next_back().as_slice())
        .spawn()?
        .wait()
        .await?;
    if result.code() != Some(0) {
        bail!("asv run --bench=just-discover exited with {result}");
    }

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
    if result.code() != Some(0) {
        bail!("asv run exited with {result}");
    }

    Ok(wd)
}

#[derive(Deserialize)]
struct AsvConfig {
    #[serde(default = "default_branches")]
    branches: Vec<String>,
}

fn default_branches() -> Vec<String> {
    vec!["master".to_owned()]
}

fn fetch_configured_refs(repo: &git2::Repository, refs: &[String]) -> Result<PathBuf> {
    let wd = {
        let wd = repo.workdir().context("no workdir")?;
        if wd.join("benchmarks").join("asv.conf.json").is_file() {
            wd.join("benchmarks")
        } else {
            wd.to_path_buf()
        }
    };
    // read ASV config
    let file = File::open(wd.join("asv.conf.json"))?;
    let mut buffer = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut buffer)?;
    let config: AsvConfig = serde_json5::from_str(&buffer)?;

    {
        let mut remote = repo.find_remote("origin")?;
        let refs: Vec<String> = config
            .branches
            .iter()
            .map(|b| format!("refs/heads/{b}"))
            .chain(refs.iter().cloned())
            .collect();
        tracing::info!(
            "Fetching refs {refs:?} from remote {}",
            remote.name().unwrap_or("")
        );
        remote.fetch(&refs, None, None)?;
    }
    Ok(wd)
}
