// Issue commands: list, read, create, update, delete, batch ops, subscriptions, history.
// Issue 命令：list、read、create、update、delete、批量操作、订阅、历史。
//
// Dispatches to the kuayle Issues resource for typed API access.
// Uses IssueFilter for type-safe query building and supports
// --all for full pagination.
// 分发到 kuayle Issues 资源进行类型化 API 访问。
// 使用 IssueFilter 进行类型安全的查询构建，支持 --all 完整分页。

use futures_util::StreamExt;
use kuayle_sdk::filter::{IssueFilter, Priority};
use kuayle_sdk::resources::issues::Issues;
use kuayle_sdk::types::issue::{
    BatchUpdateRequest, CreateIssueRequest, IssueResponse, UpdateIssueRequest,
};

use crate::cli::{Cli, IssueAction};
use crate::commands::resolve_client;
use crate::output::{self, is_json_output};

/// Handle issue subcommand dispatch.
/// 处理 issue 子命令分发。
pub async fn handle(action: &IssueAction, cli: &Cli) {
    match action {
        IssueAction::List {
            status,
            priority,
            assignee,
            label,
            search,
            all,
        } => cmd_list(cli, status, *priority, assignee, label, search, *all).await,
        IssueAction::Read { identifier } => cmd_read(cli, identifier).await,
        IssueAction::Create {
            title,
            description,
            priority,
            team,
            assignee,
            labels,
            project,
            cycle,
        } => {
            cmd_create(
                cli,
                title,
                description.as_deref(),
                *priority,
                team.as_deref(),
                assignee.as_ref(),
                labels.as_ref(),
                project.as_deref(),
                cycle.as_deref(),
            )
            .await
        }
        IssueAction::Update {
            identifier,
            title,
            description,
            status,
            priority,
            assignee,
            labels,
        } => {
            cmd_update(
                cli,
                identifier,
                title.as_deref(),
                description.as_deref(),
                status.as_deref(),
                *priority,
                assignee.as_ref(),
                labels.as_ref(),
            )
            .await
        }
        IssueAction::Delete { identifier } => cmd_delete(cli, identifier).await,
        IssueAction::BatchUpdate {
            identifiers,
            status,
            priority,
        } => cmd_batch_update(cli, identifiers, status.as_deref(), *priority).await,
        IssueAction::BatchDelete { identifiers } => cmd_batch_delete(cli, identifiers).await,
        IssueAction::Subscribe { identifier } => cmd_subscribe(cli, identifier).await,
        IssueAction::Unsubscribe { identifier } => cmd_unsubscribe(cli, identifier).await,
        IssueAction::History { identifier } => cmd_history(cli, identifier).await,
    }
}

// ── resolve helper ──────────────────────────────────────────────────

/// Resolve client+Issues resource and is_json flag from CLI context.
/// 从 CLI 上下文解析 client+Issues 资源和 is_json 标志。
async fn resolve(cli: &Cli) -> (Issues, bool) {
    let is_json = is_json_output(cli);
    let (client, _url) = match resolve_client(cli).await {
        Ok(c) => c,
        Err(e) => output::print_string_error(&e, 2, is_json),
    };
    let workspace_slug = cli.workspace.as_deref().unwrap_or("acme");
    let issues = Issues::new(&client, workspace_slug);
    (issues, is_json)
}

// ── list ────────────────────────────────────────────────────────────

/// List issues with optional filters. Supports --all for full pagination.
/// 列出 issue，带可选过滤。支持 --all 进行完整分页。
async fn cmd_list(
    cli: &Cli,
    status: &Option<String>,
    priority: Option<i32>,
    assignee: &Option<String>,
    label: &Option<String>,
    search: &Option<String>,
    all: bool,
) {
    let (issues, is_json) = resolve(cli).await;

    // Build the issue filter from CLI args.
    // 从 CLI 参数构建 issue filter。
    let mut filter = IssueFilter::new();
    if let Some(s) = status {
        filter = filter.status(s.clone());
    }
    if let Some(p) = priority {
        filter = filter.priority(priority_from_int(p));
    }
    if let Some(a) = assignee {
        filter = filter.assignee(a.clone());
    }
    if let Some(l) = label {
        filter = filter.label(l.clone());
    }
    if let Some(s) = search {
        filter = filter.search(s.clone());
    }

    let mut stream = issues.list(filter);
    let mut items: Vec<IssueResponse> = Vec::new();
    let mut has_more = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(item) => {
                items.push(item);
                // If not fetching all pages, stop after one page worth of items.
                // 如果不获取全部页面，在一页内容之后停止。
                if !all && items.len() >= 100 {
                    has_more = true;
                    break;
                }
            }
            Err(e) => {
                output::print_error(&e, is_json);
                std::process::exit(e.exit_code());
            }
        }
    }

    if is_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_default()
        );
    } else {
        if items.is_empty() {
            println!("No issues found.");
            println!("没有找到 issue。");
            return;
        }
        println!("{:<16}  {:<40}  {:<10}", "IDENTIFIER", "TITLE", "PRIORITY");
        println!("{:-<16}  {:-<40}  {:-<10}", "", "", "");
        for issue in &items {
            println!(
                "{:<16}  {:<40}  {:<10}",
                issue.identifier,
                truncate(&issue.title, 40),
                priority_label(issue.priority)
            );
        }
        println!("\n{} issue(s)", items.len());
        if has_more {
            println!("… and more (use --all)");
            println!("… 还有更多（使用 --all）");
        }
    }
}

// ── read ────────────────────────────────────────────────────────────

/// Read a single issue by identifier (e.g. "ENG-25").
/// 通过 identifier 读取单个 issue（如 "ENG-25"）。
async fn cmd_read(cli: &Cli, identifier: &str) {
    let (issues, is_json) = resolve(cli).await;

    match issues.read(identifier).await {
        Ok(issue) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&issue).unwrap_or_default()
                );
            } else {
                println!("Identifier:  {}", issue.identifier);
                println!("ID:          {}", issue.id);
                println!("Title:       {}", issue.title);
                if let Some(ref desc) = issue.description {
                    println!("Description: {}", desc);
                }
                println!("Status:      {}", issue.status);
                println!("Priority:    {}", priority_label(issue.priority));
                if let Some(ref assignee) = issue.assignee {
                    println!(
                        "Assignee:    {} ({})",
                        assignee.display_name, assignee.email
                    );
                }
                if !issue.labels.is_empty() {
                    let names: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();
                    println!("Labels:      {}", names.join(", "));
                }
                if let Some(ref creator) = issue.creator {
                    println!("Creator:     {} ({})", creator.display_name, creator.email);
                }
                println!("Created:     {}", issue.created_at);
                println!("Updated:     {}", issue.updated_at);
                println!("Subscribed:  {}", issue.is_subscribed);
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── create ──────────────────────────────────────────────────────────

/// Create a new issue.
/// 创建新 issue。
#[allow(clippy::too_many_arguments)]
async fn cmd_create(
    cli: &Cli,
    title: &str,
    description: Option<&str>,
    priority: Option<i32>,
    team: Option<&str>,
    assignee: Option<&Vec<String>>,
    labels: Option<&Vec<String>>,
    project: Option<&str>,
    cycle: Option<&str>,
) {
    let (issues, is_json) = resolve(cli).await;

    let req = CreateIssueRequest {
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
        priority,
        team_id: team.map(|s| s.to_string()),
        assignee_ids: assignee.cloned(),
        label_ids: labels.cloned(),
        project_id: project.map(|s| s.to_string()),
        cycle_id: cycle.map(|s| s.to_string()),
        ..Default::default()
    };

    match issues.create(&req).await {
        Ok(issue) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&issue).unwrap_or_default()
                );
            } else {
                println!("✓ Created issue {}", issue.identifier);
                println!("✓ 已创建 issue {}", issue.identifier);
                println!("  Title: {}", issue.title);
                println!("  Status: {}", issue.status);
                println!("  Priority: {}", priority_label(issue.priority));
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── update ──────────────────────────────────────────────────────────

/// Update an existing issue.
/// 更新已有 issue。
#[allow(clippy::too_many_arguments)]
async fn cmd_update(
    cli: &Cli,
    identifier: &str,
    title: Option<&str>,
    description: Option<&str>,
    status: Option<&str>,
    priority: Option<i32>,
    assignee: Option<&Vec<String>>,
    labels: Option<&Vec<String>>,
) {
    let (issues, is_json) = resolve(cli).await;

    let req = UpdateIssueRequest {
        title: title.map(|s| s.to_string()),
        description: description.map(|s| s.to_string()),
        status: status.map(|s| s.to_string()),
        priority,
        assignee_ids: assignee.cloned(),
        label_ids: labels.cloned(),
        ..Default::default()
    };

    match issues.update(identifier, &req).await {
        Ok(issue) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&issue).unwrap_or_default()
                );
            } else {
                println!("✓ Updated issue {}", issue.identifier);
                println!("✓ 已更新 issue {}", issue.identifier);
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── delete ──────────────────────────────────────────────────────────

/// Delete an issue by identifier.
/// 通过 identifier 删除 issue。
async fn cmd_delete(cli: &Cli, identifier: &str) {
    let (issues, is_json) = resolve(cli).await;

    match issues.delete(identifier).await {
        Ok(_) => {
            if is_json {
                println!(r#"{{"deleted":"{identifier}"}}"#);
            } else {
                println!("✓ Deleted issue {identifier}");
                println!("✓ 已删除 issue {identifier}");
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── batch update ────────────────────────────────────────────────────

/// Batch update multiple issues at once.
/// 批量更新多个 issue。
async fn cmd_batch_update(
    cli: &Cli,
    identifiers: &[String],
    status: Option<&str>,
    priority: Option<i32>,
) {
    let (issues, is_json) = resolve(cli).await;

    let req = BatchUpdateRequest {
        issue_identifiers: identifiers.to_vec(),
        update: UpdateIssueRequest {
            status: status.map(|s| s.to_string()),
            priority,
            ..Default::default()
        },
    };

    match issues.batch_update(&req).await {
        Ok(result) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            } else {
                println!("✓ Batch updated {} issue(s)", identifiers.len());
                println!("✓ 已批量更新 {} 个 issue", identifiers.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── batch delete ────────────────────────────────────────────────────

/// Batch delete multiple issues at once.
/// 批量删除多个 issue。
async fn cmd_batch_delete(cli: &Cli, identifiers: &[String]) {
    let (issues, is_json) = resolve(cli).await;

    match issues.batch_delete(identifiers).await {
        Ok(result) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            } else {
                println!("✓ Batch deleted {} issue(s)", identifiers.len());
                println!("✓ 已批量删除 {} 个 issue", identifiers.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── subscribe / unsubscribe ─────────────────────────────────────────

/// Subscribe to issue notifications.
/// 订阅 issue 通知。
async fn cmd_subscribe(cli: &Cli, identifier: &str) {
    let (issues, is_json) = resolve(cli).await;

    match issues.subscribe(identifier).await {
        Ok(_) => {
            if is_json {
                println!(r#"{{"subscribed":"{identifier}"}}"#);
            } else {
                println!("✓ Subscribed to issue {identifier}");
                println!("✓ 已订阅 issue {identifier}");
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

/// Unsubscribe from issue notifications.
/// 取消订阅 issue 通知。
async fn cmd_unsubscribe(cli: &Cli, identifier: &str) {
    let (issues, is_json) = resolve(cli).await;

    match issues.unsubscribe(identifier).await {
        Ok(_) => {
            if is_json {
                println!(r#"{{"unsubscribed":"{identifier}"}}"#);
            } else {
                println!("✓ Unsubscribed from issue {identifier}");
                println!("✓ 已取消订阅 issue {identifier}");
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── history ─────────────────────────────────────────────────────────

/// Show issue activity history.
/// 显示 issue 活动历史。
async fn cmd_history(cli: &Cli, identifier: &str) {
    let (issues, is_json) = resolve(cli).await;

    match issues.history(identifier).await {
        Ok(result) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            } else {
                println!("History for {identifier}:");
                println!("{identifier} 的历史：");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
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

/// Map priority integer (0-4) to a human-readable label.
/// 将优先级整数 (0-4) 映射为人类可读的标签。
fn priority_label(p: i32) -> &'static str {
    match p {
        0 => "None / 无",
        1 => "Urgent / 紧急",
        2 => "High / 高",
        3 => "Medium / 中",
        4 => "Low / 低",
        _ => "Unknown / 未知",
    }
}

/// Convert a priority integer (0-4) into a `Priority` enum.
/// 将优先级整数 (0-4) 转换为 `Priority` 枚举。
fn priority_from_int(p: i32) -> Priority {
    match p {
        0 => Priority::None,
        1 => Priority::Urgent,
        2 => Priority::High,
        3 => Priority::Medium,
        4 => Priority::Low,
        _ => Priority::None,
    }
}
