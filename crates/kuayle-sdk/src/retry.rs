// Retry policy for transient failures.
// 瞬时失败的重试策略。

use std::time::Duration;

/// Retry configuration for the SDK client.
/// SDK 客户端的重试配置。
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum retries for network/5xx errors.
    /// 网络/5xx 错误的最大重试次数。
    pub max_retries: u32,
    /// Maximum retries for 429 rate-limit errors.
    /// 429 限流错误的最大重试次数。
    pub max_rate_limit_retries: u32,
    /// Base backoff duration (doubles each retry).
    /// 基础退避时长（每次重试翻倍）。
    pub base_backoff: Duration,
    /// Maximum backoff duration cap.
    /// 最大退避时长上限。
    pub max_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_retries: 3,
            max_rate_limit_retries: 2,
            base_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// Compute the backoff duration for retry attempt `n` (0-indexed).
    /// 计算第 `n` 次重试（从 0 开始）的退避时长。
    ///
    /// Uses exponential backoff with ±20% jitter.
    /// 使用带 ±20% 抖动的指数退避。
    pub fn backoff(&self, attempt: u32) -> Duration {
        let base = self.base_backoff.as_millis() as u64;
        let exponential = base * (1u64 << attempt.min(10)); // cap at 2^10 to avoid overflow
        let capped = exponential.min(self.max_backoff.as_millis() as u64);

        // ±20% jitter
        // ±20% 抖动
        let jitter_range = (capped / 5) as i64;
        let jitter = if jitter_range > 0 {
            // Deterministic-ish jitter based on attempt for testability.
            // 基于 attempt 的确定性抖动以便测试。
            ((attempt as i64 * 17 + 3) % (jitter_range * 2 + 1)) - jitter_range
        } else {
            0
        };

        let ms = (capped as i64 + jitter).max(0) as u64;
        Duration::from_millis(ms)
    }

    /// Whether an HTTP status code is a server error worth retrying.
    /// HTTP 状态码是否值得重试的服务端错误。
    pub fn is_retryable_server_error(status: u16) -> bool {
        matches!(status, 502 | 503 | 504)
    }

    /// Whether an HTTP status code signals rate-limiting.
    /// HTTP 状态码是否表示限流。
    pub fn is_rate_limit(status: u16) -> bool {
        status == 429
    }

    /// Whether the request method is inherently idempotent (safe to retry).
    /// 请求方法是否天然幂等（安全重试）。
    pub fn is_idempotent(method: &str) -> bool {
        matches!(method, "GET" | "HEAD" | "OPTIONS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_increases_with_attempts() {
        let policy = RetryPolicy::default();
        let d0 = policy.backoff(0);
        let d1 = policy.backoff(1);
        let d2 = policy.backoff(2);
        assert!(d1 > d0, "backoff should increase");
        assert!(d2 > d1, "backoff should increase");
    }

    #[test]
    fn backoff_capped_at_max() {
        let policy = RetryPolicy::default();
        let d = policy.backoff(10);
        assert!(d <= policy.max_backoff);
    }

    #[test]
    fn is_retryable_server_error_502_503_504() {
        assert!(RetryPolicy::is_retryable_server_error(502));
        assert!(RetryPolicy::is_retryable_server_error(503));
        assert!(RetryPolicy::is_retryable_server_error(504));
    }

    #[test]
    fn is_not_retryable_server_error_for_500_501() {
        assert!(!RetryPolicy::is_retryable_server_error(500));
        assert!(!RetryPolicy::is_retryable_server_error(501));
    }

    #[test]
    fn is_rate_limit_429() {
        assert!(RetryPolicy::is_rate_limit(429));
        assert!(!RetryPolicy::is_rate_limit(400));
    }

    #[test]
    fn get_is_idempotent() {
        assert!(RetryPolicy::is_idempotent("GET"));
        assert!(RetryPolicy::is_idempotent("HEAD"));
        assert!(!RetryPolicy::is_idempotent("POST"));
        assert!(!RetryPolicy::is_idempotent("DELETE"));
    }

    #[test]
    fn default_policy_values() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_retries, 3);
        assert_eq!(p.max_rate_limit_retries, 2);
    }
}
