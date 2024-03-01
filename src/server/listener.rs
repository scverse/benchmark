use anyhow::Result;
use futures::{channel::mpsc::Sender, SinkExt};
use std::sync::Arc;

use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use axum_github_webhook_extract::{GithubEvent, GithubToken as GitHubSecret};
use octocrab::{params::repos::Reference, Octocrab};
use tower_http::trace::TraceLayer;

use crate::event::{Event, PullRequestEvent, PullRequestEventAction, RunBenchmark, ORG};

#[derive(Debug, Clone)]
struct AppState {
    secret: GitHubSecret,
    sender: Sender<Event>,
    github_client: Arc<Octocrab>,
}

impl FromRef<AppState> for GitHubSecret {
    fn from_ref(state: &AppState) -> GitHubSecret {
        state.secret.clone()
    }
}

async fn handle(
    State(state): State<AppState>,
    GithubEvent(event): GithubEvent<PullRequestEvent>,
) -> impl IntoResponse {
    match event.action {
        PullRequestEventAction::Synchronize(sync) => {
            if event
                .pull_request
                .labels
                .iter()
                .flatten()
                .all(|e| e.name != "benchmark")
            {
                return Ok("skipped".to_owned());
            }
            let e = RunBenchmark {
                repo: event.repository.name,
                branch: None,
                run_on: vec![event.pull_request.base.sha, sync.after],
            };
            handle_enqueue(e, state).await
        }
    }
}

async fn handle_enqueue(
    req: RunBenchmark,
    mut state: AppState,
) -> Result<String, (StatusCode, String)> {
    let branch_ok = if let Some(branch) = &req.branch {
        state
            .github_client
            .repos(ORG, &req.repo)
            .get_ref(&Reference::Branch(branch.to_owned()))
            .await
            .is_ok()
    } else {
        true
    };
    if branch_ok {
        state
            .sender
            .send(req.into())
            .await
            .map(|()| "enqueued".to_owned())
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error: Failed to send event".to_owned(),
                )
            })
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            format!("Error: {req} is not a valid repo/branch"),
        ))
    }
}

pub(crate) fn listen(sender: Sender<Event>, secret: &str) -> axum::Router {
    let state = AppState {
        sender,
        secret: GitHubSecret(Arc::new(secret.to_owned())),
        github_client: octocrab::instance(),
    };

    Router::new()
        .route("/", post(handle))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Arc};

    use assert_json_diff::assert_json_eq;
    use axum::{
        body::Body, extract::Request, http::StatusCode, response::IntoResponse, routing::post,
        Router,
    };
    use axum_github_webhook_extract::{GithubEvent, GithubToken as GitHubSecret};
    use hmac_sha256::HMAC;
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::util::ServiceExt;

    use crate::{event::PullRequestEvent, fixtures::PR};

    const TEST_SECRET: &str = "It's a Secret to Everybody";

    async fn handle_test(GithubEvent(event): GithubEvent<PullRequestEvent>) -> impl IntoResponse {
        serde_json::to_string(&event.action).unwrap()
    }

    fn app() -> Router {
        Router::new()
            .route("/", post(handle_test))
            .with_state(GitHubSecret(Arc::new(TEST_SECRET.to_owned())))
    }

    fn make_request<B: Into<Body> + AsRef<[u8]>>(body: B, valid: bool) -> Request {
        let mac = if valid {
            HMAC::mac(&body, TEST_SECRET.as_bytes())
        } else {
            [0; 32]
        };
        Request::builder()
            .method("POST")
            .header(
                "X-Hub-Signature-256",
                format!("sha256={}", hex::encode(mac)),
            )
            .body(body.into())
            .unwrap()
    }

    async fn body_string(body: Body) -> String {
        String::from_utf8_lossy(&body.collect().await.unwrap().to_bytes()).into_owned()
    }

    #[tokio::test]
    async fn valid_pr_event() {
        let request = make_request(PR, true);
        let res = app().oneshot(request).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_json_eq!(
            serde_json::Value::from_str(&body_string(res.into_body()).await).unwrap(),
            json!({
                "action": "synchronize",
                "before": "cc6d6ea741ff6c35df3747a95c4869cc3ed5f84e",
                "after": "f88f7bd4250b963752d615e491b7e676ce5eb7f0",
            })
        );
    }

    #[tokio::test]
    async fn invalid_signature() {
        let request = make_request(PR, false);
        let res = app().oneshot(request).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(&body_string(res.into_body()).await, "signature mismatch");
    }

    #[tokio::test]
    async fn invalid_event_payload() {
        let request = make_request("{}", true);
        let res = app().oneshot(request).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            &body_string(res.into_body()).await,
            "missing field `number` at line 1 column 2"
        );
    }
}
