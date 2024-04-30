use axum::{
    body::Body, extract::Request, http::StatusCode, response::Response, routing::post, Router,
};
use axum_github_webhook_extract::GithubToken as GitHubSecret;
use futures::{
    channel::mpsc::{channel, Receiver},
    StreamExt,
};
use hmac_sha256::HMAC;
use http_body_util::BodyExt;
use octocrab::{
    models::{commits::Commit, webhook_events::payload::PullRequestWebhookEventPayload},
    Octocrab,
};
use std::sync::Arc;
use tower::util::ServiceExt;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use crate::constants::ORG;
use crate::event::{Compare, Event};
use crate::fixtures::{COMMIT, PR};

use super::{handle, AppState};

mod mock_error {
    use serde_json::json;
    use wiremock::{
        matchers::{method, path_regex},
        Mock, MockServer, ResponseTemplate,
    };

    // Sets up a handler on the mock server which will return a 500 with the given message. This
    // will be mapped internally into a GitHub json error, making it much easier to identify the cause
    // of these test failures.
    //
    // This handler should always come after your real expectations as it will match any GET request.
    pub async fn setup_error_handler(mock_server: &MockServer, message: impl ToString) {
        let message = message.to_string();
        Mock::given(method("GET"))
            .and(path_regex(".*"))
            .respond_with(move |req: &wiremock::Request| {
                ResponseTemplate::new(500).set_body_json(json!( {
                    "documentation_url": "",
                    "errors": None::<Vec<serde_json::Value>>,
                    "message": format!("{message} on {}", req.url),
                }))
            })
            .mount(mock_server)
            .await;
    }
}

use mock_error::setup_error_handler;

const TEST_SECRET: &str = "It's a Secret to Everybody";

async fn setup_github_api(template: Option<ResponseTemplate>) -> MockServer {
    let mock_server = MockServer::start().await;
    if let Some(template) = template {
        let uri =
            format!("/repos/{ORG}/benchmark/commits/0d41f8596349daeadaa17c551fa0598f0a95666d");
        Mock::given(method("GET"))
            .and(path(&uri))
            .respond_with(template)
            .mount(&mock_server)
            .await;
        setup_error_handler(&mock_server, format!("GET on {uri} was not received")).await;
    } else {
        setup_error_handler(&mock_server, "Unexpected GET").await;
    }
    mock_server
}

async fn app(template: Option<ResponseTemplate>) -> (Router, Receiver<Event>) {
    // https://github.com/flows-network/octocrab/blob/main/examples/custom_client.rs
    let mock_github_server = setup_github_api(template).await;
    let (sender, receiver) = channel(1);
    let state = AppState {
        sender,
        secret: GitHubSecret(Arc::new(TEST_SECRET.to_owned())),
        github_client: Arc::new(
            Octocrab::builder()
                .base_uri(mock_github_server.uri())
                .unwrap()
                .build()
                .unwrap(),
        ),
    };
    let router = Router::new().route("/", post(handle)).with_state(state);
    (router, receiver)
}

fn make_webhook_request<B: Into<Body> + AsRef<[u8]>>(body: B, valid: bool) -> Request {
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

async fn assert_status_eq(res: Response<Body>, status_expected: StatusCode) -> String {
    let status = res.status();
    let body = body_string(res.into_body()).await;
    if status == status_expected {
        return body;
    }
    panic!("{status} != {status_expected} ({body})",);
}

#[tokio::test]
async fn should_error_on_invalid_signature() {
    let (app, mut recv) = app(None).await;
    let request = make_webhook_request(PR, false);
    let res = app.oneshot(request).await.unwrap();

    let body = assert_status_eq(res, StatusCode::BAD_REQUEST).await;
    assert_eq!(&body, "signature mismatch");
    assert!(recv.next().await.is_none());
}

#[tokio::test]
async fn should_error_on_invalid_event_payload() {
    let (app, mut recv) = app(None).await;
    let request = make_webhook_request("{}", true);
    let res = app.oneshot(request).await.unwrap();

    let body = assert_status_eq(res, StatusCode::BAD_REQUEST).await;
    assert!(body.starts_with("missing field"));
    assert!(recv.next().await.is_none());
}

#[tokio::test]
async fn should_skip_on_no_label() {
    let commit_after: Commit = serde_json::from_str(COMMIT).unwrap();
    let template = ResponseTemplate::new(200).set_body_json(commit_after);
    let (app, mut recv) = app(Some(template)).await;
    // remove the benchmark label
    let mut evt: PullRequestWebhookEventPayload = serde_json::from_str(PR).unwrap();
    evt.pull_request.labels.as_mut().unwrap().clear();
    let request = make_webhook_request(serde_json::to_string(&evt).unwrap(), true);
    let res = app.oneshot(request).await.unwrap();

    let body = assert_status_eq(res, StatusCode::OK).await;
    assert_eq!(&body, "skipped: missing benchmark label");
    assert!(recv.next().await.is_none());
}

#[tokio::test]
async fn should_enqueue_valid_pr_event() {
    // pull request with benchmark label
    let evt: PullRequestWebhookEventPayload = serde_json::from_str(PR).unwrap();
    // expected event payload
    let sha_base: &str = evt.pull_request.base.sha.as_ref();
    let sha_head: &str = evt.pull_request.head.sha.as_ref();
    let commit_after: Commit = serde_json::from_str(COMMIT).unwrap();
    assert_eq!(commit_after.sha, sha_head);
    let template = ResponseTemplate::new(200).set_body_json(commit_after);
    let (app, mut recv) = app(Some(template)).await;
    let request = make_webhook_request(serde_json::to_string(&evt).unwrap(), true);
    let res = app.oneshot(request).await.unwrap();

    let body = assert_status_eq(res, StatusCode::OK).await;
    assert_eq!(body, "enqueued");
    let evt = Compare {
        repo: evt.pull_request.base.repo.unwrap().name,
        commits: [sha_base.to_owned(), sha_head.to_owned()],
        pr: evt.pull_request.number,
        check_id: None,
    };
    assert_eq!(recv.next().await, Some(evt.into()));
}
