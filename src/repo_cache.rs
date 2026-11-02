use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use gix::{interrupt::IS_INTERRUPTED, progress::Discard, worktree};
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::constants::ORG;

static DIRS: LazyLock<ProjectDirs> = LazyLock::new(|| {
    ProjectDirs::from("org", "scverse", "scverse-benchmark").expect("No Home dir")
});
static CACHE_DIR: LazyLock<&'static Path> = LazyLock::new(|| DIRS.cache_dir());

/// A worktree of a cached bare repo, removed (with everything ASV put into it) on drop.
#[derive(Debug)]
pub(crate) struct Checkout {
    git_dir: PathBuf,
    pub path: PathBuf,
}

impl Checkout {
    /// Fetch `refspecs` from origin into the cached repo.
    pub(crate) fn fetch(&self, refspecs: &[String]) -> Result<()> {
        fetch(&gix::open(&self.git_dir)?, refspecs)?;
        Ok(())
    }
}

impl Drop for Checkout {
    fn drop(&mut self) {
        let removed = gix::open(&self.git_dir)
            .map_err(anyhow::Error::from)
            .and_then(|repo| remove_worktree(&repo, &self.path));
        if let Err(e) = removed {
            tracing::error!("Failed to remove {}: {e:#}", self.path.display());
        }
    }
}

/// Check out remote’s ref into a fresh worktree. If ref is None, check out default branch.
pub(crate) fn sync_repo(repo: &str, to_ref: Option<&str>) -> Result<(Checkout, String)> {
    let git_dir = CACHE_DIR.join(format!("{repo}.git"));
    let bare = if git_dir.is_dir() {
        gix::open(&git_dir)?
    } else {
        let url = format!("https://github.com/{ORG}/{repo}.git");
        gix::prepare_clone_bare(url.as_str(), &git_dir)?
            .fetch_only(Discard, &IS_INTERRUPTED)
            .context(anyhow!("failed to clone {url}"))?
            .0
    };
    let to_ref = to_ref.unwrap_or("HEAD").to_owned();
    let commit = fetch(&bare, std::slice::from_ref(&to_ref))?
        .ref_map
        .mappings
        .iter()
        .find_map(|m| m.remote.as_id())
        .context(anyhow!("{to_ref} not found on remote"))?
        .to_owned();
    // ASV configs refer to the repo as e.g. `../../anndata`, so the directory needs the repo’s name
    let path = CACHE_DIR.join("worktrees").join(repo);
    remove_worktree(&bare, &path)?; // left over if we crashed
    bare.add_worktree(
        &path,
        worktree::add::Head::Detached(commit),
        Discard,
        &IS_INTERRUPTED,
    )?;
    Ok((Checkout { git_dir, path }, to_ref))
}

fn fetch(repo: &gix::Repository, refspecs: &[String]) -> Result<gix::remote::fetch::Outcome> {
    Ok(repo
        .find_remote("origin")?
        .with_refspecs(
            refspecs.iter().map(String::as_str),
            gix::remote::Direction::Fetch,
        )?
        .connect(gix::remote::Direction::Fetch)?
        .prepare_fetch(Discard, gix::remote::ref_map::Options::default())?
        .receive(Discard, &IS_INTERRUPTED)?)
}

fn remove_worktree(repo: &gix::Repository, path: &Path) -> Result<()> {
    if let Ok(target) = repo.prepare_remove_worktree(path) {
        target.remove(worktree::remove::Force::OverrideLock, Discard)?;
    }
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_fetch_and_cleanup() -> Result<()> {
        let (checkout, _) = sync_repo("anndata", None)?;
        assert!(checkout.path.join("benchmarks/asv.conf.json").is_file());
        // ASV fetches branches by name and commits by SHA
        let sha = "f69cc685c7d820167a249a9d11e997651ed583ce".to_owned();
        checkout.fetch(&["+refs/heads/main:refs/heads/main".to_owned(), sha.clone()])?;
        let repo = gix::open(&checkout.git_dir)?;
        repo.find_reference("refs/heads/main")?;
        repo.find_object(gix::ObjectId::from_hex(sha.as_bytes())?)?;
        let path = checkout.path.clone();
        drop(checkout);
        assert!(!path.exists());
        assert!(repo.worktrees()?.is_empty());
        Ok(())
    }
}
