// CLI clap definitions.
// CLI clap 定义。

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "kuayle", version, about, long_about = None)]
pub struct Cli {
    #[arg(long, global = true, env = "KUAYLE_PROFILE")]
    pub profile: Option<String>,
    #[arg(long, global = true, env = "KUAYLE_URL")]
    pub url: Option<String>,
    #[arg(long, global = true, env = "KUAYLE_WORKSPACE")]
    pub workspace: Option<String>,
    #[arg(long, global = true, default_value = "auto")]
    pub format: String,
    /// Disable resolve disk cache / 禁用解析磁盘缓存
    #[arg(long, global = true, env = "KUAYLE_NO_CACHE")]
    pub no_cache: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    Whoami,
    Workspaces {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    Issues {
        #[command(subcommand)]
        action: IssueAction,
    },
    Comments {
        #[command(subcommand)]
        action: CommentAction,
    },
    Relations {
        #[command(subcommand)]
        action: RelationAction,
    },
    Labels {
        #[command(subcommand)]
        action: LabelAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum AuthAction {
    Login {
        #[arg(long)]
        token: Option<String>,
    },
    Logout,
    Status,
}

#[derive(Subcommand, Debug, Clone)]
pub enum WorkspaceAction {
    List,
}

#[derive(Subcommand, Debug, Clone)]
pub enum IssueAction {
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        priority: Option<i32>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        all: bool,
    },
    Read {
        identifier: String,
    },
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        priority: Option<i32>,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        assignee: Option<Vec<String>>,
        #[arg(long)]
        labels: Option<Vec<String>>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        cycle: Option<String>,
    },
    Update {
        identifier: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        priority: Option<i32>,
        #[arg(long)]
        assignee: Option<Vec<String>>,
        #[arg(long)]
        labels: Option<Vec<String>>,
    },
    Delete {
        identifier: String,
    },
    BatchUpdate {
        identifiers: Vec<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        priority: Option<i32>,
    },
    BatchDelete {
        identifiers: Vec<String>,
    },
    Subscribe {
        identifier: String,
    },
    Unsubscribe {
        identifier: String,
    },
    History {
        identifier: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum CommentAction {
    List {
        issue: String,
    },
    Create {
        issue: String,
        #[arg(long)]
        body: String,
    },
    Resolve {
        issue: String,
        id: String,
    },
    Reopen {
        issue: String,
        id: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum RelationAction {
    List {
        issue: String,
    },
    Create {
        issue: String,
        #[arg(long)]
        related: String,
        #[arg(long, default_value = "related")]
        r#type: String,
    },
    Delete {
        id: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum LabelAction {
    List,
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        color: Option<String>,
    },
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        color: Option<String>,
    },
    Delete {
        id: String,
    },
}
