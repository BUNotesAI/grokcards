# super-tauri 设计文档

## 技术栈

| 层 | 技术 | 版本 |
|---|---|---|
| Rust Backend | Tauri | 2.10.x |
| IPC 类型桥接 | tauri-specta | 2.x |
| Frontend | React | 19.x |
| Build | Vite | 7.x |
| Language | TypeScript | 5.8.x |

## 架构概览

```
┌─────────────────────────────────────────┐
│           React UI (src/)               │
│  components / hooks / pages             │
│  只负责渲染 + 纯 UI 状态                  │
│  从 bindings.ts import typed commands   │
├─────────────────────────────────────────┤
│        bindings.ts (auto-generated)     │
│  tauri-specta 从 Rust 生成              │
│  typed commands / events / types        │
├─────────────────────────────────────────┤
│        Tauri IPC Layer                  │
│  #[tauri::command] + #[specta::specta]  │
│  薄壳：参数解构 → 委托 domain 纯函数      │
├─────────────────────────────────────────┤
│        Rust Domain (src-tauri/src/)     │
│  业务逻辑 / 数据模型 / 持久化             │
│  所有 id 生成、校验、状态变更在此          │
└─────────────────────────────────────────┘
```

## 模块职责

### src-tauri/src/ (Rust)

| 模块 | 职责 |
|---|---|
| `lib.rs` | Tauri Builder 配置、command 注册、specta 绑定生成 |
| `commands/` | `#[tauri::command]` 薄壳，只做 IPC 胶水 |
| `domain/` | 业务纯函数，接收 plain 依赖，零框架依赖 |
| `models/` | 数据结构定义，derive `Serialize + Deserialize + specta::Type` |
| `errors.rs` | 统一错误类型，derive `thiserror + Serialize + specta::Type` |

### src/ (TypeScript/React)

| 模块 | 职责 |
|---|---|
| `bindings.ts` | tauri-specta 自动生成，禁止手动编辑 |
| `components/` | React 组件，纯 UI 渲染 |
| `hooks/` | 自定义 hooks，封装对 commands/events 的调用 |
| `App.tsx` | 根组件 |

## IPC 设计原则

1. **Rust = Source of Truth** — 所有类型定义在 Rust，通过 tauri-specta 生成 TS 类型
2. **Command = 薄壳** — `#[tauri::command]` 只做参数解构和调用委托，业务逻辑在 `domain/`
3. **TS 只消费** — TS 从 `bindings.ts` import typed commands，不用 raw `invoke()`
4. **Event 同样 typed** — Events 通过 `tauri_specta::Event` derive 宏实现类型安全

## 功能域

| 功能域 | 说明 | Progress 文件 |
|---|---|---|
| backend | Rust 业务逻辑（domain 模块） | `docs/progress/backend.md` |
| frontend | React UI 组件 | `docs/progress/frontend.md` |
| ipc | Command/Event 定义 + specta 配置 | `docs/progress/ipc.md` |
| infra | 构建、分发、CI/CD、权限配置 | `docs/progress/infra.md` |

## 推荐实施顺序

以下是架构规划参考，不是执行约束：

1. **Bootstrap** — tauri-specta 集成、项目管理体系、质量规则
2. **Core Domain** — 领域模型、业务逻辑、typed commands
3. **UI** — 组件架构、页面、交互
4. **Polish** — 权限最小化、CSP、分发配置
