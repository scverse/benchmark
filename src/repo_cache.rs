use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use std::{path::Path, sync::LazyLock};

use crate::constants::ORG;

static DIRS: LazyLock<ProjectDirs> = LazyLock::new(|| {
    ProjectDirs::from("org", "scverse", "scverse-benchmark").expect("No Home dir")
});
static CACHE_DIR: LazyLock<&'static Path> = LazyLock::new(|| DIRS.cache_dir());

/// Sync repo to match remote’s ref. If ref is None, sync to default branch.
pub(crate) fn sync_repo(repo: &str, to_ref: Option<&str>) -> Result<(git2::Repository, String)> {
    let path = CACHE_DIR.join(repo);
    let url = format!("https://github.com/{ORG}/{repo}.git");
    let clone = || {
        git2::build::RepoBuilder::new()
            .clone(&url, &path)
            .context(anyhow!("failed to clone {url}"))
    };
    let repo = if path.is_dir() {
        git2::Repository::open(&path)?
    } else {
        clone()?
    };
    match fetch_and_checkout(&repo, to_ref) {
        Ok(to_ref) => Ok((repo, to_ref)),
        Err(e) => {
            // libgit2 can’t check out over stale state, e.g. a submodule that became an in-tree dir
            tracing::warn!("Sync of cached {url} failed, re-cloning: {e:#}");
            drop(repo);
            std::fs::remove_dir_all(&path)?;
            let repo = clone()?;
            let to_ref = fetch_and_checkout(&repo, to_ref)?;
            Ok((repo, to_ref))
        }
    }
}

fn fetch_and_checkout(repo: &git2::Repository, to_ref: Option<&str>) -> Result<String> {
    // fetch from remote
    let to_ref = {
        let mut remote = repo.find_remote("origin")?;
        remote.connect(git2::Direction::Fetch)?;
        let to_ref = to_ref.map_or_else(|| get_default_branch(&remote), |b| Ok(b.to_owned()))?;
        remote.fetch(&[&to_ref], None, None)?;
        to_ref
    };
    // switch to first ref in FETCH_HEAD
    repo.set_head("FETCH_HEAD")?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok(to_ref)
}

fn get_default_branch(remote: &git2::Remote) -> Result<String> {
    Ok(remote
        .default_branch()?
        .as_str()
        .context("default branch is not valid UTF-8")?
        .to_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn git2_has_https() {
        // git2 ≥0.21 has no default features, so cloning from GitHub silently breaks without this
        assert!(git2::Version::get().https());
    }
}
