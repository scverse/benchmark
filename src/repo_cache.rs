use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use std::path::Path;

use crate::constants::ORG;

lazy_static::lazy_static! {
    static ref DIRS: ProjectDirs = ProjectDirs::from("org", "scverse", "scverse benchmark").expect("No Home dir");
    static ref CACHE_DIR: &'static Path = DIRS.cache_dir();
}

/// Sync repo to match remote’s ref. If ref is None, sync to default branch.
pub(crate) fn sync_repo(repo: &str, to_ref: Option<&str>) -> Result<(git2::Repository, String)> {
    let path = CACHE_DIR.join(repo);
    let repo = if path.is_dir() {
        git2::Repository::open(path)?
    } else {
        let url = format!("https://github.com/{ORG}/{repo}.git");
        git2::build::RepoBuilder::new()
            .clone(&url, &path)
            .context(anyhow!("failed to clone {url}"))?
    };
    // fetch from remote
    let to_ref = {
        let mut remote = repo.find_remote("origin")?;
        remote.connect(git2::Direction::Fetch)?;
        let to_ref = to_ref.map_or_else(|| get_default_branch(&remote), |b| Ok(b.to_owned()))?;
        remote.fetch(&[&to_ref], None, None)?;
        to_ref
    };
    // switch to ref
    repo.set_head("FETCH_HEAD")?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok((repo, to_ref))
}

fn get_default_branch(remote: &git2::Remote) -> Result<String> {
    Ok(remote
        .default_branch()?
        .as_str()
        .context("default branch is not valid UTF-8")?
        .to_owned())
}
