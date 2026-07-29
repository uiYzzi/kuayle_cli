// Resource registry and CRUD engine (§6.2).
// 资源注册表与 CRUD 引擎 (§6.2)。
#![allow(dead_code)] // Engine wiring in progress
//
// Declarative resource specs drive CLI command generation
// and usage docs. Each resource declares its REST path,
// ID kind, name resolution strategy, and capabilities.
// 声明式 resource spec 驱动 CLI 命令生成和 usage 文档。
// 每个资源声明其 REST 路径、ID 类型、名称解析策略和操作能力。

use bitflags::bitflags;

/// What kind of ID does a resource use for read/update/delete?
/// 资源在 read/update/delete 时使用什么类型的 ID？
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdKind {
    /// Standard UUID / 标准 UUID
    Uuid,
    /// Issue identifier like "ENG-25" / issue identifier 如 "ENG-25"
    IssueIdentifier,
    /// Cycle number (integer) / cycle 编号（整数）
    CycleNumber,
    /// Team-scoped status UUID / team 范围的 status UUID
    TeamStatusUuid,
}

bitflags! {
    /// Which CRUD operations a resource supports.
    /// 资源支持哪些 CRUD 操作。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Capabilities: u8 {
        const LIST   = 0b00001;
        const READ   = 0b00010;
        const CREATE = 0b00100;
        const UPDATE = 0b01000;
        const DELETE = 0b10000;
        /// All five standard CRUD operations.
        /// 全部五种标准 CRUD 操作。
        const CRUD   = Self::LIST.bits() | Self::READ.bits() | Self::CREATE.bits() | Self::UPDATE.bits() | Self::DELETE.bits();
        /// Read-only / 只读
        const READ_ONLY = Self::LIST.bits() | Self::READ.bits();
    }
}

/// A registered resource in the CLI.
/// CLI 中注册的资源。
pub struct ResourceSpec {
    /// Resource name (e.g. "teams", "projects") / 资源名称
    pub name: &'static str,
    /// Short description for --help / --help 中的简短描述
    pub about: &'static str,
    /// REST path template with {ws} placeholder / 含 {ws} 占位符的 REST 路径模板
    /// e.g. "/api/workspaces/{ws}/teams"
    pub path: &'static str,
    /// What kind of ID does this resource use? / 此资源使用什么类型的 ID？
    pub id_kind: IdKind,
    /// Capabilities supported / 支持的操作能力
    pub capabilities: Capabilities,
}

impl ResourceSpec {
    /// Build the API path for this resource, substituting workspace.
    /// 构建此资源的 API 路径，代入工作区。
    pub fn build_path(&self, workspace: &str) -> String {
        self.path.replace("{ws}", workspace)
    }

    /// Build the API path for a single resource by ID.
    /// 构建单个资源的 API 路径（按 ID）。
    pub fn build_item_path(&self, workspace: &str, id: &str) -> String {
        format!("{}/{}", self.build_path(workspace), id)
    }

    pub fn can_list(&self) -> bool {
        self.capabilities.contains(Capabilities::LIST)
    }
    pub fn can_read(&self) -> bool {
        self.capabilities.contains(Capabilities::READ)
    }
    pub fn can_create(&self) -> bool {
        self.capabilities.contains(Capabilities::CREATE)
    }
    pub fn can_update(&self) -> bool {
        self.capabilities.contains(Capabilities::UPDATE)
    }
    pub fn can_delete(&self) -> bool {
        self.capabilities.contains(Capabilities::DELETE)
    }
}

/// The static registry of all kuayle CLI resources.
/// kuayle CLI 所有资源的静态注册表。
///
/// Each entry drives CLI subcommand generation automatically.
/// 每个条目自动驱动 CLI 子命令生成。
pub static RESOURCES: &[ResourceSpec] = &[
    // ── Teams ──────────────────────────────────────────────────
    ResourceSpec {
        name: "teams",
        about: "Manage teams / 管理团队",
        path: "/api/workspaces/{ws}/teams",
        id_kind: IdKind::Uuid,
        // PAT: create/update/delete need team:manage; list/read need teams:read
        // Our token has teams:read only → read-only for now
        capabilities: Capabilities::READ_ONLY,
    },
    // ── Projects ───────────────────────────────────────────────
    ResourceSpec {
        name: "projects",
        about: "Manage projects / 管理项目",
        path: "/api/workspaces/{ws}/projects",
        id_kind: IdKind::Uuid,
        // PAT: create/update/delete need project:manage; list/read need projects:read
        capabilities: Capabilities::READ_ONLY,
    },
    // ── Cycles ─────────────────────────────────────────────────
    ResourceSpec {
        name: "cycles",
        about: "Manage cycles (read-only with PAT) / 管理周期（PAT 只读）",
        path: "/api/workspaces/{ws}/teams/{team}/cycles",
        id_kind: IdKind::CycleNumber,
        // PAT: only GET endpoints (list/get/burndown/velocity) — all sessionOnly for writes
        capabilities: Capabilities::READ_ONLY,
    },
    // ── Templates ──────────────────────────────────────────────
    ResourceSpec {
        name: "templates",
        about: "Manage issue templates / 管理 issue 模板",
        path: "/api/workspaces/{ws}/issue-templates",
        id_kind: IdKind::Uuid,
        // PAT: all operations scoped — issue:create (our token has this)
        capabilities: Capabilities::CRUD,
    },
    // ── Views ──────────────────────────────────────────────────
    ResourceSpec {
        name: "views",
        about: "Manage views (read-only with PAT) / 管理视图（PAT 只读）",
        path: "/api/workspaces/{ws}/views",
        id_kind: IdKind::Uuid,
        // PAT: only GET — all writes are sessionOnly
        capabilities: Capabilities::READ_ONLY,
    },
    // ── Members ────────────────────────────────────────────────
    ResourceSpec {
        name: "members",
        about: "Manage workspace members / 管理工作区成员",
        path: "/api/workspaces/{ws}/members",
        id_kind: IdKind::Uuid,
        // PAT: list needs members:read; update/remove need member:invite
        // Our token likely doesn't have member:invite → read-only
        capabilities: Capabilities::READ_ONLY,
    },
    // ── Favorites ──────────────────────────────────────────────
    ResourceSpec {
        name: "favorites",
        about: "Manage favorites (read-only with PAT) / 管理收藏（PAT 只读）",
        path: "/api/workspaces/{ws}/favorites",
        id_kind: IdKind::Uuid,
        // PAT: GET only — create/delete are sessionOnly
        capabilities: Capabilities::READ_ONLY,
    },
    // ── Notifications ──────────────────────────────────────────
    // Note: user-scoped, path is /api/notifications (no workspace)
    // 注意：用户级别，路径为 /api/notifications（无工作区）
    ResourceSpec {
        name: "notifications",
        about: "Manage notifications (read-only with PAT) / 管理通知（PAT 只读）",
        path: "/api/notifications",
        id_kind: IdKind::Uuid,
        // PAT: GET only — all writes are sessionOnly
        capabilities: Capabilities::READ_ONLY,
    },
    // ── Assets ─────────────────────────────────────────────────
    ResourceSpec {
        name: "assets",
        about: "Manage uploaded assets / 管理上传资源",
        path: "/api/workspaces/{ws}/assets",
        id_kind: IdKind::Uuid,
        // PAT: GET needs assets:read; upload needs issue:create
        capabilities: Capabilities::from_bits_truncate(
            Capabilities::LIST.bits() | Capabilities::READ.bits() | Capabilities::CREATE.bits(),
        ),
    },
];
