use anyhow::Result;
use directories::ProjectDirs;
use std::path::Path;

use crate::app::ORG;

lazy_static::lazy_static! {
    static ref DIRS: ProjectDirs = ProjectDirs::from("org", "scverse", "scverse benchmark").expect("No Home dir");
    static ref CACHE_DIR: &'static Path = DIRS.cache_dir();
}

pub(crate) fn sync_repo(repo: &str, branch: &str) -> Result<git2::Repository> {
    let path = CACHE_DIR.join(repo);
    let repo = if path.is_dir() {
        let r = git2::Repository::open(path)?;
        // TODO: fetch
        // r.set_head(branch)?; // TODO: use correct ref
        r.checkout_head(None)?;
        r
    } else {
        git2::build::RepoBuilder::new()
            .branch(branch)
            .clone(&format!("https://github.com/{ORG}/{repo}"), &path)?
    };
    Ok(repo)
}
