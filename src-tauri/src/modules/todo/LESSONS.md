# todo LESSONS

## [2026-04-11] tauri-specta command 函数必须 pub，不能 pub(super)

**症状**: `collect_commands![]` 在 lib.rs 中引用 command 函数时，编译报错 `could not find __cmd__list_todos in modules` 和 `could not find __specta__fn__list_todos in modules`

**根因**: `#[tauri::command]` + `#[specta::specta]` proc macro 会在函数旁边生成隐藏的伴生符号（`__cmd__*`, `__specta__fn__*`），这些符号的可见性和函数本身一致。当函数声明为 `pub(super)` 时，`pub use commands::*` 无法将这些隐藏符号 re-export 到上层模块，导致 `collect_commands![]` 找不到它们。

**解决**: command 函数用 `pub fn`（不是 `pub(super)`）。lib.rs 中用完整路径 `modules::todo::commands::list_todos` 引用，而不是依赖 glob re-export。

**影响范围**: 所有 `#[tauri::command]` + `#[specta::specta]` 函数。新增模块时同样适用。
