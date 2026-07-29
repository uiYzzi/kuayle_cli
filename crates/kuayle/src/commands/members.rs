// Member commands: list (read-only).
// 成员命令：list（只读）。
//
// invite/remove require member:invite PAT — not implemented.
// invite/remove 需要 member:invite PAT — 未实现。
//
// Uses Client directly for member endpoints since there is no
// dedicated resource module for members in kuayle-sdk.
// 直接使用 Client 访问成员端点，因为 kuayle-sdk 没有专门的成员资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::member::MemberResponse;

use crate::cli::{Cli, MemberAction};
use crate::output::{self, is_json_output};

/// Handle member subcommand dispatch.
/// 处理 member 子命令分发。
pub async fn handle(action: &MemberAction, cli: &Cli) {
    match action {
        MemberAction::List => cmd_list(cli).await,
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

/// List all members in the workspace.
/// 列出工作区中的所有成员。
async fn cmd_list(cli: &Cli) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/members");

    match client.get::<Vec<MemberResponse>>(&path).await {
        Ok(members) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&members).unwrap_or_default()
                );
            } else {
                if members.is_empty() {
                    println!("No members found.");
                    println!("没有找到成员。");
                    return;
                }
                println!(
                    "{:<40}  {:<30}  {:<15}  {:<36}",
                    "NAME", "EMAIL", "ROLE", "USER ID"
                );
                println!("{:-<40}  {:-<30}  {:-<15}  {:-<36}", "", "", "", "");
                for m in &members {
                    let email = m.email.as_deref().unwrap_or("-");
                    let role = m.role.as_deref().unwrap_or("-");
                    println!(
                        "{:<40}  {:<30}  {:<15}  {:<36}",
                        m.name, email, role, m.user_id
                    );
                }
                println!("\n{} member(s)", members.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}
