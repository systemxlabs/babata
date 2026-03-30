use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

async fn read_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).expect("response should be valid JSON")
}

async fn create_task(
    app: axum::Router,
    prompt: &str,
    agent: Option<&str>,
    parent_task_id: Option<&str>,
) -> String {
    let mut payload = json!({
        "prompt": [{ "type": "text", "text": prompt }],
        "never_ends": false,
    });

    if let Some(agent) = agent {
        payload["agent"] = Value::String(agent.to_string());
    }

    if let Some(parent_task_id) = parent_task_id {
        payload["parent_task_id"] = Value::String(parent_task_id.to_string());
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    read_json(response)
        .await
        .get("task_id")
        .and_then(Value::as_str)
        .expect("create response should include task_id")
        .to_string()
}

fn write_task_file(task_id: &str, relative_path: &str, contents: &str) {
    let task_id = Uuid::parse_str(task_id).expect("task id should be valid UUID");
    let path = babata::task::task_dir(task_id)
        .expect("resolve task dir")
        .join(relative_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create task subdirectory");
    }
    std::fs::write(path, contents).expect("write task file");
}

fn cleanup_task_dir(task_id: &str) {
    let task_id = Uuid::parse_str(task_id).expect("task id should be valid UUID");
    let path = babata::task::task_dir(task_id).expect("resolve task dir");
    let _ = std::fs::remove_dir_all(path);
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
async fn dashboard_shell_assets_are_served_from_embedded_bundle() {
    let app = babata::http::router_for_test();

    let shell_response = app
        .clone()
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

    assert_eq!(shell_response.status(), StatusCode::OK);

    let shell_body = to_bytes(shell_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let shell_body = String::from_utf8(shell_body.to_vec()).unwrap();

    let script_path = shell_body
        .lines()
        .find_map(|line| {
            let marker = "src=\"";
            let start = line.find(marker)? + marker.len();
            let end = line[start..].find('"')?;
            Some(line[start..start + end].to_string())
        })
        .expect("dashboard shell should reference a script asset");

    let asset_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&script_path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(asset_response.status(), StatusCode::OK);

    let content_type = asset_response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.starts_with("text/javascript")
            || content_type.starts_with("application/javascript"),
        "expected JavaScript asset; got content-type={content_type:?}"
    );
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
async fn dashboard_create_and_system_routes_serve_html_shell() {
    let app = babata::http::router_for_test();

    for route in ["/create", "/system"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(route)
                    .header(header::ACCEPT, "text/html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "route {route} should serve the shell"
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body.contains("Babata Dashboard"),
            "route {route} should return the dashboard shell"
        );
    }
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

#[tokio::test]
async fn task_content_returns_task_and_progress_markdown() {
    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "content task", Some("codex"), None).await;

    write_task_file(&task_id, "task.md", "# Task\n\ncustom task body\n");
    write_task_file(
        &task_id,
        "progress.md",
        "# Progress\n\ncustom progress body\n",
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks/{task_id}/content"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;

    assert_eq!(
        body.get("task_id").and_then(Value::as_str),
        Some(task_id.as_str())
    );
    assert_eq!(
        body.get("task_markdown").and_then(Value::as_str),
        Some("# Task\n\ncustom task body\n")
    );
    assert_eq!(
        body.get("progress_markdown").and_then(Value::as_str),
        Some("# Progress\n\ncustom progress body\n")
    );

    cleanup_task_dir(&task_id);
}

#[tokio::test]
async fn task_tree_returns_recursive_root_hierarchy() {
    let app = babata::http::router_for_test();
    let root_task_id = create_task(app.clone(), "root task", Some("codex"), None).await;
    let child_task_id = create_task(
        app.clone(),
        "child task",
        Some("codex"),
        Some(&root_task_id),
    )
    .await;
    let grandchild_task_id = create_task(
        app.clone(),
        "grandchild task",
        Some("codex"),
        Some(&child_task_id),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks/{child_task_id}/tree"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;

    assert_eq!(
        body.get("root_task_id").and_then(Value::as_str),
        Some(root_task_id.as_str())
    );
    assert_eq!(
        body.get("parent")
            .and_then(|value| value.get("task_id"))
            .and_then(Value::as_str),
        Some(root_task_id.as_str())
    );
    assert_eq!(
        body.get("current")
            .and_then(|value| value.get("task_id"))
            .and_then(Value::as_str),
        Some(child_task_id.as_str())
    );
    assert_eq!(
        body.get("children")
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        Some(1)
    );
    assert_eq!(
        body.get("children")
            .and_then(Value::as_array)
            .and_then(|children| children.first())
            .and_then(|child| child.get("task_id"))
            .and_then(Value::as_str),
        Some(grandchild_task_id.as_str())
    );

    let root = body
        .get("root")
        .and_then(Value::as_object)
        .expect("tree response should include root object");
    assert_eq!(
        root.get("task")
            .and_then(|task| task.get("task_id"))
            .and_then(Value::as_str),
        Some(root_task_id.as_str())
    );
    let children = root
        .get("children")
        .and_then(Value::as_array)
        .expect("tree root should include children array");
    assert_eq!(children.len(), 1);
    assert_eq!(
        children[0]
            .get("task")
            .and_then(|task| task.get("task_id"))
            .and_then(Value::as_str),
        Some(child_task_id.as_str())
    );

    let grandchild_nodes = children[0]
        .get("children")
        .and_then(Value::as_array)
        .expect("child node should include grandchildren array");
    assert_eq!(grandchild_nodes.len(), 1);
    assert_eq!(
        grandchild_nodes[0]
            .get("task")
            .and_then(|task| task.get("task_id"))
            .and_then(Value::as_str),
        Some(grandchild_task_id.as_str())
    );

    cleanup_task_dir(&grandchild_task_id);
    cleanup_task_dir(&child_task_id);
    cleanup_task_dir(&root_task_id);
}

#[tokio::test]
async fn task_artifacts_returns_file_list() {
    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact task", Some("codex"), None).await;

    write_task_file(&task_id, "artifacts/summary.txt", "artifact body");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks/{task_id}/artifacts"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let artifacts = body
        .get("artifacts")
        .and_then(Value::as_array)
        .expect("artifact response should include artifacts");

    assert_eq!(artifacts.len(), 1);
    assert_eq!(
        artifacts[0].get("path").and_then(Value::as_str),
        Some("summary.txt")
    );
    assert_eq!(
        artifacts[0].get("is_text").and_then(Value::as_bool),
        Some(true)
    );

    cleanup_task_dir(&task_id);
}

#[tokio::test]
async fn task_artifact_content_returns_text_preview_for_selected_file() {
    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact content task", Some("codex"), None).await;

    write_task_file(
        &task_id,
        "artifacts/notes/output.md",
        "# Output\n\nartifact body",
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/tasks/{task_id}/artifacts/content?path=notes/output.md"
                ))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;

    assert_eq!(
        body.get("path").and_then(Value::as_str),
        Some("notes/output.md")
    );
    assert_eq!(body.get("is_text").and_then(Value::as_bool), Some(true));

    cleanup_task_dir(&task_id);
}

#[tokio::test]
async fn task_artifact_content_returns_unsupported_state_for_missing_file() {
    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact content task", Some("codex"), None).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/tasks/{task_id}/artifacts/content?path=notes/missing.md"
                ))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;

    assert_eq!(
        body.get("path").and_then(Value::as_str),
        Some("notes/missing.md")
    );
    assert_eq!(body.get("is_text").and_then(Value::as_bool), Some(false));
    assert!(body.get("content").is_some());
    assert_eq!(body.get("content").and_then(Value::as_str), None);
    assert!(
        body.get("reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("not found"))
    );

    cleanup_task_dir(&task_id);
}

#[cfg(unix)]
#[tokio::test]
async fn task_artifact_content_blocks_symlink_escape() {
    use std::os::unix::fs::symlink;

    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact symlink task", Some("codex"), None).await;
    let task_uuid = Uuid::parse_str(&task_id).expect("task id should be valid UUID");
    let task_dir = babata::task::task_dir(task_uuid).expect("resolve task dir");
    let artifacts_dir = task_dir.join("artifacts");
    std::fs::create_dir_all(&artifacts_dir).expect("create artifacts dir");

    let outside_path = task_dir.join("outside-secret.txt");
    std::fs::write(&outside_path, "secret").expect("write outside file");
    symlink(&outside_path, artifacts_dir.join("secret-link.txt")).expect("create artifact symlink");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/tasks/{task_id}/artifacts/content?path=secret-link.txt"
                ))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert_eq!(body.get("is_text").and_then(Value::as_bool), Some(false));
    assert!(
        body.get("reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("symlink") || reason.contains("outside"))
    );

    cleanup_task_dir(&task_id);
}

#[cfg(unix)]
#[tokio::test]
async fn task_artifacts_skip_symlink_entries() {
    use std::os::unix::fs::symlink;

    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact listing task", Some("codex"), None).await;
    let task_uuid = Uuid::parse_str(&task_id).expect("task id should be valid UUID");
    let task_dir = babata::task::task_dir(task_uuid).expect("resolve task dir");
    let artifacts_dir = task_dir.join("artifacts");
    std::fs::create_dir_all(&artifacts_dir).expect("create artifacts dir");

    std::fs::write(artifacts_dir.join("summary.txt"), "artifact body").expect("write artifact");
    let outside_path = task_dir.join("outside-secret.txt");
    std::fs::write(&outside_path, "secret").expect("write outside file");
    symlink(&outside_path, artifacts_dir.join("secret-link.txt")).expect("create artifact symlink");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks/{task_id}/artifacts"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let artifacts = body
        .get("artifacts")
        .and_then(Value::as_array)
        .expect("artifact response should include artifacts");

    assert_eq!(artifacts.len(), 1);
    assert_eq!(
        artifacts[0].get("path").and_then(Value::as_str),
        Some("summary.txt")
    );

    cleanup_task_dir(&task_id);
}

#[cfg(unix)]
#[tokio::test]
async fn task_artifacts_reject_symlinked_artifact_root() {
    use std::os::unix::fs::symlink;

    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact listing task", Some("codex"), None).await;
    let task_uuid = Uuid::parse_str(&task_id).expect("task id should be valid UUID");
    let task_dir = babata::task::task_dir(task_uuid).expect("resolve task dir");
    let artifacts_dir = task_dir.join("artifacts");
    let _ = std::fs::remove_dir_all(&artifacts_dir);

    let outside_dir = task_dir.join("outside-artifacts");
    std::fs::create_dir_all(&outside_dir).expect("create outside dir");
    std::fs::write(outside_dir.join("secret.txt"), "secret").expect("write outside artifact");
    symlink(&outside_dir, &artifacts_dir).expect("replace artifacts dir with symlink");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks/{task_id}/artifacts"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cleanup_task_dir(&task_id);
}

#[cfg(unix)]
#[tokio::test]
async fn task_artifacts_reject_broken_symlinked_artifact_root() {
    use std::os::unix::fs::symlink;

    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact listing task", Some("codex"), None).await;
    let task_uuid = Uuid::parse_str(&task_id).expect("task id should be valid UUID");
    let task_dir = babata::task::task_dir(task_uuid).expect("resolve task dir");
    let artifacts_dir = task_dir.join("artifacts");
    let _ = std::fs::remove_dir_all(&artifacts_dir);

    let missing_dir = task_dir.join("missing-artifacts");
    symlink(&missing_dir, &artifacts_dir).expect("replace artifacts dir with broken symlink");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks/{task_id}/artifacts"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cleanup_task_dir(&task_id);
}

#[cfg(unix)]
#[tokio::test]
async fn task_artifact_content_rejects_symlinked_artifact_root() {
    use std::os::unix::fs::symlink;

    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact content task", Some("codex"), None).await;
    let task_uuid = Uuid::parse_str(&task_id).expect("task id should be valid UUID");
    let task_dir = babata::task::task_dir(task_uuid).expect("resolve task dir");
    let artifacts_dir = task_dir.join("artifacts");
    let _ = std::fs::remove_dir_all(&artifacts_dir);

    let outside_dir = task_dir.join("outside-artifacts");
    std::fs::create_dir_all(&outside_dir).expect("create outside dir");
    std::fs::write(outside_dir.join("secret.txt"), "secret").expect("write outside artifact");
    symlink(&outside_dir, &artifacts_dir).expect("replace artifacts dir with symlink");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/tasks/{task_id}/artifacts/content?path=secret.txt"
                ))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cleanup_task_dir(&task_id);
}

#[cfg(unix)]
#[tokio::test]
async fn task_artifact_content_rejects_broken_symlinked_artifact_root() {
    use std::os::unix::fs::symlink;

    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "artifact content task", Some("codex"), None).await;
    let task_uuid = Uuid::parse_str(&task_id).expect("task id should be valid UUID");
    let task_dir = babata::task::task_dir(task_uuid).expect("resolve task dir");
    let artifacts_dir = task_dir.join("artifacts");
    let _ = std::fs::remove_dir_all(&artifacts_dir);

    let missing_dir = task_dir.join("missing-artifacts");
    symlink(&missing_dir, &artifacts_dir).expect("replace artifacts dir with broken symlink");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/tasks/{task_id}/artifacts/content?path=secret.txt"
                ))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cleanup_task_dir(&task_id);
}

#[tokio::test]
async fn task_logs_returns_unsupported_state_when_no_files_exist() {
    let app = babata::http::router_for_test();
    let task_id = create_task(app.clone(), "logless task", Some("codex"), None).await;
    let task_uuid = Uuid::parse_str(&task_id).expect("task id should be valid UUID");
    let task_dir = babata::task::task_dir(task_uuid).expect("resolve task dir");
    let _ = std::fs::remove_file(task_dir.join("codex-last-message.md"));
    let _ = std::fs::remove_file(task_dir.join("codex-stdout.log"));
    let _ = std::fs::remove_file(task_dir.join("codex-stderr.log"));

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks/{task_id}/logs"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;

    assert_eq!(body.get("supported").and_then(Value::as_bool), Some(false));
    assert!(
        body.get("reason")
            .and_then(Value::as_str)
            .expect("logs response should include reason")
            .contains("No known log files")
    );

    cleanup_task_dir(&task_id);
}
