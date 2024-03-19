use anyhow::Result;
use futures::TryStreamExt;
use secrecy::ExposeSecret;

use crate::{cli, constants::APP_ID};

use super::Auth;

pub(super) async fn auth_to_octocrab<A>(auth: &mut A) -> Result<octocrab::Octocrab>
where
    A: TryInto<Auth, Error = anyhow::Error> + Default,
{
    match std::mem::take(auth).try_into()? {
        cli::Auth::AppKey(app_key) => {
            let key = jsonwebtoken::EncodingKey::from_rsa_pem(app_key.expose_secret().as_bytes())?;
            let base = octocrab::Octocrab::builder().app(APP_ID, key).build()?;
            let insts = base
                .apps()
                .installations()
                .send()
                .await?
                .into_stream(&base)
                .try_collect::<Vec<_>>()
                .await?;
            tracing::info!("Installations: {}", serde_json5::to_string(&insts)?);
            Ok(octocrab::Octocrab::installation(&base, insts[0].id))
        }
        cli::Auth::GitHubToken(github_token) => {
            Ok(octocrab::Octocrab::builder()
                // https://github.com/XAMPPRocky/octocrab/issues/594
                .personal_token(github_token.expose_secret().to_owned())
                .build()?)
        }
    }
}
