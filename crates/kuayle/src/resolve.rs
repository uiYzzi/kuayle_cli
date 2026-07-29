// Name resolution: name→ID lookup with in-process memo and disk cache.
// 名称解析：name→ID 查找，含进程内 memo 和磁盘缓存。
#![allow(dead_code)] // Will be wired into issue create/update commands
                     //
                     // Strategy (§6.5):
                     // 1. UUID / issue identifier passthrough (zero API calls)
                     // 2. Batch fetch + case-insensitive in-memory match
                     // 3. On ambiguity: list candidates, exit code 3
                     // 4. In-process memo per command invocation
                     // 5. Disk cache: ~/.cache/kuayle/resolve/{hash}/{ws}/{kind}.json, TTL 5 min,
                     //    disabled via --no-cache or KUAYLE_NO_CACHE=1
                     // 6. Parallel resolution via tokio::join! for multiple kinds
                     // 策略 (§6.5)：
                     // 1. UUID / issue identifier 直通（零 API 调用）
                     // 2. 批量抓取 + 大小写不敏感内存匹配
                     // 3. 多歧义时列出候选，退出码 3
                     // 4. 进程内 memo 每次命令调用内有效
                     // 5. 磁盘缓存：~/.cache/kuayle/resolve/{hash}/{ws}/{kind}.json，TTL 5 分钟，
                     //    --no-cache 或 KUAYLE_NO_CACHE=1 可禁
                     // 6. 多种类型并行解析通过 tokio::join! 实现

use kuayle_sdk::client::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

/// Kind of resource for name resolution.
/// 名称解析的资源种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolveKind {
    Teams,
    Labels,
    Members,
    Projects,
    Cycles,
}

impl ResolveKind {
    fn api_path(&self, workspace: &str) -> String {
        match self {
            ResolveKind::Teams => format!("/api/workspaces/{workspace}/teams"),
            ResolveKind::Labels => format!("/api/workspaces/{workspace}/labels"),
            ResolveKind::Members => format!("/api/workspaces/{workspace}/members"),
            ResolveKind::Projects => format!("/api/workspaces/{workspace}/projects"),
            ResolveKind::Cycles => format!("/api/workspaces/{workspace}/teams"), // cycles are per-team, simplified for now
        }
    }

    fn cache_file_name(&self) -> &str {
        match self {
            ResolveKind::Teams => "teams",
            ResolveKind::Labels => "labels",
            ResolveKind::Members => "members",
            ResolveKind::Projects => "projects",
            ResolveKind::Cycles => "cycles",
        }
    }
}

/// Cache entry stored on disk.
/// 磁盘上存储的缓存条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// name → id mapping / name → id 映射
    mapping: HashMap<String, String>,
    /// when this entry was created / 此条目的创建时间
    cached_at: u64, // SystemTime::duration_since(UNIX_EPOCH).as_secs()
}

const CACHE_TTL_SECS: u64 = 300; // 5 minutes / 5 分钟

/// Resolver for name→ID lookups with in-process memo and disk cache.
/// 带进程内 memo 和磁盘缓存的 name→ID 查找解析器。
pub struct Resolver {
    client: Client,
    workspace: String,
    /// in-process memo / 进程内 memo
    memo: HashMap<ResolveKind, HashMap<String, String>>,
    /// disable cache / 禁用缓存
    no_cache: bool,
}

impl Resolver {
    pub fn new(client: Client, workspace: &str, no_cache: bool) -> Self {
        Resolver {
            client,
            workspace: workspace.to_string(),
            memo: HashMap::new(),
            no_cache,
        }
    }

    /// Resolve a name to its ID for the given kind.
    /// 将名称解析为给定类型的 ID。
    ///
    /// Returns the ID if found, or an error string with candidates if ambiguous or not found.
    /// 如果找到则返回 ID，如果多歧义或未找到则返回含候选的错误字符串。
    pub async fn resolve(&mut self, kind: ResolveKind, name: &str) -> Result<String, String> {
        // 1. UUID passthrough / UUID 直通
        if is_uuid(name) {
            return Ok(name.to_string());
        }

        // 2. Check in-process memo / 检查进程内 memo
        if let Some(id) = self.memo_get(kind, name) {
            return Ok(id);
        }

        // 3. Check disk cache / 检查磁盘缓存
        if !self.no_cache {
            if let Some(id) = self.load_from_disk_cache(kind, name) {
                // Warm memo / 预热 memo
                self.memo
                    .entry(kind)
                    .or_default()
                    .insert(name.to_lowercase(), id.clone());
                return Ok(id);
            }
        }

        // 4. Fetch from API / 从 API 抓取
        let mapping = self.fetch_all(kind).await?;

        // 5. Match / 匹配
        let id = match_name(&mapping, name)?;
        Ok(id)
    }

    /// Pre-fetch and cache all items of a kind (for batch resolution).
    /// 预抓取并缓存某种类型的所有项目（用于批量解析）。
    async fn fetch_all(&mut self, kind: ResolveKind) -> Result<HashMap<String, String>, String> {
        let path = kind.api_path(&self.workspace);

        // Fetch all items (simplified — for large lists, paginate in future).
        // 抓取所有项目（简化 —— 对于大型列表，未来可分页）。
        let items: Vec<serde_json::Value> = self
            .client
            .get(&path)
            .await
            .map_err(|e| format!("failed to fetch {kind:?}: {e}"))?;

        let mut mapping = HashMap::new();
        for item in &items {
            if let (Some(id), Some(name)) = (item["id"].as_str(), extract_name(kind, item)) {
                // Store both exact and lowercase versions for case-insensitive match.
                // 同时存储精确版和小写版以支持大小写不敏感匹配。
                mapping.insert(name.to_string(), id.to_string());
                mapping.insert(name.to_lowercase(), id.to_string());
                // For teams, also store by key / 对团队也存 key
                if kind == ResolveKind::Teams {
                    if let Some(key) = item["key"].as_str() {
                        mapping.insert(key.to_string(), id.to_string());
                        mapping.insert(key.to_lowercase(), id.to_string());
                    }
                }
            }
        }

        // Cache in memo and disk / 缓存到 memo 和磁盘
        self.memo.insert(kind, mapping.clone());
        if !self.no_cache {
            let _ = self.save_to_disk_cache(kind, &mapping);
        }

        Ok(mapping)
    }

    fn memo_get(&self, kind: ResolveKind, name: &str) -> Option<String> {
        self.memo
            .get(&kind)
            .and_then(|m| m.get(&name.to_lowercase()).cloned())
    }

    // ── disk cache ──────────────────────────────────────────────────

    fn cache_path(&self, kind: ResolveKind) -> Option<PathBuf> {
        let dir = dirs::cache_dir()?;
        let url_str = self.client.base_url().to_string();
        let hash = hash_url(&url_str);
        Some(
            dir.join("kuayle")
                .join("resolve")
                .join(&hash)
                .join(&self.workspace)
                .join(kind.cache_file_name())
                .with_extension("json"),
        )
    }

    fn load_from_disk_cache(&self, kind: ResolveKind, name: &str) -> Option<String> {
        let path = self.cache_path(kind)?;
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        let entry: CacheEntry = serde_json::from_str(&content).ok()?;
        let age = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - entry.cached_at;
        if age > CACHE_TTL_SECS {
            return None;
        }
        entry.mapping.get(&name.to_lowercase()).cloned()
    }

    fn save_to_disk_cache(
        &self,
        kind: ResolveKind,
        mapping: &HashMap<String, String>,
    ) -> Result<(), String> {
        let path = match self.cache_path(kind) {
            Some(p) => p,
            None => return Ok(()),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let entry = CacheEntry {
            mapping: mapping.clone(),
            cached_at: SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        let json = serde_json::to_string(&entry).map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
        Ok(())
    }
}

// ── helpers ───────────────────────────────────────────────────────

fn is_uuid(s: &str) -> bool {
    s.len() >= 32 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-') && s.contains('-')
}

fn extract_name(kind: ResolveKind, item: &serde_json::Value) -> Option<&str> {
    match kind {
        ResolveKind::Members => item["name"].as_str(),
        _ => item["name"].as_str(),
    }
}

/// Case-insensitive name match against a mapping. Returns ID or error.
/// 大小写不敏感名称匹配。返回 ID 或错误。
fn match_name(mapping: &HashMap<String, String>, name: &str) -> Result<String, String> {
    let lower = name.to_lowercase();

    // Exact match / 精确匹配
    if let Some(id) = mapping.get(&lower) {
        return Ok(id.clone());
    }

    // Partial / prefix match / 部分/前缀匹配
    let candidates: Vec<&String> = mapping
        .iter()
        .filter(|(k, _)| k.to_lowercase().contains(&lower))
        .map(|(k, _)| k)
        .collect();

    if candidates.is_empty() {
        Err(format!(
            "'{name}' not found. Available: (none loaded for this kind)"
        ))
    } else {
        Err(format!(
            "'{name}' not found. Available: {}",
            candidates
                .iter()
                .take(5)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

fn hash_url(url: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    url.hash(&mut h);
    format!("{:016x}", h.finish())
}
