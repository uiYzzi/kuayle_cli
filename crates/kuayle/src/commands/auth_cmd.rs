// Auth commands: login, logout, status, whoami.
// 认证命令：login、logout、status、whoami。

use kuayle_sdk::client::Client;
use kuayle_sdk::types::user::UserResponse;
use url::Url;

use crate::cli::{AuthAction, Cli};
use crate::config::Config;
use crate::creds;

/// Handle auth subcommand dispatch.
/// 处理 auth 子命令分发。
pub async fn handle(action: &AuthAction, cli: &Cli) {
    match action {
        AuthAction::Login { token } => cmd_login(cli, token.as_deref()).await,
        AuthAction::Logout => cmd_logout(cli),
        AuthAction::Status => cmd_status(cli).await,
    }
}

/// Handle whoami command.
/// 处理 whoami 命令。
pub async fn handle_whoami(cli: &Cli) {
    cmd_whoami(cli).await;
}

// ── login ─────────────────────────────────────────────────────────

async fn cmd_login(cli: &Cli, token: Option<&str>) {
    let token = match token {
        Some(t) => t.to_string(),
        None => {
            eprintln!("Error: --token is required for PAT authentication");
            eprintln!("错误：PAT 认证需要 --token 参数");
            std::process::exit(4);
        }
    };

    // Determine the URL.
    // 确定 URL。
    let url = match &cli.url {
        Some(u) => u.clone(),
        None => {
            eprintln!("Error: --url is required (e.g. --url http://localhost:5173)");
            eprintln!("错误：需要 --url 参数（例如 --url http://localhost:5173）");
            std::process::exit(4);
        }
    };

    // Validate the token by calling /api/auth/me.
    // 通过调用 /api/auth/me 验证 token。
    let base_url = match Url::parse(&url) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Error: invalid URL '{}': {e}", url);
            std::process::exit(4);
        }
    };

    let client = Client::new(base_url, token.clone());
    match client.get::<UserResponse>("/api/auth/me").await {
        Ok(user) => {
            // Save the credential.
            // 保存凭据。
            let store = match creds::get_credential_store() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };

            let profile = Config::load_or_default().resolve_profile(cli.profile.as_deref());
            let session = kuayle_sdk::session::Session::pat(&token);

            if let Err(e) = store.save(&profile, &session) {
                eprintln!("Error saving credentials: {e}");
                eprintln!("保存凭据错误：{e}");
                std::process::exit(1);
            }

            // Save profile URL to config.
            // 将 profile URL 保存到配置。
            if let Err(e) = save_profile_to_config(&profile, &url) {
                eprintln!("Warning: could not save profile to config: {e}");
                eprintln!("警告：无法保存 profile 到配置：{e}");
            }

            println!("✓ Logged in as {} ({})", user.display_name, user.email);
            println!("  Profile: {profile}");
            println!("  URL: {url}");
        }
        Err(e) => {
            eprintln!("Error: authentication failed — {e}");
            eprintln!("错误：认证失败 — {e}");
            std::process::exit(e.exit_code());
        }
    }
}

// ── logout ────────────────────────────────────────────────────────

fn cmd_logout(cli: &Cli) {
    let config = Config::load_or_default();
    let profile = config.resolve_profile(cli.profile.as_deref());

    let store = match creds::get_credential_store() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    match store.delete(&profile) {
        Ok(()) => {
            println!("✓ Logged out of profile '{}'", profile);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

// ── status ────────────────────────────────────────────────────────

async fn cmd_status(cli: &Cli) {
    let config = Config::load_or_default();
    let profile = config.resolve_profile(cli.profile.as_deref());

    let store = match creds::get_credential_store() {
        Ok(s) => s,
        Err(e) => {
            println!("Not logged in. Error: {e}");
            println!("未登录。错误：{e}");
            std::process::exit(2);
        }
    };

    let session = match store.load(&profile) {
        Ok(Some(s)) => s,
        Ok(None) => {
            println!("Not logged in. Run 'kuayle auth login' to authenticate.");
            println!("未登录。运行 'kuayle auth login' 进行认证。");
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("Error loading session: {e}");
            std::process::exit(1);
        }
    };

    // Resolve URL.
    // 解析 URL。
    let url = match &cli.url {
        Some(u) => u.clone(),
        None => {
            // Try to get from config.
            // 尝试从配置获取。
            match config.resolve(cli.profile.as_deref(), None, None) {
                Ok(r) => r.url,
                Err(_) => {
                    eprintln!("Error: could not determine instance URL. Use --url or set it in config.");
                    eprintln!("错误：无法确定实例 URL。使用 --url 或在配置中设置。");
                    std::process::exit(1);
                }
            }
        }
    };

    println!("Profile:  {profile}");
    println!("URL:      {url}");
    match session {
        kuayle_sdk::session::Session::Pat { .. } => {
            println!("Auth:     Personal Access Token");

            // Validate the token.
            // 验证 token。
            let base_url = match Url::parse(&url) {
                Ok(u) => u,
                Err(_) => {
                    println!("Status:   (could not validate — invalid URL)");
                    return;
                }
            };

            let client = Client::new(base_url, session.bearer_token().to_string());
            match client.get::<UserResponse>("/api/auth/me").await {
                Ok(user) => {
                    println!("Status:   ✓ authenticated as {} ({})", user.display_name, user.email);
                }
                Err(e) => {
                    println!("Status:   ✗ token invalid or expired ({e})");
                    std::process::exit(2);
                }
            }
        }
    }
}

// ── whoami ────────────────────────────────────────────────────────

async fn cmd_whoami(cli: &Cli) {
    let resolved = match resolve_client(cli).await {
        Ok((client, _url)) => client,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(2);
        }
    };

    match resolved.get::<UserResponse>("/api/auth/me").await {
        Ok(user) => {
            if is_json_output(cli) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&user).unwrap_or_default()
                );
            } else {
                println!("ID:           {}", user.id);
                println!("Name:         {}", user.name);
                println!("Display Name: {}", user.display_name);
                println!("Email:        {}", user.email);
                println!("Sysadmin:     {}", user.is_sysadmin);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(e.exit_code());
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────

/// Resolve a Client from current config and stored credentials.
/// 从当前配置和已存储凭据解析 Client。
async fn resolve_client(cli: &Cli) -> Result<(Client, String), String> {
    let config = Config::load_or_default();
    let resolved = config
        .resolve(cli.profile.as_deref(), cli.url.as_deref(), None)
        .map_err(|e| format!("config: {e}"))?;

    let store =
        creds::get_credential_store().map_err(|e| format!("credential store: {e}"))?;

    let session = store
        .load(&resolved.profile)
        .map_err(|e| format!("load session: {e}"))?
        .ok_or_else(|| {
            "not logged in. Run 'kuayle auth login' to authenticate.".to_string()
        })?;

    let base_url = Url::parse(&resolved.url).map_err(|e| format!("invalid URL: {e}"))?;
    let client = Client::new(base_url, session.bearer_token().to_string());

    Ok((client, resolved.url))
}

/// Save a profile URL to config.toml (auto-creates if needed).
/// 将 profile URL 保存到 config.toml（如需要则自动创建）。
fn save_profile_to_config(profile: &str, url: &str) -> Result<(), String> {
    let path = crate::config::config_path()?;
    let mut config = Config::load_from(&path)?;

    let entry = config
        .profiles
        .entry(profile.to_string())
        .or_insert_with(|| crate::config::ProfileConfig {
            url: url.to_string(),
            workspace: None,
        });
    entry.url = url.to_string();

    if config.default_profile.is_none() {
        config.default_profile = Some(profile.to_string());
    }

    // Write back.
    // 写回。
    let toml_str = toml::to_string_pretty(&config).map_err(|e| format!("serialize: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create config dir: {e}"))?;
    }
    std::fs::write(&path, toml_str).map_err(|e| format!("write config: {e}"))?;

    Ok(())
}

/// Check if JSON output is requested.
/// 检查是否请求 JSON 输出。
fn is_json_output(cli: &Cli) -> bool {
    match cli.format.as_str() {
        "json" => true,
        "human" => false,
        _ => !std::io::IsTerminal::is_terminal(&std::io::stdout()),
    }
}
