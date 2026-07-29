// kuayle CLI entry point.
// kuayle CLI 入口点。
mod cli;
mod commands;
mod config;
mod creds;
mod output;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();

    match &cli.command {
        cli::Command::Auth { action } => {
            commands::auth_cmd::handle(action, &cli).await;
        }
        cli::Command::Whoami => {
            commands::auth_cmd::handle_whoami(&cli).await;
        }
        cli::Command::Workspaces { action } => {
            commands::workspaces::handle(action, &cli).await;
        }
    }
}
