# Card title 反斜杠转义 bug — codex 接手任务

> 用 codex CLI 在 `/Users/alexwang/codes/vibe-coding/super-tauri` 根目录运行。
> 直接把本文件作为 prompt 交给 codex。

## 任务

修复 keysight 卡片 title 中残留 `\<`, `\>` 等反斜杠转义的 bug。修复后系统**永远不应**对 title 字符做 markdown 转义；只有 `**bold**` 应被识别为 markdown。其他所有字符必须在 SQLite ↔ markdown 文件 ↔ UI ↔ 用户输入之间原样往返。

## 项目背景

Tauri v2 + Rust + React 应用，路径：`/Users/alexwang/codes/vibe-coding/super-tauri`。
keysight 模块是从 Obsidian 插件 keysight 迁移到独立 app 的过程产物。Vault markdown 文件是 source of truth，SQLite 是镜像。

## Bug 现象

用户在运行的 app 里双击编辑 `card_db4cda0d` 的 title。textarea 显示字面文本 `**Arc\<T\>** 原子引用计数...` — `<` 和 `>` 前面有反斜杠。用户期望看到 `**Arc<T>** 原子引用计数...`（无反斜杠），写回时也**不应**产生反斜杠。

DB 验证（`sqlite3 ~/Library/Application\ Support/co.bunotes.super-tauri/keysight.db`）：

```
sqlite> SELECT title FROM entities WHERE id='card_db4cda0d';
**Arc\<T\> 原子引用计数** — Rc 的线程安全版
```

DB 里的对应文件路径：`whiteboard/rust/【ATC】Arc T 原子引用计数 — Rc 的线程安全版.md`。
Vault 根目录来自环境变量 `KEYSIGHT_VAULT_PATH`。

**第一步**：读这个 env var，找到磁盘上实际的 md 文件，看它是否已经包含反斜杠 — 这决定了 bug 在 parser 端（导入时引入）还是 writer 端（之前调 card_edit_title 时引入）。

## 调查线索

关键代码路径：

- `src-tauri/src/modules/keysight/parser.rs` — `extract_title_and_body` 从 H1 行 `# title` 读 title 并 trim `【ATC】` 等标记。看起来不显式做转义，但 YAML frontmatter 处理可能产生副作用
- `src-tauri/src/modules/keysight/domain/sync.rs` — `sync_file` 调用 parser 并 UPSERT entity（含 title）
- `src-tauri/src/modules/keysight/domain/legacy_import.rs` — 另一条 ingest 路径，从 legacy SQLite DB 导入。用户的数据最初是从这里导入的
- `src-tauri/src/modules/keysight/domain/card.rs` — `edit_title` 通过 `vault_fs` 写回。看 `new_title` 在 INSERT 之前是否被任何字符串变换过
- `src-tauri/src/modules/keysight/parser.rs` 里如果有 `insert_h1_title` 或类似的写回 helper

在 `src-tauri/src/modules/keysight` 下 grep `replace`、`escape`、反斜杠序列可能没匹配 — 那是预期的，如果转义来自 YAML serializer 或上游 Obsidian 数据。准备好查看磁盘文件来定位来源。

## "修复"的标准

### 1. 找根因

明确说明：反斜杠在磁盘 markdown 文件里、还是只在 SQLite、还是两边都有？给出 disk 文件相关内容（前几行就够）。

### 2. 修读路径

如果 parser 引入了反斜杠：`extract_title_and_body` 返回的 title 字符串（被存到 `entities.title`）必须含原始 `<` 和 `>`，绝不是 `\<` `\>`。

在 `parser.rs` 加一个回归测试，输入 H1 行：

```
# 【ATC】**Arc\<T\>** 原子引用计数
```

断言解析后 title 等于 `**Arc<T>** 原子引用计数`（原始字符，无反斜杠）。

对其他可能被转义的标点同样处理：`\[`, `\]`, `\(`, `\)`, `\|`, `\#`, `\*`。**注意**：如果 `\*` 是 `**` 配对的一部分**不能**反转义；只有单独的 `\` + 单字符 ASCII 标点才被剥离。`**` 加粗定界符必须完整保留。

### 3. 修写路径

`card_edit_title` 和任何其他写 title 的 command 都必须把用户输入原样写进 markdown 文件，不做反斜杠转义。

加一个回归测试：调 `edit_title` 传 `**Arc<T>**`，读回文件，断言文件含原始字符串（无 `\<`）。

### 4. 清理已有数据

提供一次性 SQL 或 domain 函数，从所有 `kind='card'` 行的 `entities.title` 里剥离 bogus 转义序列（`\<`, `\>`, `\[`, `\]`, `\(`, `\)`, `\|`, `\#`）。**不要**剥离 `\\*` 或 `\\`，那些是真实 markdown。打印更新行数。**先备份 DB**。

### 5. Code Review note

遵守项目 TDD 纪律 — 先写 Red 测试，确认失败，再实现。用户是交互式的，不能因为 bug 小就跳步。

## 硬规则

- title 里只支持 `**bold**` 这一种 markdown。parse 时剥离字面 `\<`, `\>`, `\[`, `\]`, `\(`, `\)`, `\|`, `\#`（以及大概 `\` + 任意非 `*` 的 ASCII 标点），write 时永不产生它们
- 这次修复**不要**碰 card content/body 或 note content — 只动 `entities.title`。Body markdown 可以保留 CommonMark 风格的 escape 语义
- 所有现有测试必须保持 green。在宣布完成前从项目根目录跑：
  - `cd src-tauri && cargo test --lib modules::keysight`
  - `pnpm test`
- 用户运行的 app 读 `~/Library/Application Support/co.bunotes.super-tauri/keysight.db`。修复落地后，针对该 DB 跑一次数据迁移，让用户重启 app 立即看到清理后的 title。备份文件名带时间戳

## 项目约定（来自 CLAUDE.md）

- doc comment 和行内注释用**中文**
- 所有有副作用的 pub fn 写 Operation Contract（前置条件 / 执行效果 / 不做的事 / 幂等性 / 关联操作）
- 测试真实代码路径 — 拒绝 test theater。`domain/` 下的纯函数可单元测；store 测用 `:memory:` SQLite
- TDD 强制 Red 关卡，实现前先确认测试失败
- 落地后追加一行到 `~/Documents/obsidian_workspace/agent-slipbox-v3/logs/changelog/2026-04-12.md`，状态 `✅`，描述本 fix

## 交付物

完成后报告：

1. 反斜杠从哪来（parser？writer？legacy import？上游 Obsidian 数据？）
2. 你新增/修改了哪些纯函数来修复
3. 新测试列表 + 各自的断言
4. 你跑的数据迁移命令 + 改了多少行
5. 最终测试计数（`cargo test` + `pnpm test`）
6. 任何不那么显眼的后续点（比如其他可能需要类似处理的地方）

## 给人的提示

跑 codex 之前可以先在终端确认 vault 路径：

```bash
echo $KEYSIGHT_VAULT_PATH
ls "$KEYSIGHT_VAULT_PATH/whiteboard/rust/" | grep "Arc"
```

然后 codex 应该能直接读到那个 md 文件。
