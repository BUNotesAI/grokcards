# Phase 5e: 多白板 — 子白板卡 + 白板切换 + 视口持久化

## 概述

根白板（wb_root）上渲染子白板卡片，点击进入子白板，支持返回。每个白板的画布视口（pan/zoom）持久化到 localStorage。子白板卡片位置存 positions 表，和普通实体统一管理。

## 约束

- **单层结构**：只有 `wb_root → sub-whiteboard`，不嵌套
- **wb_root 是正常白板**：可以有任何实体（cards/notes/sections/...），子白板卡和实体共存
- **导航模型**：同页面 useState 切换，不走 URL 路由
- **数据规模**：实体可能达几千个，接口需轻量

---

## 1. 数据层（Rust）

### 1.1 新增 `WhiteboardSummary` 模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct WhiteboardSummary {
    pub whiteboard_id: String,
    pub cards: i64,
    pub notes: i64,
    pub sections: i64,
    pub aliases: i64,
    pub tasks: i64,
    pub questions: i64,
}
```

放入 `models.rs`，derive `specta::Type` 跨 IPC。

### 1.2 新增 domain 函数 `list_whiteboards`

位置：`domain/overview.rs`（和现有 `graph_overview` 同模块）

```sql
SELECT whiteboard_id,
       SUM(CASE WHEN kind = 'card' THEN 1 ELSE 0 END) AS cards,
       SUM(CASE WHEN kind = 'note' THEN 1 ELSE 0 END) AS notes,
       SUM(CASE WHEN kind = 'section' THEN 1 ELSE 0 END) AS sections,
       SUM(CASE WHEN kind = 'alias' THEN 1 ELSE 0 END) AS aliases,
       SUM(CASE WHEN kind = 'task' THEN 1 ELSE 0 END) AS tasks,
       SUM(CASE WHEN kind = 'question' THEN 1 ELSE 0 END) AS questions
FROM entities
WHERE whiteboard_id != 'wb_root'
GROUP BY whiteboard_id
ORDER BY whiteboard_id
```

签名：`pub(super) fn list_whiteboards(conn: &Connection) -> Result<Vec<WhiteboardSummary>, KeysightError>`

### 1.3 新增 command

```rust
#[tauri::command]
#[specta::specta]
pub fn whiteboard_list(state: State<KeysightState>) -> Result<Vec<WhiteboardSummary>, AppError> {
    let conn = state.db.lock().unwrap();
    overview::list_whiteboards(&conn).map_err(Into::into)
}
```

注册到 `lib.rs` 的 `collect_commands![]`，`cargo test export_bindings` 更新 bindings.ts。

### 1.4 子白板卡的 positions 约定

子白板卡在 positions 表中的 entity_id 使用约定前缀：`"wb:{whiteboard_id}"`

| entity_id | whiteboard_id | x | y |
|-----------|--------------|---|---|
| `wb:rust` | `wb_root` | 100 | 100 |
| `wb:chentian` | `wb_root` | 460 | 100 |

使用现有的 `layout_set_position` / `layout_query_positions` command 读写，无需新增接口。

---

## 2. 前端架构

### 2.1 白板切换状态

GraphView 新增：

```typescript
const [currentWhiteboardId, setCurrentWhiteboardId] = useState("wb_root");
```

切换触发 `useWhiteboardData` 和视口加载/保存。

### 2.2 数据流

```
currentWhiteboardId = "wb_root" 时：
  useWhiteboardData("wb_root")    → wb_root 的实体
  useQuery(["whiteboards"])       → whiteboard_list → WhiteboardSummary[]
  positions("wb_root")            → 包含 "wb:*" 前缀的子白板卡位置
  合并渲染：实体节点 + 子白板卡节点

currentWhiteboardId = "rust" 时：
  useWhiteboardData("rust")       → rust 的实体
  不请求 whiteboard_list          → 无子白板卡（单层）
```

### 2.3 WhiteboardNode 组件

新建 `src/components/keysight/nodes/WhiteboardNode.tsx`

**Props：**

```typescript
interface WhiteboardNodeProps {
  summary: WhiteboardSummary;
  onNavigate: (whiteboardId: string) => void;
}
```

**视觉设计**（Clean Elevated 风格，比旧 Obsidian 版更精致）：

- 白底圆角卡片，左侧带渐变色竖条（紫色系，区别于其他节点类型）
- 标题行：白板图标 + 名称
- 统计行：`145 cards · 72 notes · 6 aliases` 格式，只显示非零项
- 空白板显示 "empty"
- 底部：淡色 "Click to enter →" 提示
- 宽度 320px
- 点击整个卡片触发 `onNavigate(summary.whiteboard_id)`

### 2.4 渲染层次

GraphView 中分两层渲染，子白板卡也参与视口裁剪：

```tsx
{/* 正常实体 */}
{visibleEntities.map(e => <EntityNode key={e.id} ... />)}

{/* 子白板卡 — 仅 wb_root */}
{currentWhiteboardId === "wb_root" &&
  visibleWhiteboards.map(wb => <WhiteboardNode key={wb.whiteboard_id} ... />)}
```

子白板卡的 ENTITY_DIMENSIONS 新增：`whiteboard: { width: 320, height: 160 }`

### 2.5 导航交互

- **进入子白板**：点击 WhiteboardNode → `setCurrentWhiteboardId(id)`
- **返回根白板**：Toolbar 显示 `← Root` 按钮 → `setCurrentWhiteboardId("wb_root")`
- **Toolbar 动态显示**：
  - `wb_root` 时：不显示返回按钮（已在根）
  - 子白板时：替换原 disabled "Root" 为 `← Root` 可点击按钮 + 当前白板名

---

## 3. 视口持久化

### 3.1 存储方案

localStorage，key 格式 `keysight:viewport:{whiteboard_id}`

```json
{ "panX": -200, "panY": -100, "zoom": 0.8 }
```

### 3.2 useViewport 改造

接口变化：

```typescript
// 之前
useViewport(containerRef)

// 之后
useViewport(containerRef, whiteboardId)
```

行为：
- **初始化**：从 localStorage 读取对应白板视口，无则默认 `{ panX: 0, panY: 0, zoom: 1 }`
- **保存**：视口变化时 500ms debounce 写 localStorage
- **白板切换**：`whiteboardId` 变化时，保存当前视口 → 加载目标白板视口

### 3.3 无位置子白板卡的初始布局

首次打开根白板时，`whiteboard_list` 返回的子白板可能没有 positions 记录。前端检测到后：

- 在现有实体 bounding box 之外，按网格排列（2 列，间距 40px）
- 调用 `layout_set_position` 逐个写入 DB
- 只执行一次，后续从 positions 读取

---

## 4. 测试策略

### 4.1 Rust 测试（domain 纯函数）

| 测试 | 覆盖 |
|------|------|
| `list_whiteboards` happy path | 多白板正确统计，wb_root 排除 |
| `list_whiteboards` 空库 | 无实体 → 空 Vec |
| `list_whiteboards` 仅 wb_root | 所有实体在 wb_root → 空 Vec |

### 4.2 TS 组件测试

| 测试 | 覆盖 |
|------|------|
| WhiteboardNode 渲染 | 给定 summary，显示名称 + 非零统计 |
| WhiteboardNode 点击 | 点击触发 onNavigate |
| WhiteboardNode 空白板 | 全零统计显示 "empty" |
| Toolbar 导航 — 根白板 | 不显示返回按钮 |
| Toolbar 导航 — 子白板 | 显示 `← Root` + 白板名，点击触发 onBack |

### 4.3 不测

- localStorage 读写（浏览器 API，mock 是 test theater）
- GraphView 白板切换集成（多 hook 联动，手动验证）
- 初始布局坐标值（手动验证视觉）

### 4.4 手动验证清单

1. 根白板同时显示实体 + 子白板卡（rust/chentian/rust-examples/agent）
2. 子白板卡显示正确统计
3. 点击 rust 卡 → 切换为 rust 白板
4. Toolbar 显示 `← Root`，点击返回
5. 返回后视口位置不变
6. 进入 rust → 拖动画布 → 返回 → 再进入 → 位置保持

---

## 5. 副作用矩阵更新

| 写操作 | 影响的表/资源 | 副作用 | 测试覆盖 |
|--------|-------------|--------|---------|
| `layout_set_position`（子白板卡） | `positions` | DB 写 | 已有 domain unit test |

无新增写操作 — `whiteboard_list` 是纯读取，子白板卡位置复用已有的 `layout_set_position`。

---

## 6. 文件变更清单

### Rust（新增/修改）

| 文件 | 变更 |
|------|------|
| `models.rs` | 新增 `WhiteboardSummary` struct |
| `domain/overview.rs` | 新增 `list_whiteboards()` 函数 |
| `commands.rs` | 新增 `whiteboard_list` command |
| `mod.rs` | 无变更（commands 已 pub） |
| `lib.rs` | `collect_commands![]` 添加 `whiteboard_list` |

### 前端（新增/修改）

| 文件 | 变更 |
|------|------|
| `nodes/WhiteboardNode.tsx` | **新建** — 子白板卡组件 |
| `GraphView.tsx` | 添加白板切换 state + 子白板卡渲染 + 导航回调 |
| `hooks/useWhiteboardData.ts` | 新增 `useWhiteboardList` query（仅 wb_root 时启用） |
| `useViewport.ts` | 添加 `whiteboardId` 参数 + localStorage 持久化 |
| `GraphToolbar.tsx` | 替换 disabled "Root" 为动态导航按钮 |
| `types.ts` | 新增 `whiteboard` 的 ENTITY_DIMENSIONS |

### 测试

| 文件 | 变更 |
|------|------|
| `domain/overview.rs` 内 `#[cfg(test)]` | 新增 3 个 `list_whiteboards` 测试 |
| `src/__tests__/components/WhiteboardNode.test.tsx` | **新建** — 3 个测试 |
| `src/__tests__/components/GraphToolbar.test.tsx` | 新增 2 个导航测试 |
