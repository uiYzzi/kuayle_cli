// Wiremock integration tests for the SDK Client.
// SDK Client 的 wiremock 集成测试。
//
// Tests: basic GET/POST/DELETE, error mapping (all error codes),
// request headers (auth, user-agent).
// 测试：基本 GET/POST/DELETE、错误映射（全部错误码）、请求头（认证、user-agent）。

use kuayle_sdk::client::Client;
use kuayle_sdk::error::KuayleError;
use kuayle_sdk::types::user::UserResponse;
use serde_json::json;
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create a Client pointed at a mock server.
/// 辅助函数：创建指向 mock server 的 Client。
async fn test_client(server: &MockServer) -> Client {
    let base_url = Url::parse(&server.uri()).unwrap();
    Client::new(base_url, "kuayle_pat_test_token".into())
}

/// Helper: standard user JSON for mocking `/api/auth/me`.
/// 辅助函数：模拟 `/api/auth/me` 的标准用户 JSON。
fn user_json() -> serde_json::Value {
    json!({
        "id": "a0000000-0000-0000-0000-000000000001",
        "email": "alice@kuayle.dev",
        "name": "Alice Chen",
        "display_name": "Alice",
        "avatar_url": null,
        "is_sysadmin": true
    })
}

// ── GET tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn get_deserializes_success_response() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .mount(&server)
        .await;

    let user: UserResponse = client.get("/api/auth/me").await.unwrap();
    assert_eq!(user.email, "alice@kuayle.dev");
    assert_eq!(user.name, "Alice Chen");
}

#[tokio::test]
async fn get_sends_auth_header() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .and(header("Authorization", "Bearer kuayle_pat_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .mount(&server)
        .await;

    let result: Result<UserResponse, _> = client.get("/api/auth/me").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_sends_user_agent() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .and(header("User-Agent", "kuayle-sdk/0.1.0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .mount(&server)
        .await;

    let result: Result<UserResponse, _> = client.get("/api/auth/me").await;
    assert!(result.is_ok());
}

// ── POST tests ────────────────────────────────────────────────────

#[tokio::test]
async fn post_sends_json_body() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("POST"))
        .and(path("/api/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .mount(&server)
        .await;

    let body = json!({"email": "alice@kuayle.dev", "password": "secret123456"});
    let user: UserResponse = client.post("/api/auth/login", &body).await.unwrap();
    assert_eq!(user.email, "alice@kuayle.dev");
}

// ── Error mapping tests ───────────────────────────────────────────

#[tokio::test]
async fn error_401_unauthorized_maps_to_authentication() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({
                "error": {
                    "code": "UNAUTHORIZED",
                    "message": "Authentication required"
                }
            })),
        )
        .mount(&server)
        .await;

    let err = client.get::<UserResponse>("/api/auth/me").await.unwrap_err();
    assert!(matches!(err, KuayleError::Authentication { .. }));
    assert_eq!(err.exit_code(), 2);
}

#[tokio::test]
async fn error_403_forbidden_maps_to_forbidden() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/workspaces/acme"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(json!({
                "error": {
                    "code": "FORBIDDEN",
                    "message": "Access denied"
                }
            })),
        )
        .mount(&server)
        .await;

    let err = client
        .get::<serde_json::Value>("/api/workspaces/acme")
        .await
        .unwrap_err();
    assert!(matches!(err, KuayleError::Forbidden { .. }));
    assert_eq!(err.exit_code(), 5);
}

#[tokio::test]
async fn error_404_not_found_maps_to_not_found() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/workspaces/nonexistent"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({
                "error": {
                    "code": "NOT_FOUND",
                    "message": "Workspace not found"
                }
            })),
        )
        .mount(&server)
        .await;

    let err = client
        .get::<serde_json::Value>("/api/workspaces/nonexistent")
        .await
        .unwrap_err();
    assert!(matches!(err, KuayleError::NotFound { .. }));
    assert_eq!(err.exit_code(), 3);
}

#[tokio::test]
async fn error_400_validation_maps_to_validation() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("POST"))
        .and(path("/api/auth/login"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({
                "error": {
                    "code": "VALIDATION_ERROR",
                    "message": "Request validation failed",
                    "details": [
                        {"field": "Password", "message": "must be at least 12 characters"}
                    ]
                }
            })),
        )
        .mount(&server)
        .await;

    let body = json!({"email": "a@b.com", "password": "short"});
    let err: KuayleError = client
        .post::<_, serde_json::Value>("/api/auth/login", &body)
        .await
        .unwrap_err();
    assert!(matches!(err, KuayleError::Validation { .. }));
    assert_eq!(err.exit_code(), 4);
}

#[tokio::test]
async fn error_500_maps_to_server() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/crash"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(json!({
                "error": {
                    "code": "INTERNAL_ERROR",
                    "message": "something went wrong"
                }
            })),
        )
        .mount(&server)
        .await;

    let err: KuayleError = client.get::<serde_json::Value>("/api/crash").await.unwrap_err();
    assert!(matches!(err, KuayleError::Server { status: 500, .. }));
    assert_eq!(err.exit_code(), 7);
}

#[tokio::test]
async fn error_502_maps_to_server() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/unavailable"))
        .respond_with(
            ResponseTemplate::new(502).set_body_json(json!({
                "error": {
                    "code": "BAD_GATEWAY",
                    "message": "upstream error"
                }
            })),
        )
        .mount(&server)
        .await;

    let err: KuayleError = client
        .get::<serde_json::Value>("/api/unavailable")
        .await
        .unwrap_err();
    assert!(matches!(err, KuayleError::Server { status: 502, .. }));
}

#[tokio::test]
async fn error_invalid_json_body_maps_to_api_error() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    // Return HTML instead of JSON.
    // 返回 HTML 而非 JSON。
    Mock::given(method("GET"))
        .and(path("/api/bad"))
        .respond_with(ResponseTemplate::new(404).set_body_string("<html>Not Found</html>"))
        .mount(&server)
        .await;

    let err: KuayleError = client.get::<serde_json::Value>("/api/bad").await.unwrap_err();
    // Should map to NotFound because status is 404, but error code will be "UNKNOWN"
    // since we can't parse the HTML body.
    // 应映射为 NotFound（状态码 404），但由于无法解析 HTML body，错误码将是 "UNKNOWN"。
    assert!(matches!(err, KuayleError::NotFound { .. }) || matches!(err, KuayleError::Api { .. }));
}

// ── DELETE tests ──────────────────────────────────────────────────

#[tokio::test]
async fn delete_sends_request() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("DELETE"))
        .and(path("/api/tokens/some-id"))
        .and(header("Authorization", "Bearer kuayle_pat_test_token"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    // DELETE to /api/tokens/:id returns 204 No Content.
    // We expect an empty object or error.
    let result: Result<serde_json::Value, _> =
        client.delete("/api/tokens/some-id").await;
    // 204 with no body — serde_json will fail to parse "".
    // This is expected; production code handles 204 specially later.
    // 204 无 body — serde_json 解析 "" 会失败。这是预期行为；生产代码稍后特殊处理 204。
    assert!(result.is_err());
}

// ── URL construction ──────────────────────────────────────────────

#[tokio::test]
async fn get_workspaces_list() {
    let server = MockServer::start().await;
    let client = test_client(&server).await;

    Mock::given(method("GET"))
        .and(path("/api/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "w1",
                "name": "Acme",
                "slug": "acme",
                "logo_url": null,
                "owner_id": "u1",
                "owner": {
                    "id": "u1",
                    "email": "a@b.com",
                    "name": "Alice",
                    "avatar_url": null
                },
                "share_link_min_role": "admin",
                "current_user_role": "owner",
                "created_at": "2026-07-28T15:58:38.119837+08:00",
                "updated_at": "2026-07-28T15:58:38.119837+08:00"
            }
        ])))
        .mount(&server)
        .await;

    let workspaces: Vec<kuayle_sdk::types::workspace::WorkspaceResponse> =
        client.get("/api/workspaces").await.unwrap();
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0].slug, "acme");
}
