# super-tauri

基于 Tauri v2 的桌面知识管理应用，从 Obsidian 插件 [keysight](https://github.com/BUNotesAI/obsidian-plugin-keysight) 独立演化而来。

核心功能：管理 Obsidian vault 中的知识卡片（Card）、笔记（Note）、任务（Task）、问题（Question）等实体，提供白板画布（Canvas）视图和看板（Kanban）视图。

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | React 19 + TypeScript + Tailwind CSS v4 + shadcn/ui |
| 后端 | Rust (edition 2024) + Tauri v2 |
| 数据库 | SQLite (rusqlite, WAL mode) |
| IPC 类型安全 | tauri-specta（编译期生成 TS bindings） |
| CLI | keysight-cli（clap，查询直连 SQLite，写操作走 HTTP IPC） |
| HTTP IPC | axum（CLI → Tauri 主进程通信） |
| 测试 | Rust: cargo test / TS: Vitest + React Testing Library |

## 前置条件

- [Node.js](https://nodejs.org/) >= 20
- [pnpm](https://pnpm.io/)
- [Rust](https://rustup.rs/) (stable)
- Tauri v2 系统依赖（macOS 需要 Xcode Command Line Tools）

## 快速开始

```bash
# 安装前端依赖
pnpm install

# 开发模式（同时启动 Vite dev server + 编译 Rust 后端 + 打开应用窗口）
KEYSIGHT_VAULT_PATH=~/Documents/obsidian_workspace/agent-slipbox-v3 pnpm tauri dev
```

### Vault 路径解析（三级 fallback）

启动时按优先级解析 vault 路径：

1. **环境变量 `KEYSIGHT_VAULT_PATH`**（仅 debug build 生效）— 开发时最方便，release build 忽略此变量以防 shell 环境污染
2. **`{app_data_dir}/config.json`** — 持久化配置，首次通过 VaultSetup 对话框写入后自动记住
3. **都没有** — 进入 first-run flow，前端弹出 VaultSetup 对话框让用户选择 vault 目录

开发时可在 shell profile 中 export 避免每次手敲：

```bash
# ~/.zshrc 或 ~/.bashrc
export KEYSIGHT_VAULT_PATH=~/Documents/obsidian_workspace/agent-slipbox-v3
```

生产构建（release）不读环境变量，完全依赖 config.json 或首次启动的 VaultSetup 对话框。

### 数据目录

所有应用数据存放在 macOS 标准 `Application Support` 目录下：

```
~/Library/Application Support/co.bunotes.super-tauri/
├── keysight.db          # 知识管理主数据库（Card/Note/Task/Question/Edge/Position 等）
├── super_tauri.db       # Todo 模块数据库
├── config.json          # 应用配置（vault_path 等）
├── cli-config.toml      # CLI 配置（keysight-cli 读取）
└── cli-endpoint.toml    # HTTP IPC 端点信息（Tauri 运行时写入，CLI 读取 port + token）
```

- 两个 SQLite 数据库均使用 WAL 模式，支持多进程并发读
- `cli-endpoint.toml` 仅在 Tauri 主进程运行期间有效，CLI 据此连接 HTTP IPC
- 数据库备份文件（`*.bak-*`、`*.backup-*`）由 legacy import 或 schema migration 自动生成

## 常用命令

```bash
# 开发
pnpm tauri dev                    # 开发模式运行

# 构建
pnpm tauri build                  # 生产构建（打包 .app / .dmg）
pnpm tauri build --no-bundle      # 生产构建（仅编译，不打包）

# Rust
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings   # lint
cargo test --workspace --manifest-path src-tauri/Cargo.toml                     # 测试

# 前端
pnpm build                        # TypeScript 类型检查 + Vite 构建
pnpm test                         # Vitest 组件测试（单次运行）
pnpm test:watch                   # Vitest watch 模式

# IPC 类型绑定（修改 Rust command 签名后必须运行）
cargo test export_bindings --manifest-path src-tauri/Cargo.toml
```

## 项目结构

```
super-tauri/
├── src/                          # React 前端
│   ├── components/               # UI 组件
│   │   ├── kanban/               #   看板视图
│   │   ├── keysight/             #   白板画布 + 侧边栏
│   │   └── ui/                   #   shadcn/ui 基础组件
│   ├── pages/                    # 页面路由
│   ├── bindings.ts               # tauri-specta 自动生成（勿手动编辑）
│   └── __tests__/                # TS 组件测试
├── src-tauri/                    # Tauri Rust 后端
│   ├── src/
│   │   ├── lib.rs                # Tauri Builder + specta 注册
│   │   ├── app_error.rs          # 统一错误类型
│   │   └── modules/              # 业务模块（Deep Module 模式）
│   │       ├── keysight/         #   核心知识管理模块
│   │       ├── todo/             #   待办事项模块
│   │       └── config/           #   应用配置模块
│   ├── capabilities/             # Tauri v2 权限声明
│   └── tests/                    # Rust 集成测试
├── keysight-core/                # 共享 domain 层（Tauri + CLI 共用）
│   └── src/
│       ├── domain/               #   业务纯函数（15 个子模块）
│       │                         #   id.rs newtype 集中定义 / migration.rs schema 演进 /
│       │                         #   alias·card·note·section·task·question·entity·edge·
│       │                         #   layout·sync·whiteboard·overview·legacy_import
│       ├── models.rs             #   数据类型
│       └── parser.rs             #   Markdown/YAML frontmatter 解析
└── keysight-cli/                 # CLI 工具
    └── src/
        ├── main.rs               #   clap 入口（RootCommand 16 顶层 + 嵌套子组 ≈ 29 命令）
        └── commands.rs           #   命令实现
```

## 架构概要

**TS 只负责 UI，Rust 独占业务逻辑。** 所有数据写入通过 Rust → SQLite，TS 侧不持有持久业务状态。

- **IPC 类型安全**：每个 `#[tauri::command]` 同时标记 `#[specta::specta]`，自动生成 `bindings.ts`。TS 侧从 bindings import typed commands，不使用 raw `invoke()`
- **双端 branded ID**：7 个 entity id（WhiteboardId / CardId / NoteId / AliasId / SectionId / QuestionId / TaskId）+ EntityId 判别联合，Rust 端是 newtype（prefix 校验 + `#[serde(try_from = "String")]` + `ToSql`），IPC 反序列化边界自动 reject illegal prefix；TS 端通过 ts-morph 后处理把 `bindings.ts` 中对应 type alias 改写为 `string & { __brand: "XId" }`，编译期挡 id 类型混用与参数顺序写反
- **Deep Module 模式**：每个业务模块（`keysight/`、`todo/`）对外暴露窄接口（commands + models），内部实现（domain、db、errors）对外不可见
- **CLI 双路径**：查询命令直连 SQLite（WAL 多进程读）；写命令走 localhost HTTP IPC 委托 Tauri 主进程处理

## IDE 推荐配置

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## License

Private
