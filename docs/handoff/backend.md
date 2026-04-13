---
area: backend
last_updated: 2026-04-14T02:00:00+08:00
session_id: 35ea0d27
status: done
stale_check: cargo test --manifest-path src-tauri/Cargo.toml --lib keysight::domain::edge 2>&1 | grep "test result"
---

# Handoff: backend

## 状态:无活跃 task

Phase A(Keysight Edge 判别联合重构)已在本 session 全部落地,共 4 commit,无工作树残留。
下一个 Active task 是 Phase B(节点菜单能力类型化),当前**未开工**,起步上下文保留在
下方「Phase B 排队上下文」小节。

## Phase A 完成时的成果摘要

| 子阶段 | Commit | 改动要点 |
|---|---|---|
| rename | `cddecc4` | `models::Edge` → `EdgeRow`(DB 行 DTO,让名给新判别联合) |
| 1 type sketch | `90d727a` | 新增 `domain/edge.rs` —— 6 newtype id + `EntityId` + `ObsidianLink` + 8 Edge 变体 + 7 parse 单测,零业务 impl |
| 2a connect 强类型化 | `be7ea8b` | `EntityGraph::connect(&Edge)` + `user_draw_edge` 意图函数 + 绷带退场(删 `resolve_user_drawn_edge_type`)+ `entity_relate` 命令 + TS GraphView 同步 |
| 2b reader 修复 | `675f764` | `note.rs`/`alias.rs` reader 穷尽 match + `GraphNote`/`CardAlias` 扩展 Q/T 字段 + `buildEdges.ts` 渲染新链接 |

**防火墙机制落地点**:
- Section/Task 作 from 编译期不可表达(Edge 枚举无对应变体)
- 四个通用 link 变体的 `to` 用 `EntityId`,reader 必须穷尽 match 6 类 variant,禁止 `_` 通配
- SeeAlso 的 `to` 是 `ObsidianLink` newtype,与 entity id 类型层隔离
- Alias 的独立 edge 能力仅 `AliasLink`,Related/SeeAlso 靠 `CardToAlias` 反查 owning card 继承

**测试计数**:Rust 198 → 211(+13 新),TS 209 不变。

## 验证基线

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib keysight::domain::edge 2>&1 | grep "test result"
# 期望: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 199 filtered out; finished in 0.00s
```

```bash
git log --oneline -6
# 期望 HEAD:
# 675f764 feat(keysight): Phase A 子阶段 2b — reader 穷尽 match + Note/Alias 支持 Q/T 链接
# be7ea8b refactor(keysight): Phase A 子阶段 2a — connect 签名强类型化 + 绷带退场
# 90d727a feat(keysight): Phase A type sketch — Edge 判别联合骨架（无业务 impl）
# cddecc4 refactor(keysight): rename models::Edge → EdgeRow 区分 DB 行投影
```

## Phase B 排队上下文(保留给下一 session 起步使用)

**Phase B — Keysight 节点菜单能力类型化** —— 对应 `docs/progress/backend.md` > Active

**触发背景**:2026-04-13 session 修 Question 菜单时发现 Card/Note/Alias/Question/Task 五类
节点的 ⋯ 菜单能力是按 kind 分散硬编码在 `src/components/keysight/nodes/NodeContextMenu.tsx`
五个分支里,缺统一视图:

- 能力空白看不出来(Question 之前完全没菜单,直到 2026-04-13 session 才补上)
- 能力重复看不出来(Move to Section / Remove from group 在 Card / Alias / Note / Question 四份重复声明)
- 共享能力(Draw connection / Move to Section / Delete)和专属能力(Card 的 Create alias / Note 的 SetColor)没分离

**目标**:把「五类节点的 ⋯ 菜单能力」做成强类型 catalog —— 让能力是显式的、共享的能力
只声明一次、能力空白被强制可见。

**起点参考文件**:
- `src/components/keysight/nodes/NodeContextMenu.tsx` —— 当前的 `NodeMenuConfig` 判别联合(Card/Alias/Note/Question/Section 五个 variant)
- `src/components/keysight/nodes/EntityNode.tsx::NodeContextMenuHandlers` —— 当前的扁平 handler 集合
- `src/components/keysight/GraphView.tsx::menuHandlers useMemo` —— 当前的具体实现集中地

**Phase A 已提供的基础**:
- 已有 `EntityId` 判别联合(`domain/edge.rs`),Phase B 如果需要「按 EntityId 派发菜单能力」可以直接用
- `Edge` 判别联合可用作「菜单项触发的 edge 类型」类型签名
- user_draw_edge / entity_relate 已经是 Phase B 菜单能力里 "Draw connection" / "Related" 的后端对接点

**Phase B 启动第一步**:把 `NodeContextMenu.tsx` 五个 variant 里的菜单项做交集 + 差集分析,
列出共享能力清单 vs 专属能力清单 —— 这是 Phase B type sketch 的输入

## 可选的遗留技术债(不影响 Phase B)

以下三条是 Phase A 推进过程中刻意留下的小尾巴,可在 Phase B 或后续 session 顺手处理:

1. **`disconnect` 签名仍是 stringly typed** —— Phase A 子阶段 2a 决策保留 `disconnect(from: &str, to: &str, edge_type: EdgeType)` 旧签名,因为 TS 侧无 disconnect 调用且投入收益比不高。当 reader 能力到位后可统一升级到 `disconnect(&Edge)`。

2. **`CardToAlias` 与 `alias_fields.card_id` 的一致性** —— 目前是 alias 继承机制的反查路径,与 alias_fields 表的 card_id 看起来有冗余。子阶段 2 的 doc 已标 TODO,等遇到实际维护成本再决定是否合并。

3. **`state.db.lock().unwrap()` 的 `// 例外:` 注释** —— CLAUDE.md L0 建模优先章节要求务实例外带 `// 例外:` 注释,但 `commands.rs` 整个文件共 31 处 `.lock().unwrap()` 都没写。要么一次性补齐,要么加模块级 doc 覆盖约定。Phase A 没动。
