// HTTP client for the kuayle REST API.
// kuayle REST API 的 HTTP 客户端。
//
// All requests go through a single pipeline:
// 1. URL join (base_url + path)
// 2. Auth header injection (Bearer token)
// 3. Execute request
// 4. Error mapping (non-2xx → KuayleError)
// 5. Retry on transient failures (see Slice 5)
//
// 所有请求走同一条管线：
// 1. URL 拼接（base_url + path）
// 2. 认证头注入（Bearer token）
// 3. 执行请求
// 4. 错误映射（非 2xx → KuayleError）
// 5. 瞬时失败重试（见 Slice 5）

use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

use crate::error::{ErrorBody, KuayleError};
use crate::session::Session;

/// Kuayle API client.
/// Kuayle API 客户端。
///
/// All API calls go through this client. It handles auth injection
/// and error mapping. Retry/rate-limit is deferred to Slice 5.
/// 所有 API 调用都通过此客户端。它处理认证注入和错误映射。
/// 重试/限流延后到 Slice 5。
pub struct Client {
    http: reqwest::Client,
    base_url: Url,
    session: Arc<RwLock<Session>>,
    user_agent: String,
}

impl Client {
    /// Build a new client from a base URL and PAT session.
    /// 从 base URL 和 PAT 会话构建新客户端。
    ///
    /// `base_url` is the self-hosted kuayle instance URL
    /// (e.g. `http://localhost:5173`).
    /// `base_url` 是自托管 kuayle 实例的 URL（如 `http://localhost:5173`）。
    pub fn new(base_url: Url, token: String) -> Self {
        let version = env!("CARGO_PKG_VERSION");
        let session = Session::pat(token);
        Client {
            http: reqwest::Client::new(),
            base_url,
            session: Arc::new(RwLock::new(session)),
            user_agent: format!("kuayle-sdk/{version}"),
        }
    }

    /// Build a client with an existing session (for refresh support later).
    /// 用已有会话构建客户端（为后续刷新支持预留）。
    #[allow(dead_code)]
    pub(crate) fn with_session(base_url: Url, session: Arc<RwLock<Session>>) -> Self {
        let version = env!("CARGO_PKG_VERSION");
        Client {
            http: reqwest::Client::new(),
            base_url,
            session,
            user_agent: format!("kuayle-sdk/{version}"),
        }
    }

    /// Return a clone of the base URL.
    /// 返回 base URL 的克隆。
    pub fn base_url(&self) -> Url {
        self.base_url.clone()
    }

    /// Perform a GET request, deserializing the response body.
    /// 执行 GET 请求，反序列化响应体。
    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, KuayleError> {
        let url = self.build_url(path)?;
        let token = self.read_token().await;

        let resp = self
            .http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(USER_AGENT, &self.user_agent)
            .send()
            .await?;

        self.handle_response(resp).await
    }

    /// Perform a POST request with a JSON body.
    /// 执行带 JSON body 的 POST 请求。
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, KuayleError> {
        let url = self.build_url(path)?;
        let token = self.read_token().await;

        let resp = self
            .http
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(USER_AGENT, &self.user_agent)
            .json(body)
            .send()
            .await?;

        self.handle_response(resp).await
    }

    /// Perform a DELETE request.
    /// 执行 DELETE 请求。
    pub async fn delete<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, KuayleError> {
        let url = self.build_url(path)?;
        let token = self.read_token().await;

        let resp = self
            .http
            .delete(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(USER_AGENT, &self.user_agent)
            .send()
            .await?;

        self.handle_response(resp).await
    }

    // ── private helpers ────────────────────────────────────────────

    /// Build the full URL from base_url + path.
    /// 从 base_url + path 构建完整 URL。
    fn build_url(&self, path: &str) -> Result<Url, KuayleError> {
        // Path must start with / or be a relative reference.
        // path 必须以 / 开头或为相对引用。
        self.base_url
            .join(path)
            .map_err(|e| KuayleError::Api {
                code: "INVALID_URL".into(),
                message: format!("failed to join URL: {e}"),
            })
    }

    /// Read the current bearer token from session.
    /// 从会话读取当前 bearer token。
    async fn read_token(&self) -> String {
        let guard = self.session.read().await;
        guard.bearer_token().to_string()
    }

    /// Handle the HTTP response: success → deserialize, error → KuayleError.
    /// 处理 HTTP 响应：成功 → 反序列化，错误 → KuayleError。
    async fn handle_response<T: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, KuayleError> {
        let status = resp.status().as_u16();

        if status >= 200 && status < 300 {
            // Success: deserialize the body.
            // 成功：反序列化 body。
            let body = resp.text().await.map_err(KuayleError::Transport)?;
            serde_json::from_str(&body).map_err(|e| KuayleError::Api {
                code: "DESERIALIZE_ERROR".into(),
                message: format!("failed to parse response: {e}"),
            })
        } else {
            // Error: parse the error envelope, then map to KuayleError.
            // 错误：解析错误信封，然后映射为 KuayleError。
            let body_text = resp.text().await.map_err(KuayleError::Transport)?;
            let error_body: ErrorBody = serde_json::from_str(&body_text).unwrap_or_else(|_| {
                // If we can't parse the error envelope, create a generic one.
                // 如果无法解析错误信封，创建一个通用错误。
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
