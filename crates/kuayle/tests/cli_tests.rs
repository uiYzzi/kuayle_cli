// CLI integration tests using assert_cmd + wiremock.
// 使用 assert_cmd + wiremock 的 CLI 集成测试。
//
// These tests run the actual kuayle binary against a wiremock server,
// with an isolated HOME to prevent interference with real config.
// 这些测试在隔离的 HOME 下对 wiremock 服务器运行真实的 kuayle 二进制文件。

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Set up a fake HOME directory with a config directory.
/// 设置带配置目录的虚拟 HOME 目录。
fn setup_home() -> TempDir {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join(".config").join("kuayle");
    fs::create_dir_all(&config_dir).unwrap();
    dir
}

/// Build a kuayle Command pointed at a fake HOME.
/// 构建指向虚拟 HOME 的 kuayle Command。
fn kuayle_cmd(home: &TempDir, server_uri: &str) -> Command {
    let mut cmd = Command::cargo_bin("kuayle").unwrap();
    cmd.env("HOME", home.path())
        .env("KUAYLE_URL", server_uri);
    cmd
}

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

// ── auth status ───────────────────────────────────────────────────

#[test]
fn auth_status_not_logged_in() {
    let home = setup_home();
    let mut cmd = Command::cargo_bin("kuayle").unwrap();
    cmd.env("HOME", home.path()).arg("auth").arg("status");

    cmd.assert()
        .failure()
        .code(predicate::eq(2))
        .stdout(predicate::str::contains("Not logged in"));
}

// ── auth login ────────────────────────────────────────────────────

#[tokio::test]
async fn auth_login_with_token_succeeds() {
    let home = setup_home();
    let server = MockServer::start().await;

    // Mock /api/auth/me to validate the token.
    // 模拟 /api/auth/me 验证 token。
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args([
        "auth", "login",
        "--token", "kuayle_pat_test123",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Logged in as Alice"));
}

#[tokio::test]
async fn auth_login_with_invalid_token_fails() {
    let home = setup_home();
    let server = MockServer::start().await;

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
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args([
        "auth", "login",
        "--token", "kuayle_pat_bad_token",
    ]);

    cmd.assert()
        .failure()
        .code(predicate::eq(2));
}

// ── auth status with session ──────────────────────────────────────

#[tokio::test]
async fn auth_status_after_login() {
    let home = setup_home();
    let server = MockServer::start().await;

    // Login first.
    // 先登录。
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args([
        "auth", "login",
        "--token", "kuayle_pat_test123",
    ]);
    cmd.assert().success();

    // Now check status — needs another mock for the status check.
    // 现在检查状态 — 需要另一个 mock 用于状态检查。
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args(["auth", "status"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Personal Access Token"))
        .stdout(predicate::str::contains("authenticated as Alice"));
}

// ── auth logout ───────────────────────────────────────────────────

#[tokio::test]
async fn auth_logout_removes_session() {
    let home = setup_home();
    let server = MockServer::start().await;

    // Login.
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args([
        "auth", "login",
        "--token", "kuayle_pat_test123",
    ]);
    cmd.assert().success();

    // Logout.
    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args(["auth", "logout"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Logged out"));

    // Verify status shows not logged in.
    let mut cmd = Command::cargo_bin("kuayle").unwrap();
    cmd.env("HOME", home.path()).arg("auth").arg("status");
    cmd.assert()
        .failure()
        .code(predicate::eq(2));
}

// ── whoami ────────────────────────────────────────────────────────

#[tokio::test]
async fn whoami_shows_user_info() {
    let home = setup_home();
    let server = MockServer::start().await;

    // Login first.
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args([
        "auth", "login",
        "--token", "kuayle_pat_test123",
    ]);
    cmd.assert().success();

    // whoami
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.arg("whoami");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alice Chen"))
        .stdout(predicate::str::contains("alice@kuayle.dev"));
}

#[tokio::test]
async fn whoami_not_logged_in_fails() {
    let home = setup_home();

    let mut cmd = Command::cargo_bin("kuayle").unwrap();
    cmd.env("HOME", home.path()).arg("whoami");

    cmd.assert()
        .failure()
        .code(predicate::eq(2));
}

// ── JSON output ───────────────────────────────────────────────────

#[tokio::test]
async fn whoami_json_output() {
    let home = setup_home();
    let server = MockServer::start().await;

    // Login.
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args([
        "auth", "login",
        "--token", "kuayle_pat_test123",
    ]);
    cmd.assert().success();

    // whoami --format json
    Mock::given(method("GET"))
        .and(path("/api/auth/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_json()))
        .expect(1)
        .mount(&server)
        .await;

    let mut cmd = kuayle_cmd(&home, &server.uri());
    cmd.args(["whoami", "--format", "json"]);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    // Should be valid JSON.
    // 应为有效 JSON。
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["email"], "alice@kuayle.dev");
}
