use axum::Router;
use reqwest::Client;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

/// Spin up a mock upstream that returns a fixed response
async fn start_mock_upstream(status: u16, body: &'static str) -> String {
    let app = Router::new().route(
        "/{*path}",
        axum::routing::any(move || async move {
            (
                axum::http::StatusCode::from_u16(status).unwrap(),
                body.to_string(),
            )
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{}", addr)
}

fn write_file(dir: &std::path::Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

/// Build Guard Rail app with test config, bind to a port, return the base URL
async fn start_test_app(upstream_url: &str) -> (String, TempDir) {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path();

    write_file(
        config_dir,
        "routes.yaml",
        &format!(
            r#"
routes:
  - id: test-route
    path: /v1/execute/test-route
    upstream: {upstream_url}/api/target
    methods: [POST]
    policies: [block-callbacks, size-limit]
    timeout_ms: 5000
  - id: open-route
    path: /v1/execute/open-route
    upstream: {upstream_url}/api/open
    methods: [POST, PUT]
    policies: []
    timeout_ms: 5000
"#
        ),
    );

    let policies_dir = config_dir.join("policies");
    std::fs::create_dir_all(&policies_dir).unwrap();
    write_file(
        &policies_dir,
        "test.yaml",
        r#"
policies:
  - name: block-callbacks
    rules:
      - field: "$.callback"
        condition: domain_not_in
        values: ["*.safe.com"]
        action: block
        severity: critical
  - name: size-limit
    rules:
      - field: "$"
        condition: size_exceeds
        max_bytes: 10240
        action: block
"#,
    );

    let routes_path = config_dir.join("routes.yaml");
    let route_table = guard_rail_engine::routes::RouteTable::load(&routes_path).unwrap();
    let policy_set = guard_rail_engine::policy::PolicySet::load_dir(&policies_dir).unwrap();

    let state = guard_rail_engine::proxy::AppState {
        routes: Arc::new(RwLock::new(route_table)),
        policies: Arc::new(RwLock::new(policy_set)),
        http_client: Client::new(),
    };

    let app = Router::new()
        .route(
            "/v1/execute/{route_id}",
            axum::routing::any(guard_rail_engine::proxy::handle_execute),
        )
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });

    (format!("http://{}", addr), tmp)
}

#[tokio::test]
async fn test_allow_path_forwards_to_upstream() {
    let upstream = start_mock_upstream(200, r#"{"result":"ok"}"#).await;
    let (base_url, _tmp) = start_test_app(&upstream).await;

    let client = Client::new();
    let resp = client
        .post(format!("{}/v1/execute/open-route", base_url))
        .header("content-type", "application/json")
        .body(r#"{"data": "hello"}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("ok"));
}

#[tokio::test]
async fn test_block_path_returns_403() {
    let upstream = start_mock_upstream(200, "ok").await;
    let (base_url, _tmp) = start_test_app(&upstream).await;

    let client = Client::new();
    let resp = client
        .post(format!("{}/v1/execute/test-route", base_url))
        .header("content-type", "application/json")
        .body(r#"{"callback": "https://evil.sh/exfil", "amount": 100}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
    let body = resp.text().await.unwrap();
    assert!(body.contains("blocked"));
    assert!(body.contains("block-callbacks"));
}

#[tokio::test]
async fn test_allow_when_policy_passes() {
    let upstream = start_mock_upstream(200, r#"{"forwarded": true}"#).await;
    let (base_url, _tmp) = start_test_app(&upstream).await;

    let client = Client::new();
    let resp = client
        .post(format!("{}/v1/execute/test-route", base_url))
        .header("content-type", "application/json")
        .body(r#"{"callback": "https://api.safe.com/hook", "amount": 100}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("forwarded"));
}

#[tokio::test]
async fn test_invalid_json_returns_400() {
    let upstream = start_mock_upstream(200, "ok").await;
    let (base_url, _tmp) = start_test_app(&upstream).await;

    let client = Client::new();
    let resp = client
        .post(format!("{}/v1/execute/test-route", base_url))
        .header("content-type", "application/json")
        .body("this is not json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body = resp.text().await.unwrap();
    assert!(body.contains("rejected"));
}

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let upstream = start_mock_upstream(200, "ok").await;
    let (base_url, _tmp) = start_test_app(&upstream).await;

    let client = Client::new();
    let resp = client
        .post(format!("{}/v1/execute/nonexistent", base_url))
        .header("content-type", "application/json")
        .body(r#"{"data": 1}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_wrong_method_returns_405() {
    let upstream = start_mock_upstream(200, "ok").await;
    let (base_url, _tmp) = start_test_app(&upstream).await;

    let client = Client::new();
    let resp = client
        .put(format!("{}/v1/execute/test-route", base_url))
        .header("content-type", "application/json")
        .body(r#"{"data": 1}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 405);
    let body = resp.text().await.unwrap();
    assert!(body.contains("rejected"));
}
