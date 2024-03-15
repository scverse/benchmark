use std::pin::pin;

use anyhow::{Context, Result};
use futures::{future, stream, StreamExt, TryStreamExt};
use lazy_static::lazy_static;
use octocrab::{
    models::repos::{Ref, RepoCommit},
    params::repos::Reference,
    Page,
};
use serde::de::DeserializeOwned;

use crate::{cli::RunBenchmark, constants::ORG};

lazy_static! {
    static ref SHA1_RE: regex::Regex = regex::Regex::new(r"^[a-f0-9]{40}$").unwrap();
}

pub(super) trait PageExt<I>
where
    I: DeserializeOwned + 'static,
{
    async fn find<F: Fn(&I) -> bool>(
        self,
        github_api: &octocrab::Octocrab,
        pred: F,
    ) -> octocrab::Result<Option<I>>;
}

impl<I> PageExt<I> for Page<I>
where
    I: DeserializeOwned + 'static,
{
    async fn find<F: Fn(&I) -> bool>(
        self,
        github_api: &octocrab::Octocrab,
        pred: F,
    ) -> octocrab::Result<Option<I>> {
        let items = pin!(self.into_stream(github_api));
        items
            .try_filter(|item| future::ready(pred(item)))
            .try_next()
            .await
    }
}

pub(super) async fn ref_exists(
    github_client: &octocrab::Octocrab,
    RunBenchmark {
        config_ref, repo, ..
    }: &RunBenchmark,
) -> Result<bool> {
    let Some(config_ref) = config_ref.as_ref() else {
        return Ok(true);
    };
    if SHA1_RE.is_match(config_ref) {
        return Ok(github_client
            .commits(ORG, repo)
            .get(config_ref)
            .await
            .found()
            .context("failed to check if commit exists")?
            .is_some());
    }
    stream::iter([
        Reference::Branch(config_ref.to_owned()),
        Reference::Tag(config_ref.to_owned()),
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

// TODO: switch to status_code once it exists
// https://github.com/XAMPPRocky/octocrab/issues/598
trait OctocrabOptional<T> {
    fn found(self) -> octocrab::Result<Option<T>>;
}

impl OctocrabOptional<RepoCommit> for octocrab::Result<RepoCommit> {
    fn found(self) -> octocrab::Result<Option<RepoCommit>> {
        match self {
            Ok(commit) => Ok(Some(commit)),
            Err(octocrab::Error::GitHub { source, .. })
                if source.message.starts_with("No commit found for SHA") =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

impl OctocrabOptional<Ref> for octocrab::Result<Ref> {
    fn found(self) -> octocrab::Result<Option<Ref>> {
        match self {
            Ok(ref_) => Ok(Some(ref_)),
            Err(octocrab::Error::GitHub { source, .. }) if source.message == "Not Found" => {
                Ok(None)
            }
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
