// Team commands: list, read.
// 团队命令：list、read。
//
// Uses Client directly for team endpoints since there is no
// dedicated resource module for teams in kuayle-sdk.
// 直接使用 Client 访问团队端点，因为 kuayle-sdk 没有专门的团队资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::team::TeamResponse;

use crate::cli::{Cli, TeamAction};
use crate::output::{self, is_json_output};

/// Handle team subcommand dispatch.
/// 处理 team 子命令分发。
pub async fn handle(action: &TeamAction, cli: &Cli) {
    match action {
        TeamAction::List => cmd_list(cli).await,
        TeamAction::Read { id } => cmd_read(cli, id).await,
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

/// List all teams in the workspace.
/// 列出工作区中的所有团队。
async fn cmd_list(cli: &Cli) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/teams");

    match client.get::<Vec<TeamResponse>>(&path).await {
        Ok(teams) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&teams).unwrap_or_default()
                );
            } else {
                if teams.is_empty() {
                    println!("No teams found.");
                    println!("没有找到团队。");
                    return;
                }
                println!("{:<40}  {:<10}  {:<50}", "NAME", "KEY", "DESCRIPTION");
                println!("{:-<40}  {:-<10}  {:-<50}", "", "", "");
                for t in &teams {
                    let desc = t.description.as_deref().unwrap_or("-");
                    println!("{:<40}  {:<10}  {:<50}", t.name, t.key, truncate(desc, 48));
                }
                println!("\n{} team(s)", teams.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── read ────────────────────────────────────────────────────────────

/// Read a single team by ID.
/// 通过 ID 读取单个团队。
async fn cmd_read(cli: &Cli, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/teams/{id}");

    match client.get::<TeamResponse>(&path).await {
        Ok(team) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&team).unwrap_or_default()
                );
            } else {
                println!("ID:                       {}", team.id);
                println!("Name:                     {}", team.name);
                println!("Key:                      {}", team.key);
                if let Some(ref d) = team.description {
                    println!("Description:              {}", d);
                }
                if let Some(ref c) = team.color {
                    println!("Color:                    {}", c);
                }
                if let Some(ref i) = team.icon {
                    println!("Icon:                     {}", i);
                }
                if let Some(te) = team.triage_enabled {
                    println!("Triage Enabled:           {}", te);
                }
                if let Some(pa) = team.parent_auto_close_enabled {
                    println!("Parent Auto Close:        {}", pa);
                }
                if let Some(sa) = team.sub_issue_auto_close_enabled {
                    println!("Sub Issue Auto Close:     {}", sa);
                }
                if let Some(ref cp) = team.issue_copy_prompt {
                    println!("Issue Copy Prompt:        {}", cp);
                }
                println!("Created:                  {}", team.created_at);
                println!("Updated:                  {}", team.updated_at);
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
