use axum::{body::Body, extract::Request, http::StatusCode, routing::post, Router};
use axum_github_webhook_extract::GithubToken as GitHubSecret;
use futures::{
    channel::mpsc::{channel, Receiver},
    StreamExt,
};
use hmac_sha256::HMAC;
use http_body_util::BodyExt;
use octocrab::{models::repos::Ref, Octocrab};
use serde_json::json;
use std::sync::Arc;
use tower::util::ServiceExt;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use crate::{
    cli::RunBenchmark,
    event::{Compare, Event, PullRequestEvent, ORG},
    fixtures::PR,
};

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
    pub async fn setup_error_handler(mock_server: &MockServer, message: &str) {
        Mock::given(method("GET"))
            .and(path_regex(".*"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!( {
                "documentation_url": "",
                "errors": None::<Vec<serde_json::Value>>,
                "message": message,
            })))
            .mount(mock_server)
            .await;
    }
}

use mock_error::setup_error_handler;

const TEST_SECRET: &str = "It's a Secret to Everybody";

async fn setup_github_api(template: Option<ResponseTemplate>) -> MockServer {
    let mock_server = MockServer::start().await;
    if let Some(template) = template {
        let uri = format!("/repos/{ORG}/anndata/git/ref/f88f7bd4250b963752d615e491b7e676ce5eb7f0");
        Mock::given(method("GET"))
            .and(path(&uri))
            .respond_with(template)
            .mount(&mock_server)
            .await;
        setup_error_handler(&mock_server, &format!("GET on {uri} was not received")).await;
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

#[tokio::test]
async fn should_error_on_invalid_signature() {
    let (app, mut recv) = app(None).await;
    let request = make_webhook_request(PR, false);
    let res = app.oneshot(request).await.unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "{res:?}");
    assert_eq!(&body_string(res.into_body()).await, "signature mismatch");
    assert!(recv.next().await.is_none());
}

#[tokio::test]
async fn should_error_on_invalid_event_payload() {
    let (app, mut recv) = app(None).await;
    let request = make_webhook_request("{}", true);
    let res = app.oneshot(request).await.unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "{res:?}");
    assert_eq!(
        &body_string(res.into_body()).await,
        "missing field `number` at line 1 column 2"
    );
    assert!(recv.next().await.is_none());
}

#[tokio::test]
async fn should_skip_on_no_label() {
    let (app, mut recv) = app(None).await;
    let request = make_webhook_request(PR, true);
    let res = app.oneshot(request).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK, "{res:?}");
    assert_eq!(body_string(res.into_body()).await, "skipped");
    assert!(recv.next().await.is_none());
}

#[tokio::test]
async fn should_enqueue_valid_pr_event() {
    // expected event payload
    let sha_base = "a4786471ee4d4e894fec150e426c3551db0f31e0";
    let sha_after = "f88f7bd4250b963752d615e491b7e676ce5eb7f0";
    let config_ref: Ref = serde_json::from_value(json!({
        "ref": sha_after.to_owned(),
        "node_id": "xyz".to_owned(),
        "url": format!("https://api.github.com/repos/scverse/anndata/ref/{sha_after}"),
        "object": {
            "type": "commit",
            "sha": sha_after.to_owned(),
            "url": format!("https://api.github.com/repos/scverse/anndata/commits/{sha_after}"),
        },
    }))
    .unwrap();
    let template = ResponseTemplate::new(200).set_body_json(config_ref);
    let (app, mut recv) = app(Some(template)).await;
    let mut body: PullRequestEvent = serde_json::from_str(PR).unwrap();
    // the test data has no “benchmark” label, add one:
    body.pull_request.labels.as_mut().unwrap().push(
        serde_json::from_value(json!({
            "id": 2_532_885_704_u64,
            "node_id": "MDU6TGFiZWwyNTMyODg1N5Ax",
            "name": "benchmark",
            "description": "Allow benchmark runs for PRs marked with this label.",
            "color": "f1c40f",
            "url": "https://api.github.com/repos/scverse/anndata/labels/benchmark",
            "default": false,
        }))
        .unwrap(),
    );
    let request = make_webhook_request(serde_json::to_string(&body).unwrap(), true);
    let res = app.oneshot(request).await.unwrap();

    //assert_eq!(res.status(), StatusCode::OK, "{res:?}");
    assert_eq!(body_string(res.into_body()).await, "enqueued");
    let run_benchmark = RunBenchmark {
        repo: "anndata".to_owned(),
        config_ref: Some(sha_after.to_owned()),
        run_on: vec![
            // pull request base, not `before`
            sha_base.to_owned(),
            sha_after.to_owned(),
        ],
    };
    let evt = Compare {
        run_benchmark,
        pr: body.pull_request.number,
    };
    assert_eq!(recv.next().await, Some(evt.into()));
}
