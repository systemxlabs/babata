use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn read_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).expect("response should be valid JSON")
}

#[tokio::test]
async fn dashboard_root_serves_html_shell() {
    let app = babata::http::router_for_test();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::ACCEPT, "text/html")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.starts_with("text/html"),
        "expected text/html response; got content-type={content_type:?}"
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("Babata Dashboard"));
}

#[tokio::test]
async fn dashboard_tasks_route_serves_html_shell_without_limit_for_html_requests() {
    let app = babata::http::router_for_test();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tasks")
                .header(header::ACCEPT, "text/html")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("Babata Dashboard"));
}

#[tokio::test]
async fn dashboard_tasks_route_prefers_json_when_json_is_ranked_higher() {
    let app = babata::http::router_for_test();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tasks?limit=1")
                .header(header::ACCEPT, "application/json, text/html;q=0.5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.starts_with("application/json"),
        "expected JSON response; got content-type={content_type:?}"
    );
}

#[tokio::test]
async fn dashboard_tasks_route_keeps_json_when_html_is_not_acceptable() {
    let app = babata::http::router_for_test();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tasks?limit=1")
                .header(header::ACCEPT, "application/json, text/html;q=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.starts_with("application/json"),
        "expected JSON response; got content-type={content_type:?}"
    );
}

#[tokio::test]
async fn dashboard_tasks_route_preserves_json_validation_error_without_html() {
    let app = babata::http::router_for_test();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tasks")
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.starts_with("application/json"),
        "expected JSON response; got content-type={content_type:?}"
    );
}

#[tokio::test]
async fn dashboard_task_route_serves_html_shell_when_html_is_preferred() {
    let app = babata::http::router_for_test();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tasks/not-a-uuid")
                .header(header::ACCEPT, "text/html, application/json;q=0.8")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("Babata Dashboard"));
}

#[tokio::test]
async fn dashboard_task_route_preserves_json_errors_when_json_is_preferred() {
    let app = babata::http::router_for_test();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/tasks/not-a-uuid")
                .header(header::ACCEPT, "application/json, text/html;q=0.5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.starts_with("application/json"),
        "expected JSON response; got content-type={content_type:?}"
    );
}

#[tokio::test]
async fn overview_returns_status_counts_and_recent_tasks() {
    let app = babata::http::router_for_test();

    let create_task = |prompt_text: &str| {
        let app = app.clone();
        let prompt_text = prompt_text.to_string();
        async move {
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "prompt": [{ "type": "text", "text": prompt_text }],
                            "agent": "codex",
                            "never_ends": false
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap()
        }
    };

    let response = create_task("task-a").await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_a = read_json(response).await;
    let task_a = created_a
        .get("task_id")
        .and_then(Value::as_str)
        .expect("create response should include task_id")
        .to_string();

    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    let response = create_task("task-b").await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_b = read_json(response).await;
    let task_b = created_b
        .get("task_id")
        .and_then(Value::as_str)
        .expect("create response should include task_id")
        .to_string();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{task_a}/pause"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/overview")
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let overview = read_json(response).await;

    let counts = overview
        .get("status_counts")
        .expect("overview should include status_counts");
    assert_eq!(counts.get("running").and_then(Value::as_u64), Some(1));
    assert_eq!(counts.get("paused").and_then(Value::as_u64), Some(1));
    assert_eq!(counts.get("canceled").and_then(Value::as_u64), Some(0));
    assert_eq!(counts.get("done").and_then(Value::as_u64), Some(0));
    assert_eq!(counts.get("total").and_then(Value::as_u64), Some(2));

    let recent = overview
        .get("recent_tasks")
        .and_then(Value::as_array)
        .expect("overview should include recent_tasks array");
    assert!(!recent.is_empty(), "recent_tasks should not be empty");

    assert_eq!(
        recent[0].get("task_id").and_then(Value::as_str),
        Some(task_b.as_str())
    );
    assert!(
        recent[0]
            .get("actions")
            .and_then(Value::as_object)
            .is_some(),
        "task should include dashboard action availability"
    );
}

#[tokio::test]
async fn system_endpoint_returns_runtime_metadata() {
    let app = babata::http::router_for_test();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/system")
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let system = read_json(response).await;

    assert_eq!(
        system.get("version").and_then(Value::as_str),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        system.get("http_addr").and_then(Value::as_str),
        Some(babata::http::DEFAULT_HTTP_ADDR)
    );
}

#[tokio::test]
async fn api_tasks_supports_root_only_filter() {
    let app = babata::http::router_for_test();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "prompt": [{ "type": "text", "text": "root task" }],
                        "agent": "codex",
                        "never_ends": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_root = read_json(response).await;
    let root_task_id = created_root
        .get("task_id")
        .and_then(Value::as_str)
        .expect("create response should include task_id")
        .to_string();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "prompt": [{ "type": "text", "text": "child task" }],
                        "agent": "codex",
                        "parent_task_id": root_task_id,
                        "never_ends": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/tasks?root_only=true")
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let list = read_json(response).await;
    let tasks = list
        .get("tasks")
        .and_then(Value::as_array)
        .expect("response should include tasks");

    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0].get("task_id").and_then(Value::as_str),
        Some(root_task_id.as_str())
    );
    assert!(tasks[0].get("parent_task_id").is_some());
    assert!(tasks[0].get("actions").and_then(Value::as_object).is_some());
}

#[tokio::test]
async fn api_tasks_supports_root_task_id_filter() {
    let app = babata::http::router_for_test();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "prompt": [{ "type": "text", "text": "root task" }],
                        "agent": "codex",
                        "never_ends": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_root = read_json(response).await;
    let root_task_id = created_root
        .get("task_id")
        .and_then(Value::as_str)
        .expect("create response should include task_id")
        .to_string();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "prompt": [{ "type": "text", "text": "child task" }],
                        "agent": "codex",
                        "parent_task_id": root_task_id,
                        "never_ends": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "prompt": [{ "type": "text", "text": "other root" }],
                        "agent": "codex",
                        "never_ends": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks?root_task_id={root_task_id}"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let list = read_json(response).await;
    let tasks = list
        .get("tasks")
        .and_then(Value::as_array)
        .expect("response should include tasks");

    assert_eq!(tasks.len(), 2);
    assert!(tasks.iter().all(|task| {
        task.get("root_task_id").and_then(Value::as_str) == Some(root_task_id.as_str())
    }));
}
