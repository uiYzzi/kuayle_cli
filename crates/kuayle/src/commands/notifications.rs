// Notification commands: list (read-only, user-scoped).
// 通知命令：list（只读，用户范围）。
//
// mark_read/snooze require sessionOnly PAT — not implemented.
// mark_read/snooze 需要 sessionOnly PAT — 未实现。
//
// Note: notifications are user-scoped, not workspace-scoped.
// The path is "/api/notifications" (no workspace prefix).
// 注意：通知是用户范围的，而非工作区范围。
// 路径为 "/api/notifications"（无工作区前缀）。
//
// Uses Client directly for notification endpoints since there is no
// dedicated resource module for notifications in kuayle-sdk.
// 直接使用 Client 访问通知端点，因为 kuayle-sdk 没有专门的通知资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::notification::NotificationResponse;

use crate::cli::{Cli, NotificationAction};
use crate::output::{self, is_json_output};

/// Handle notification subcommand dispatch.
/// 处理 notification 子命令分发。
pub async fn handle(action: &NotificationAction, cli: &Cli) {
    match action {
        NotificationAction::List => cmd_list(cli).await,
    }
}

// ── resolve helper (user-scoped, no workspace) ──────────────────────

/// Resolve client and is_json flag from CLI context.
/// Notifications are user-scoped — no workspace prefix in the path.
/// 从 CLI 上下文解析 client 和 is_json 标志。
/// 通知是用户范围的 — 路径中无工作区前缀。
async fn resolve(cli: &Cli) -> (Client, bool) {
    let is_json = is_json_output(cli);
    let (client, _url) = match crate::commands::resolve_client(cli).await {
        Ok(c) => c,
        Err(e) => output::print_string_error(&e, 2, is_json),
    };
    (client, is_json)
}

// ── list ────────────────────────────────────────────────────────────

/// List notifications for the authenticated user.
/// GET /api/notifications (no workspace prefix)
/// 列出已认证用户的通知。
async fn cmd_list(cli: &Cli) {
    let (client, is_json) = resolve(cli).await;
    let path = "/api/notifications";

    match client.get::<Vec<NotificationResponse>>(path).await {
        Ok(notifications) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&notifications).unwrap_or_default()
                );
            } else {
                if notifications.is_empty() {
                    println!("No notifications.");
                    println!("没有通知。");
                    return;
                }
                println!(
                    "{:<20}  {:<50}  {:<15}  {:<36}",
                    "TYPE", "TITLE", "READ", "ID"
                );
                println!("{:-<20}  {:-<50}  {:-<15}  {:-<36}", "", "", "", "");
                for n in &notifications {
                    let title = n.title.as_deref().unwrap_or("-");
                    let read = if n.read_at.is_some() { "✓" } else { "" };
                    println!(
                        "{:<20}  {:<50}  {:<15}  {:<36}",
                        n.notification_type,
                        truncate(title, 48),
                        read,
                        n.id
                    );
                }
                println!("\n{} notification(s)", notifications.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── helpers ─────────────────────────────────────────────────────────

/// Truncate a string to `max` chars, appending "…" if truncated.
/// 将字符串截断到 `max` 个字符，超出则追加 "…"。
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
