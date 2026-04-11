# keysight

KeySight 功能从 Obsidian 插件迁移到 Tauri 独立应用。

## Active

## Next
- [ ] Phase 4: 文件监听（TDD + Trait-First，FileEventSource/Handler trait，🔴🟢 关卡）
- [ ] Phase 5: GraphView 移植（TS 测试基础设施 + TDD 组件测试，🔴🟢 关卡）
- [ ] Phase 6: UI 补全（每个组件 TDD，🔴🟢 关卡）

## Done

- [x] Phase 3: 一次性旧 DB 导入 — LegacyReader/Importer traits, 23 tests, app_data_dir 迁移, 真实数据验证 (145 cards, 0 skipped)
- [x] Phase 2: Tauri commands + specta 绑定 — 40 commands, KeysightState, Operation Contracts, 副作用矩阵

### 2026-04-11

- [x] Phase 1 补完: card mutations, FTS5 搜索, card links, section 跨白板迁移, graph overview（+20 tests, 104 total）
- [x] Phase 1: Rust domain 移植（102 tests, 10 domain modules, TDD + Trait-First）
