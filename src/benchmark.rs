/// Run ASV
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::repo_cache::sync_repo;
use crate::traits::RunConfig;

#[derive(Default, Debug, Clone)]
pub(crate) struct EnvSpecs(pub Vec<String>);

impl EnvSpecs {
    pub fn args(&self) -> Vec<&str> {
        self.0
            .iter()
            .flat_map(|env_spec| ["-E", env_spec])
            .collect()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RunResult {
    pub success: bool,
    pub wd: PathBuf,
    pub env_specs: EnvSpecs,
}

/// Sync repo to match remote’s branch, and run ASV afterwards.
pub(crate) async fn sync_repo_and_run<R>(req: &R) -> Result<RunResult>
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
    run_benchmark(repo, req.run_on()).await
}

/// Create an `asv` command in the working directory
pub(crate) fn asv_command(wd: &Path) -> Command {
    let mut command = Command::new("asv");
    command.current_dir(wd);
    command
}

#[derive(Default, Debug, Clone)]
pub(crate) struct AsvCompare {
    wd: PathBuf,
    left: String,
    right: String,
    only_changed: bool,
    /// The envs to run on. If empty, run on all
    envs: EnvSpecs,
}

impl AsvCompare {
    pub fn new(wd: &Path, left: &str, right: &str) -> Self {
        Self {
            wd: wd.to_path_buf(),
            left: left.into(),
            right: right.into(),
            only_changed: true,
            envs: EnvSpecs::default(),
        }
    }
    pub fn only_changed(&mut self, only_changed: bool) -> &mut Self {
        self.only_changed = only_changed;
        self
    }
    pub fn in_envs(&mut self, envs: EnvSpecs) -> &mut Self {
        self.envs = envs;
        self
    }
    fn command(&self) -> Command {
        let mut command = asv_command(&self.wd);
        command.arg("compare");
        if self.only_changed {
            command.arg("--only-changed");
        }
        command
            .args(self.envs.args())
            .args([&self.left, &self.right]);
        command
    }
    pub async fn run(&self) -> Result<()> {
        self.command()
            .spawn()?
            .wait()
            .await?
            .success()
            .then_some(())
            .ok_or_else(|| anyhow!("asv compare failed"))
    }
    pub async fn output(&self) -> Result<String> {
        let Output {
            stdout,
            stderr,
            status,
        } = self.command().output().await?;
        if status.code() == Some(0) {
            return Ok(String::from_utf8(stdout)?);
        }
        Err(anyhow::anyhow!(
            "asv compare exited with {status}:\n{}",
            String::from_utf8_lossy(&stderr)
        ))
    }
}

pub async fn resolve_env(wd: &Path) -> Result<EnvSpecs> {
    tracing::info!("Resolving Environments: {:?}", wd);
    let env_specs = resolve_env_from_stdout(
        Command::new("python")
            .current_dir(wd)
            .args(["-c", include_str!("resolve_env.py")]),
    )
    .await?;
    Ok(EnvSpecs(env_specs))
}

async fn resolve_env_from_stdout(command: &mut Command) -> Result<Vec<String>> {
    let stdout_env_specs_buffer = command.output().await?.stdout;
    let stdout_env_specs = String::from_utf8(stdout_env_specs_buffer)?;
    let parsed: Vec<String> = serde_json::from_str(&stdout_env_specs)?;
    tracing::info!("Found environments: {:?}", parsed);
    Ok(parsed)
}

async fn run_benchmark(repo: git2::Repository, on: &[String]) -> Result<RunResult> {
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
    let env_specs = resolve_env(&wd).await?;
    command
        .args(["run", "--launch-method=spawn"])
        .args(env_specs.args());
    // Adding .arg("--skip-existing-commits") would skip even if benchmarks changed
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
    let success = match result.code() {
        Some(0) => true,
        Some(2) => false,
        _ => bail!("asv run --bench=just-discover exited with {result}"),
    };

    Ok(RunResult {
        success,
        wd,
        env_specs,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_env() {
        let resolved_envs =
            resolve_env_from_stdout(Command::new("echo").arg("[\"env0\", \"env1\", \"env2\"]"))
                .await
                .expect("Parsing unexpectedly failed for echo-ing a list of strings.");
        assert_eq!(resolved_envs, vec!["env0", "env1", "env2"]);
    }

    #[tokio::test]
    async fn test_resolve_env_empty_json() {
        let resolved_envs = resolve_env_from_stdout(Command::new("echo").arg("[]"))
            .await
            .expect("Parsing unexpectedly failed for echo-ing an empty list.");
        assert_eq!(resolved_envs.len(), 0);
    }

    #[tokio::test]
    async fn test_resolve_env_crash_integer_list() {
        let e = resolve_env_from_stdout(Command::new("echo").arg("[1, 2, 3]"))
            .await
            .expect_err("Integer list is not an expected type");
        assert!(format!("{e:?}").contains("invalid type: integer `1`, expected a string"));
    }

    #[tokio::test]
    async fn test_resolve_env_crash_bad_command() {
        let e = resolve_env_from_stdout(&mut Command::new("echolllll"))
            .await
            .expect_err("echolllll should return an error");
        assert!(format!("{e:?}").starts_with("No such file or directory (os error 2)"));
    }
}
