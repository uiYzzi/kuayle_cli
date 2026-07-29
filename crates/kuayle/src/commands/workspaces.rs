// Workspace commands.
// 工作区命令。

use crate::cli::{Cli, WorkspaceAction};

/// Handle workspace subcommand dispatch.
/// 处理 workspace 子命令分发。
pub async fn handle(action: &WorkspaceAction, _cli: &Cli) {
    match action {
        WorkspaceAction::List => {
            eprintln!("workspaces list — not yet implemented");
            eprintln!("工作区列表 — 尚未实现");
            std::process::exit(1);
        }
    }
}
