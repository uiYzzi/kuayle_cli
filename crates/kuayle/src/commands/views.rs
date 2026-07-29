// View commands: list, read (read-only).
// 视图命令：list、read（只读）。
//
// create/update/delete require sessionOnly PAT — not implemented.
// create/update/delete 需要 sessionOnly PAT — 未实现。
//
// Uses Client directly for view endpoints since there is no
// dedicated resource module for views in kuayle-sdk.
// 直接使用 Client 访问视图端点，因为 kuayle-sdk 没有专门的视图资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::view::ViewResponse;

use crate::cli::{Cli, ViewAction};
use crate::output::{self, is_json_output};

/// Handle view subcommand dispatch.
/// 处理 view 子命令分发。
pub async fn handle(action: &ViewAction, cli: &Cli) {
    match action {
        ViewAction::List => cmd_list(cli).await,
        ViewAction::Read { id } => cmd_read(cli, id).await,
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

/// List all views in the workspace.
/// 列出工作区中的所有视图。
async fn cmd_list(cli: &Cli) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/views");

    match client.get::<Vec<ViewResponse>>(&path).await {
        Ok(views) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&views).unwrap_or_default()
                );
            } else {
                if views.is_empty() {
                    println!("No views found.");
                    println!("没有找到视图。");
                    return;
                }
                println!("{:<40}  {:<50}  {:<15}", "NAME", "DESCRIPTION", "ID");
                println!("{:-<40}  {:-<50}  {:-<15}", "", "", "");
                for v in &views {
                    let desc = v.description.as_deref().unwrap_or("-");
                    println!("{:<40}  {:<50}  {:<15}", v.name, truncate(desc, 48), v.id);
                }
                println!("\n{} view(s)", views.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── read ────────────────────────────────────────────────────────────

/// Read a single view by ID.
/// 通过 ID 读取单个视图。
async fn cmd_read(cli: &Cli, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/views/{id}");

    match client.get::<ViewResponse>(&path).await {
        Ok(view) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&view).unwrap_or_default()
                );
            } else {
                println!("ID:           {}", view.id);
                println!("Name:         {}", view.name);
                if let Some(ref d) = view.description {
                    println!("Description:  {}", d);
                }
                if let Some(ref f) = view.filter {
                    println!(
                        "Filter:       {}",
                        serde_json::to_string_pretty(f).unwrap_or_default()
                    );
                }
                if let Some(ref tid) = view.team_id {
                    println!("Team ID:      {}", tid);
                }
                println!("Created:      {}", view.created_at);
                println!("Updated:      {}", view.updated_at);
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
