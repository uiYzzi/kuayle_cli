// Project commands: list, read.
// 项目命令：list、read。
//
// Uses Client directly for project endpoints since there is no
// dedicated resource module for projects in kuayle-sdk.
// 直接使用 Client 访问项目端点，因为 kuayle-sdk 没有专门的项目资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::project::ProjectResponse;

use crate::cli::{Cli, ProjectAction};
use crate::output::{self, is_json_output};

/// Handle project subcommand dispatch.
/// 处理 project 子命令分发。
pub async fn handle(action: &ProjectAction, cli: &Cli) {
    match action {
        ProjectAction::List => cmd_list(cli).await,
        ProjectAction::Read { id } => cmd_read(cli, id).await,
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

/// List all projects in the workspace.
/// 列出工作区中的所有项目。
async fn cmd_list(cli: &Cli) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/projects");

    match client.get::<Vec<ProjectResponse>>(&path).await {
        Ok(projects) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&projects).unwrap_or_default()
                );
            } else {
                if projects.is_empty() {
                    println!("No projects found.");
                    println!("没有找到项目。");
                    return;
                }
                println!("{:<40}  {:<15}  {:<15}", "NAME", "STATUS", "PROGRESS");
                println!("{:-<40}  {:-<15}  {:-<15}", "", "", "");
                for p in &projects {
                    let status = p
                        .status
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_else(|| "-".to_string());
                    let progress = progress_label(p.progress.as_ref());
                    println!("{:<40}  {:<15}  {:<15}", p.name, status, progress);
                }
                println!("\n{} project(s)", projects.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── read ────────────────────────────────────────────────────────────

/// Read a single project by ID.
/// 通过 ID 读取单个项目。
async fn cmd_read(cli: &Cli, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/projects/{id}");

    match client.get::<ProjectResponse>(&path).await {
        Ok(project) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&project).unwrap_or_default()
                );
            } else {
                println!("ID:                 {}", project.id);
                println!("Name:               {}", project.name);
                if let Some(ref d) = project.description {
                    println!("Description:        {}", d);
                }
                if let Some(ref s) = project.status {
                    println!("Status:             {:?}", s);
                }
                if let Some(ref t) = project.team_id {
                    println!("Team ID:            {}", t);
                }
                if let Some(ref l) = project.lead_id {
                    println!("Lead ID:            {}", l);
                }
                if let Some(ref sd) = project.start_date {
                    println!("Start Date:         {}", sd);
                }
                if let Some(ref td) = project.target_date {
                    println!("Target Date:        {}", td);
                }
                if let Some(so) = project.sort_order {
                    println!("Sort Order:         {}", so);
                }
                if let Some(ref prog) = project.progress {
                    println!(
                        "Progress:           {}/{} ({} cancelled)",
                        prog.completed, prog.total, prog.cancelled
                    );
                }
                println!("Created:            {}", project.created_at);
                println!("Updated:            {}", project.updated_at);
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── helpers ─────────────────────────────────────────────────────────

/// Format a progress label for display.
/// 格式化进度标签以供显示。
fn progress_label(progress: Option<&kuayle_sdk::types::common::ProgressInfo>) -> String {
    match progress {
        Some(p) => {
            let pct = p
                .completed
                .checked_mul(100)
                .and_then(|x| x.checked_div(p.total))
                .unwrap_or(0);
            format!("{}/{} ({}%)", p.completed, p.total, pct)
        }
        None => "-".to_string(),
    }
}
