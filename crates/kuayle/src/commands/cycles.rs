// Cycle commands: list, read, burndown, velocity (read-only).
// 周期命令：list、read、burndown、velocity（只读）。
//
// create/update/delete require sessionOnly PAT — not implemented.
// create/update/delete 需要 sessionOnly PAT — 未实现。
//
// Uses Client directly for cycle endpoints since there is no
// dedicated resource module for cycles in kuayle-sdk.
// 直接使用 Client 访问周期端点，因为 kuayle-sdk 没有专门的周期资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::cycle::CycleResponse;

use crate::cli::{Cli, CycleAction};
use crate::output::{self, is_json_output};

/// Handle cycle subcommand dispatch.
/// 处理 cycle 子命令分发。
pub async fn handle(action: &CycleAction, cli: &Cli) {
    match action {
        CycleAction::List { team } => cmd_list(cli, team).await,
        CycleAction::Read { team, id } => cmd_read(cli, team, id).await,
        CycleAction::Burndown { team, id } => cmd_burndown(cli, team, id).await,
        CycleAction::Velocity { team } => cmd_velocity(cli, team).await,
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

/// List cycles for a team.
/// GET /api/workspaces/{ws}/teams/{team_id}/cycles
/// 列出团队的周期。
async fn cmd_list(cli: &Cli, team_id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/teams/{team_id}/cycles");

    match client.get::<Vec<CycleResponse>>(&path).await {
        Ok(cycles) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&cycles).unwrap_or_default()
                );
            } else {
                if cycles.is_empty() {
                    println!("No cycles found for team {team_id}.");
                    println!("没有找到团队 {team_id} 的周期。");
                    return;
                }
                println!("{:<40}  {:<10}  {:<15}", "NAME", "NUMBER", "STATUS");
                println!("{:-<40}  {:-<10}  {:-<15}", "", "", "");
                for c in &cycles {
                    let number = c
                        .number
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let status = c
                        .status
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_else(|| "-".to_string());
                    println!("{:<40}  {:<10}  {:<15}", c.name, number, status);
                }
                println!("\n{} cycle(s)", cycles.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── read ────────────────────────────────────────────────────────────

/// Read a single cycle by ID.
/// GET /api/workspaces/{ws}/teams/{team_id}/cycles/{id}
/// 通过 ID 读取单个周期。
async fn cmd_read(cli: &Cli, team_id: &str, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/teams/{team_id}/cycles/{id}");

    match client.get::<CycleResponse>(&path).await {
        Ok(cycle) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&cycle).unwrap_or_default()
                );
            } else {
                println!("ID:               {}", cycle.id);
                println!("Name:             {}", cycle.name);
                if let Some(n) = cycle.number {
                    println!("Number:           {}", n);
                }
                if let Some(ref s) = cycle.status {
                    println!("Status:           {:?}", s);
                }
                if let Some(ref d) = cycle.description {
                    println!("Description:      {}", d);
                }
                if let Some(ref g) = cycle.goals {
                    println!("Goals:            {}", g);
                }
                if let Some(ref r) = cycle.retrospective {
                    println!("Retrospective:    {}", r);
                }
                if let Some(ref sd) = cycle.start_date {
                    println!("Start Date:       {}", sd);
                }
                if let Some(ref ed) = cycle.end_date {
                    println!("End Date:         {}", ed);
                }
                if let Some(ref ca) = cycle.completed_at {
                    println!("Completed At:     {}", ca);
                }
                if let Some(ref prog) = cycle.progress {
                    println!(
                        "Progress:         {}/{} ({} cancelled)",
                        prog.completed, prog.total, prog.cancelled
                    );
                }
                println!("Created:          {}", cycle.created_at);
                println!("Updated:          {}", cycle.updated_at);
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── burndown ────────────────────────────────────────────────────────

/// Get burndown chart data for a cycle.
/// GET /api/workspaces/{ws}/teams/{team_id}/cycles/{id}/burndown
/// 获取周期的 burndown 图数据。
async fn cmd_burndown(cli: &Cli, team_id: &str, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/teams/{team_id}/cycles/{id}/burndown");

    match client.get::<serde_json::Value>(&path).await {
        Ok(data) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            } else {
                println!("Burndown data:");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── velocity ────────────────────────────────────────────────────────

/// Get velocity data for a team.
/// GET /api/workspaces/{ws}/teams/{team_id}/cycles/velocity
/// 获取团队的速度数据。
async fn cmd_velocity(cli: &Cli, team_id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/teams/{team_id}/cycles/velocity");

    match client.get::<serde_json::Value>(&path).await {
        Ok(data) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            } else {
                println!("Velocity data:");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_default()
                );
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}
