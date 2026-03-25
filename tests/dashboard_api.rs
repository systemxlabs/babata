use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

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
