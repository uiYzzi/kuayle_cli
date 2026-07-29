// Template commands: list, read, create, update, delete (full CRUD).
// 模板命令：list、read、create、update、delete（完整 CRUD）。
//
// Requires PAT with issue:create scope.
// 需要 issue:create 作用域的 PAT。
//
// Uses Client directly for template endpoints since there is no
// dedicated resource module for templates in kuayle-sdk.
// 直接使用 Client 访问模板端点，因为 kuayle-sdk 没有专门的模板资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::template::{CreateTemplateRequest, TemplateResponse, UpdateTemplateRequest};

use crate::cli::{Cli, TemplateAction};
use crate::output::{self, is_json_output};

/// Handle template subcommand dispatch.
/// 处理 template 子命令分发。
pub async fn handle(action: &TemplateAction, cli: &Cli) {
    match action {
        TemplateAction::List => cmd_list(cli).await,
        TemplateAction::Read { id } => cmd_read(cli, id).await,
        TemplateAction::Create {
            name,
            title,
            description,
        } => cmd_create(cli, name, title, description.as_deref()).await,
        TemplateAction::Update {
            id,
            name,
            title,
            description,
        } => {
            cmd_update(
                cli,
                id,
                name.as_deref(),
                title.as_deref(),
                description.as_deref(),
            )
            .await
        }
        TemplateAction::Delete { id } => cmd_delete(cli, id).await,
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

/// List all issue templates in the workspace.
/// 列出工作区中的所有 issue 模板。
async fn cmd_list(cli: &Cli) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/issue-templates");

    match client.get::<Vec<TemplateResponse>>(&path).await {
        Ok(templates) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&templates).unwrap_or_default()
                );
            } else {
                if templates.is_empty() {
                    println!("No templates found.");
                    println!("没有找到模板。");
                    return;
                }
                println!("{:<40}  {:<50}  {:<15}", "NAME", "TITLE", "ID");
                println!("{:-<40}  {:-<50}  {:-<15}", "", "", "");
                for t in &templates {
                    let title = t.title.as_deref().unwrap_or("-");
                    println!("{:<40}  {:<50}  {:<15}", t.name, truncate(title, 48), t.id);
                }
                println!("\n{} template(s)", templates.len());
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── read ────────────────────────────────────────────────────────────

/// Read a single template by ID.
/// 通过 ID 读取单个模板。
async fn cmd_read(cli: &Cli, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/issue-templates/{id}");

    match client.get::<TemplateResponse>(&path).await {
        Ok(template) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&template).unwrap_or_default()
                );
            } else {
                println!("ID:           {}", template.id);
                println!("Name:         {}", template.name);
                if let Some(ref t) = template.title {
                    println!("Title:        {}", t);
                }
                if let Some(ref d) = template.description {
                    println!("Description:  {}", d);
                }
                if let Some(ref tid) = template.team_id {
                    println!("Team ID:      {}", tid);
                }
                println!("Created:      {}", template.created_at);
                println!("Updated:      {}", template.updated_at);
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── create ──────────────────────────────────────────────────────────

/// Create a new issue template.
/// POST /api/workspaces/{ws}/issue-templates
/// 创建新的 issue 模板。
async fn cmd_create(cli: &Cli, name: &str, title: &str, description: Option<&str>) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/issue-templates");

    let req = CreateTemplateRequest {
        name: name.to_string(),
        title: Some(title.to_string()),
        description: description.map(|s| s.to_string()),
        team_id: None,
    };

    match client.post::<_, TemplateResponse>(&path, &req).await {
        Ok(template) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&template).unwrap_or_default()
                );
            } else {
                println!("✓ Created template \"{}\"", template.name);
                println!("✓ 已创建模板 \"{}\"", template.name);
                println!("  id: {}", template.id);
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── update ──────────────────────────────────────────────────────────

/// Update an existing template.
/// PATCH /api/workspaces/{ws}/issue-templates/{id}
/// 更新已有模板。
async fn cmd_update(
    cli: &Cli,
    id: &str,
    name: Option<&str>,
    title: Option<&str>,
    description: Option<&str>,
) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/issue-templates/{id}");

    let req = UpdateTemplateRequest {
        name: name.map(|s| s.to_string()),
        title: title.map(|s| s.to_string()),
        description: description.map(|s| s.to_string()),
        team_id: None,
    };

    match client.patch::<_, TemplateResponse>(&path, &req).await {
        Ok(template) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&template).unwrap_or_default()
                );
            } else {
                println!("✓ Updated template \"{}\"", template.name);
                println!("✓ 已更新模板 \"{}\"", template.name);
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── delete ──────────────────────────────────────────────────────────

/// Delete a template by ID.
/// DELETE /api/workspaces/{ws}/issue-templates/{id}
/// 通过 ID 删除模板。
async fn cmd_delete(cli: &Cli, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/issue-templates/{id}");

    match client.delete::<serde_json::Value>(&path).await {
        Ok(_) => {
            if is_json {
                println!(r#"{{"deleted":"{id}"}}"#);
            } else {
                println!("✓ Deleted template {id}");
                println!("✓ 已删除模板 {id}");
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
