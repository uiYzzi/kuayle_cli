// CLI clap definitions.
// CLI clap 定义。

use clap::{Parser, Subcommand};

/// kuayle — CLI for the kuayle self-hosted issue tracker
/// kuayle — 自托管 issue tracker kuayle 的命令行工具
#[derive(Parser, Debug)]
#[command(name = "kuayle", version, about, long_about = None)]
pub struct Cli {
    /// Profile name to use
    /// 要使用的 profile 名称
    #[arg(long, global = true, env = "KUAYLE_PROFILE")]
    pub profile: Option<String>,

    /// Override the kuayle instance URL
    /// 覆盖 kuayle 实例 URL
    #[arg(long, global = true, env = "KUAYLE_URL")]
    pub url: Option<String>,

    /// Override the default workspace slug
    /// 覆盖默认工作区 slug
    #[arg(long, global = true, env = "KUAYLE_WORKSPACE")]
    pub workspace: Option<String>,

    /// Output format: human, json, or auto (detect tty)
    /// 输出格式：human、json 或 auto（检测 tty）
    #[arg(long, global = true, default_value = "auto")]
    pub format: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Authenticate with a kuayle instance (login, logout, status)
    /// 认证 kuayle 实例（login、logout、status）
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    /// Show the authenticated user's profile
    /// 显示已认证用户的个人资料
    Whoami,

    /// Manage workspaces
    /// 管理工作区
    Workspaces {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthAction {
    /// Log in to a kuayle instance
    /// 登录 kuayle 实例
    Login {
        /// Personal Access Token (kuayle_pat_...)
        /// 个人访问令牌（kuayle_pat_...）
        #[arg(long)]
        token: Option<String>,
    },

    /// Log out and remove stored credentials
    /// 登出并移除存储的凭据
    Logout,

    /// Show authentication status
    /// 显示认证状态
    Status,
}

#[derive(Subcommand, Debug)]
pub enum WorkspaceAction {
    /// List workspaces
    /// 列出工作区
    List,
}
