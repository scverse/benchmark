use anyhow::{Context, Result};
use octocrab::models::Installation;
use secrecy::ExposeSecret;

use crate::{
    cli,
    constants::{APP_ID, ORG},
};

use super::Auth;

pub(super) async fn auth_to_octocrab<A>(auth: A) -> Result<octocrab::Octocrab>
where
    A: TryInto<Auth, Error = anyhow::Error> + Default,
{
    match auth.try_into()? {
        cli::Auth::AppKey(app_key) => {
            let key = jsonwebtoken::EncodingKey::from_rsa_pem(app_key.expose_secret().as_bytes())?;
            let base = octocrab::Octocrab::builder().app(APP_ID, key).build()?;
            let Installation { id, html_url, .. } = base
                .apps()
                .get_org_installation(ORG)
                .await
                .context("failed to get org installation")?;
            tracing::info!(
                "Found installation: {}",
                html_url.unwrap_or_else(|| id.to_string())
            );
            Ok(octocrab::Octocrab::installation(&base, id))
        }
        cli::Auth::GitHubToken(github_token) => Ok(octocrab::Octocrab::builder()
            .personal_token(github_token)
            .build()?),
    }
}
