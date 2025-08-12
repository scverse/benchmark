use std::sync::LazyLock;

use anyhow::{Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use octocrab::{params::repos::Reference, GitHubError};
use regex::Regex;

use crate::constants::ORG;
use crate::nightly_backports::floor_char_boundary;

static SHA1_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-f0-9]{40}$").unwrap());

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
            Err(octocrab::Error::GitHub { source, .. })
                if source.status_code == http::StatusCode::NOT_FOUND =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

pub(super) fn clamp_lines(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let text = &text[..floor_char_boundary(text, max_bytes)];
    if let Some(idx) = text.rfind('\n') {
        &text[..idx] // clamp it to the last line fitting the limit
    } else {
        text // no lines here, clamp it wherever
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

    #[test]
    fn test_clamp_lines() {
        assert_eq!(clamp_lines("foo\nbar\nbaz", 5), "foo");
        assert_eq!(clamp_lines("foo\nbar\nbaz", 10), "foo\nbar");
        assert_eq!(clamp_lines("foo\nbar\nbaz", 15), "foo\nbar\nbaz");
    }

    #[test]
    /// Test that we clamp within lines but only at char boundaries
    fn test_clamp_inline() {
        assert_eq!(clamp_lines("foo xyz", 5), "foo x");
        assert_eq!(clamp_lines("老虎", 5), "老");
    }
}
