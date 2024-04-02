use anyhow::{Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use lazy_static::lazy_static;
use octocrab::{params::repos::Reference, GitHubError};

use crate::constants::ORG;

lazy_static! {
    static ref SHA1_RE: regex::Regex = regex::Regex::new(r"^[a-f0-9]{40}$").unwrap();
}

pub(super) async fn ref_exists(
    github_client: &octocrab::Octocrab,
    repo: &str,
    git_ref: &str,
) -> Result<bool> {
    if SHA1_RE.is_match(git_ref) {
        return Ok(github_client
            .commits(ORG, repo)
            .get(git_ref)
            .await
            .found()
            .context("failed to check if commit exists")?
            .is_some());
    }
    stream::iter([
        Reference::Branch(git_ref.to_owned()),
        Reference::Tag(git_ref.to_owned()),
    ])
    .then(|reference| async move {
        github_client
            .repos(ORG, repo)
            .get_ref(&reference)
            .await
            .found()
    })
    .try_any(|ref_| async move { ref_.is_some() })
    .await
    .context("failed to check if ref exists")
}

trait OctocrabOptional<T> {
    fn found(self) -> octocrab::Result<Option<T>>;
}

impl<T> OctocrabOptional<T> for octocrab::Result<T> {
    fn found(self) -> octocrab::Result<Option<T>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(octocrab::Error::GitHub {
                source:
                    GitHubError {
                        status_code: http::StatusCode::NOT_FOUND,
                        ..
                    },
                ..
            }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sha1() {
        assert!(SHA1_RE.is_match("fb803f6392801d8c30dce7e5645a540ba74394fc"));
        assert!(!SHA1_RE.is_match("fb803f"));
        assert!(!SHA1_RE.is_match("xxxxxf6392801d8c30dce7e5645a540ba74394fc"));
    }
}
