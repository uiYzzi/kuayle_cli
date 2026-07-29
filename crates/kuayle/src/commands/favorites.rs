// Favorite commands: list (read-only).
// 收藏命令：list（只读）。
//
// create/delete require sessionOnly PAT — not implemented.
// create/delete 需要 sessionOnly PAT — 未实现。
//
// Uses Client directly for favorite endpoints since there is no
// dedicated resource module for favorites in kuayle-sdk.
// 直接使用 Client 访问收藏端点，因为 kuayle-sdk 没有专门的收藏资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::favorite::FavoriteResponse;

use crate::cli::{Cli, FavoriteAction};
use crate::output::{self, is_json_output};

/// Handle favorite subcommand dispatch.
/// 处理 favorite 子命令分发。
pub async fn handle(action: &FavoriteAction, cli: &Cli) {
    match action {
        FavoriteAction::List => cmd_list(cli).await,
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

/// List all favorites in the workspace.
/// 列出工作区中的所有收藏。
async fn cmd_list(cli: &Cli) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/favorites");

    match client.get::<Vec<FavoriteResponse>>(&path).await {
        Ok(favorites) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&favorites).unwrap_or_default()
                );
            } else {
                if favorites.is_empty() {
                    println!("No favorites found.");
                    println!("没有找到收藏。");
                    return;
                }
                println!(
                    "{:<15}  {:<15}  {:<40}  {:<40}",
                    "TYPE", "ID", "FAVORITABLE ID", "USER ID"
                );
                println!("{:-<15}  {:-<15}  {:-<40}  {:-<40}", "", "", "", "");
                for f in &favorites {
                    println!(
                        "{:<15}  {:<15}  {:<40}  {:<40}",
                        f.favoritable_type, f.id, f.favoritable_id, f.user_id
                    );
                }
                println!("\n{} favorite(s)", favorites.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}
