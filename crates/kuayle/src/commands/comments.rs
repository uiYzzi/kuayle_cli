// Comment commands: list, create, resolve, reopen.
// 评论命令：list、create、resolve、reopen。
//
// Uses Client directly for comment endpoints since there is no
// dedicated resource module for comments in kuayle-sdk.
// 直接使用 Client 访问评论端点，因为 kuayle-sdk 没有专门的评论资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::comment::{CommentResponse, CreateCommentRequest, UpdateCommentRequest};

use crate::cli::{Cli, CommentAction};
use crate::output::{self, is_json_output};

/// Handle comment subcommand dispatch.
/// 处理 comment 子命令分发。
pub async fn handle(action: &CommentAction, cli: &Cli) {
    match action {
        CommentAction::List { issue } => cmd_list(cli, issue).await,
        CommentAction::Create { issue, body } => cmd_create(cli, issue, body).await,
        CommentAction::Resolve { id } => cmd_resolve(cli, id).await,
        CommentAction::Reopen { id } => cmd_reopen(cli, id).await,
    }
}

// ── resolve helper ──────────────────────────────────────────────────

/// Resolve client, workspace slug, and is_json flag from CLI context.
/// 从 CLI 上下文解析 client、工作区 slug 和 is_json 标志。
async fn resolve(cli: &Cli) -> (Client, String, bool) {
    let is_json = is_json_output(cli);
    let (client, _url) = match crate::commands::resolve_client(cli).await {
        Ok(c) => c,
        Err(e) => output::print_string_error(&e, 2, is_json),
    };
    let ws = cli.workspace.as_deref().unwrap_or("acme").to_string();
    (client, ws, is_json)
}

// ── list ────────────────────────────────────────────────────────────

/// List comments on an issue.
/// 列出 issue 的评论。
async fn cmd_list(cli: &Cli, issue: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/issues/{issue}/comments");

    match client.get::<Vec<CommentResponse>>(&path).await {
        Ok(comments) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&comments).unwrap_or_default()
                );
            } else {
                if comments.is_empty() {
                    println!("No comments on issue {issue}.");
                    println!("issue {issue} 没有评论。");
                    return;
                }
                for c in &comments {
                    let user = c
                        .user
                        .as_ref()
                        .map(|u| u.display_name.as_str())
                        .unwrap_or(&c.user_id);
                    let resolved = if c.is_resolved { " ✓" } else { "" };
                    println!("[{user}] ({}){resolved}", c.created_at);
                    println!("  {}", c.body);
                    println!("  id: {}", c.id);
                    println!();
                }
                println!("{} comment(s)", comments.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── create ──────────────────────────────────────────────────────────

/// Create a comment on an issue.
/// 在 issue 上创建评论。
async fn cmd_create(cli: &Cli, issue: &str, body: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/issues/{issue}/comments");

    let req = CreateCommentRequest {
        body: body.to_string(),
        parent_id: None,
    };

    match client.post::<_, CommentResponse>(&path, &req).await {
        Ok(comment) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&comment).unwrap_or_default()
                );
            } else {
                println!("✓ Comment created on issue {issue}");
                println!("✓ 已在 issue {issue} 上创建评论");
                println!("  id: {}", comment.id);
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── resolve ─────────────────────────────────────────────────────────

/// Mark a comment as resolved.
/// 将评论标记为已解决。
async fn cmd_resolve(cli: &Cli, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    // The comment resolve endpoint uses PATCH on the comment resource.
    // 评论 resolve 端点使用 PATCH 请求评论资源。
    let path = format!("/api/workspaces/{ws}/issues/comments/{id}");

    let req = UpdateCommentRequest { is_resolved: true };

    match client.patch::<_, CommentResponse>(&path, &req).await {
        Ok(comment) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&comment).unwrap_or_default()
                );
            } else {
                println!("✓ Comment {id} resolved");
                println!("✓ 评论 {id} 已解决");
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── reopen ──────────────────────────────────────────────────────────

/// Reopen a resolved comment.
/// 重新打开已解决的评论。
async fn cmd_reopen(cli: &Cli, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/issues/comments/{id}");

    let req = UpdateCommentRequest { is_resolved: false };

    match client.patch::<_, CommentResponse>(&path, &req).await {
        Ok(comment) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&comment).unwrap_or_default()
                );
            } else {
                println!("✓ Comment {id} reopened");
                println!("✓ 评论 {id} 已重新打开");
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}
