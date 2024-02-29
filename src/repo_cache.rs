use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::Path;

use crate::{event::ORG, utils::PipeMap};

lazy_static::lazy_static! {
    static ref DIRS: ProjectDirs = ProjectDirs::from("org", "scverse", "scverse benchmark").expect("No Home dir");
    static ref CACHE_DIR: &'static Path = DIRS.cache_dir();
}

/// Sync repo to match remote’s branch. If branch is None, sync to default branch.
pub(crate) fn sync_repo(repo: &str, branch: Option<&str>) -> Result<git2::Repository> {
    let path = CACHE_DIR.join(repo);
    let repo = if path.is_dir() {
        let repo = git2::Repository::open(path)?;
        // fetch from remote
        let branch = {
            let mut remote = repo.find_remote("origin")?;
            remote.connect(git2::Direction::Fetch)?;
            let branch =
                branch.map_or_else(|| get_default_branch(&remote), |b| Ok(b.to_owned()))?;
            remote.fetch(&[&branch], None, None)?;
            branch
        };
        {
            // get or create local branch
            let local_branch =
                sync_local_branch(&repo, &branch, &repo.find_reference("FETCH_HEAD")?)?;
            // switch to local branch
            repo.set_head(
                local_branch
                    .get()
                    .name()
                    .context("ref name is not valid UTF-8")?,
            )?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        }
        repo
    } else {
        git2::build::RepoBuilder::new()
            .pipe_map_ref(branch, |builder, branch| builder.branch(branch))
            .clone(&format!("https://github.com/{ORG}/{repo}"), &path)?
    };
    Ok(repo)
}

fn sync_local_branch<'repo>(
    repo: &'repo git2::Repository,
    branch: &str,
    target_ref: &git2::Reference,
) -> Result<git2::Branch<'repo>> {
    if let Ok(mut local_branch) = repo.find_branch(branch, git2::BranchType::Local) {
        let oid = target_ref
            .target()
            .context("FETCH_HEAD is not a direct reference")?;
        local_branch.get_mut().set_target(oid, "fetch head")?;
        Ok(local_branch)
    } else {
        Ok(repo.branch(branch, &target_ref.peel_to_commit()?, true)?)
    }
}

fn get_default_branch(remote: &git2::Remote) -> Result<String> {
    Ok(remote
        .default_branch()?
        .as_str()
        .context("default branch is not valid UTF-8")?
        .to_owned())
}
