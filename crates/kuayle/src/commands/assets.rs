// Asset commands: read, upload (placeholder).
// 附件命令：read、upload（占位）。
//
// Upload is multipart — not implemented yet, prints a placeholder message.
// 上传使用 multipart — 尚未实现，打印占位信息。
//
// Uses Client directly for asset endpoints since there is no
// dedicated resource module for assets in kuayle-sdk.
// 直接使用 Client 访问附件端点，因为 kuayle-sdk 没有专门的附件资源模块。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::asset::AssetResponse;

use crate::cli::{AssetAction, Cli};
use crate::output::{self, is_json_output};

/// Handle asset subcommand dispatch.
/// 处理 asset 子命令分发。
pub async fn handle(action: &AssetAction, cli: &Cli) {
    match action {
        AssetAction::Read { id } => cmd_read(cli, id).await,
        AssetAction::Upload => cmd_upload(cli).await,
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

// ── read ────────────────────────────────────────────────────────────

/// Read a single asset by ID.
/// GET /api/workspaces/{ws}/assets/{id}
/// 通过 ID 读取单个附件。
async fn cmd_read(cli: &Cli, id: &str) {
    let (client, ws, is_json) = resolve(cli).await;
    let path = format!("/api/workspaces/{ws}/assets/{id}");

    match client.get::<AssetResponse>(&path).await {
        Ok(asset) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&asset).unwrap_or_default()
                );
            } else {
                println!("ID:            {}", asset.id);
                println!("Filename:      {}", asset.filename);
                if let Some(ref ct) = asset.content_type {
                    println!("Content Type:  {}", ct);
                }
                if let Some(sz) = asset.size {
                    println!("Size:          {} bytes", sz);
                }
                if let Some(ref url) = asset.url {
                    println!("URL:           {}", url);
                }
                if let Some(ref ca) = asset.created_at {
                    println!("Created:       {}", ca);
                }
            }
        }
        Err(e) => {
            output::print_error(&e, is_json);
            std::process::exit(e.exit_code());
        }
    }
}

// ── upload (placeholder) ────────────────────────────────────────────

/// Upload a file — not implemented.
/// Multipart form upload requires reqwest multipart support.
/// 上传文件 — 未实现。
/// 多部分表单上传需要 reqwest 多部分支持。
async fn cmd_upload(_cli: &Cli) {
    println!("upload not implemented");
    println!("上传尚未实现");
}
