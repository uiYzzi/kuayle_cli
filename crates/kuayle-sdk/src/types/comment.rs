// Comment types aligned with kuayle API.
// 与 kuayle API 对齐的评论类型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::issue::IssueUser;

/// A comment on an issue.
/// issue 上的评论。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub issue_id: String,
    pub body: String,
    pub user_id: String,
    pub user: Option<IssueUser>,
    pub parent_id: Option<String>,
    pub is_resolved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating a comment.
/// 创建评论的请求体。
#[derive(Debug, Clone, Serialize)]
pub struct CreateCommentRequest {
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

/// Request body for updating a comment (resolve/reopen).
/// 更新评论的请求体（resolve/reopen）。
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCommentRequest {
    pub is_resolved: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_comment() {
        let json = r#"{
            "id": "abc",
            "issue_id": "10000000-0000-0000-0000-000000000040",
            "body": "Looks good",
            "user_id": "u1",
            "user": null,
            "parent_id": null,
            "is_resolved": false,
            "created_at": "2026-07-28T15:58:38+08:00",
            "updated_at": "2026-07-28T15:58:38+08:00"
        }"#;
        let c: CommentResponse = serde_json::from_str(json).unwrap();
        assert_eq!(c.body, "Looks good");
        assert!(!c.is_resolved);
    }
}
