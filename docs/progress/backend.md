# backend

## Active

- [ ] **Keysight Edge 判别联合重构（Phase A）** — 把 `EntityGraph::connect(from: &str, to: &str, edge_type: EdgeType, ...)` 的三个 stringly typed 参数改成接 `Edge` 判别联合 + `EntityId` 强类型 id，让 Note→Question/Task 等非法组合**编译期就死**，彻底消除 d29d6df 的绷带 fix 和当前残留的 silent orphan edge 风险。详见 CLAUDE.md L0「建模优先 + 强类型」+ 踩坑样例 1，以及 obsidian「附加约束：建模优先 + 强类型（防火墙模型）」节。

## Next

- [ ] **Keysight 节点菜单能力类型化（Phase B）** — Card/Note/Alias/Question/Task 五类节点的 ⋯ 菜单能力做成强类型 catalog（共享能力 vs 专属能力分离）。当前 `NodeContextMenu` 的菜单项按 kind 分散硬编码在五个分支里，五类菜单到底有什么能力没有统一视图，能力空白（Question 之前完全没菜单）和能力重复（Move to Section / Remove from group 散在四份）都看不出来。

## Done

### 2026-04-13
- [x] Keysight Note↔Note 连线静默失败 bug fix（绷带版） + Question 节点 ⋯ 菜单四层接线（commit d29d6df）
- [x] Keysight Note 改成文件写回链路，并在启动时迁移 DB-only notes 到 `whiteboard/`（含 DB + whiteboard 备份）
- [x] Keysight Question 补齐文件型 create/update/delete 后端与回归测试
- [x] Keysight entity connect/disconnect 写回 card/note 源文件，避免 DB 与 markdown 漂移

### 2026-04-11
- [x] 定义领域模型（entities + value objects）
- [x] 抽取 command handler 业务逻辑到 domain 模块
