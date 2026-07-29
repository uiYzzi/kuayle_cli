// HTTP client for the kuayle REST API.
// kuayle REST API 的 HTTP 客户端。
//
// All requests go through a single pipeline:
// 1. URL join (base_url + path)
// 2. Auth header injection (Bearer token)
// 3. Execute request with retry/rate-limit handling
// 4. Error mapping (non-2xx → KuayleError)
//
// 所有请求走同一条管线：
// 1. URL 拼接（base_url + path）
// 2. 认证头注入（Bearer token）
// 3. 带重试/限流处理执行请求
// 4. 错误映射（非 2xx → KuayleError）

use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use url::Url;

use crate::error::{ErrorBody, KuayleError};
use crate::retry::RetryPolicy;
use crate::session::Session;

/// Kuayle API client.
/// Kuayle API 客户端。
pub struct Client {
    http: reqwest::Client,
    base_url: Url,
    session: Arc<RwLock<Session>>,
    retry: RetryPolicy,
    user_agent: String,
}

impl Client {
    /// Build a new client from a base URL and PAT session.
    /// 从 base URL 和 PAT 会话构建新客户端。
    pub fn new(base_url: Url, token: String) -> Self {
        let version = env!("CARGO_PKG_VERSION");
        let session = Session::pat(token);
        Client {
            http: reqwest::Client::new(),
            base_url,
            session: Arc::new(RwLock::new(session)),
            retry: RetryPolicy::default(),
            user_agent: format!("kuayle-sdk/{version}"),
        }
    }

    /// Return a clone of the base URL.
    /// 返回 base URL 的克隆。
    pub fn base_url(&self) -> Url {
        self.base_url.clone()
    }

    /// Perform a GET request with retry.
    /// 执行带重试的 GET 请求。
    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, KuayleError> {
        self.execute_with_retry("GET", path, None::<&()>).await
    }

    /// Perform a POST request (no retry for non-idempotent).
    /// 执行 POST 请求（非幂等请求不重试）。
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, KuayleError> {
        self.execute("POST", path, Some(body)).await
    }

    /// Perform a DELETE request (no retry for non-idempotent).
    /// 执行 DELETE 请求（非幂等请求不重试）。
    pub async fn delete<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, KuayleError> {
        self.execute("DELETE", path, None::<&()>).await
    }

    // ── retry-aware execution ──────────────────────────────────────

    /// Execute a request with retry logic for idempotent methods.
    /// 对幂等方法执行带重试逻辑的请求。
    async fn execute_with_retry<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, KuayleError> {
        let mut attempt: u32 = 0;
        let max = self.retry.max_retries;
        let idempotent = RetryPolicy::is_idempotent(method);

        loop {
            let result = self.execute_once(method, path, body).await;

            match result {
                Ok(val) => return Ok(val),
                Err(KuayleError::RateLimited { retry_after }) => {
                    // Rate-limited: wait Retry-After, then retry up to max_rate_limit_retries.
                    // 被限流：等待 Retry-After，然后在 max_rate_limit_retries 内重试。
                    if attempt < self.retry.max_rate_limit_retries {
                        let wait = retry_after.unwrap_or(Duration::from_secs(5));
                        tokio::time::sleep(wait).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(KuayleError::RateLimited { retry_after });
                }
                Err(e @ KuayleError::Server { status, .. }) if idempotent && RetryPolicy::is_retryable_server_error(status) => {
                    if attempt < max {
                        let backoff = self.retry.backoff(attempt);
                        tokio::time::sleep(backoff).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(e);
                }
                Err(e @ KuayleError::Transport(_)) if idempotent => {
                    // Network error: retry if idempotent.
                    // 网络错误：幂等时重试。
                    if attempt < max {
                        let backoff = self.retry.backoff(attempt);
                        tokio::time::sleep(backoff).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(e);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Execute a single HTTP request (no retry).
    /// 执行单次 HTTP 请求（无重试）。
    async fn execute_once<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, KuayleError> {
        let url = self.build_url(path)?;
        let token = self.read_token().await;

        let mut req = match method {
            "GET" => self.http.get(url),
            "POST" => {
                let mut r = self.http.post(url);
                if let Some(b) = body {
                    r = r.json(b);
                }
                r
            }
            "DELETE" => self.http.delete(url),
            _ => {
                return Err(KuayleError::Api {
                    code: "INVALID_METHOD".into(),
                    message: format!("unsupported HTTP method: {method}"),
                });
            }
        };

        req = req
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(USER_AGENT, &self.user_agent);

        let resp = req.send().await?;
        self.handle_response(resp).await
    }

    /// Execute without retry (for non-idempotent methods).
    /// 无重试执行（非幂等方法）。
    async fn execute<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, KuayleError> {
        self.execute_once(method, path, body).await
    }

    // ── private helpers ────────────────────────────────────────────

    fn build_url(&self, path: &str) -> Result<Url, KuayleError> {
        self.base_url
            .join(path)
            .map_err(|e| KuayleError::Api {
                code: "INVALID_URL".into(),
                message: format!("failed to join URL: {e}"),
            })
    }

    async fn read_token(&self) -> String {
        let guard = self.session.read().await;
        guard.bearer_token().to_string()
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, KuayleError> {
        let status = resp.status().as_u16();

        // Parse Retry-After for 429 responses.
        // 解析 429 响应的 Retry-After。
        let retry_after = if status == 429 {
            parse_retry_after(resp.headers())
        } else {
            None
        };

        if status >= 200 && status < 300 {
            let body = resp.text().await.map_err(KuayleError::Transport)?;
            serde_json::from_str(&body).map_err(|e| KuayleError::Api {
                code: "DESERIALIZE_ERROR".into(),
                message: format!("failed to parse response: {e}"),
            })
        } else if status == 429 {
            Err(KuayleError::RateLimited { retry_after })
        } else {
            let body_text = resp.text().await.map_err(KuayleError::Transport)?;
            let error_body: ErrorBody = serde_json::from_str(&body_text).unwrap_or_else(|_| {
                ErrorBody {
                    error: crate::error::ErrorPayload {
                        code: "UNKNOWN".into(),
                        message: body_text,
                        details: vec![],
                    },
                }
            });
            Err(KuayleError::from_response(status, error_body))
        }
    }
}

/// Parse the `Retry-After` header value.
/// 解析 `Retry-After` 头值。
///
/// Returns seconds as Duration, or None if header is missing or unparseable.
/// 返回秒数 Duration，如果 header 缺失或无法解析则返回 None。
fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
}
