# TodoMVC Design Spec

## Goal

Build a TodoMVC app to validate the Tauri + React + rusqlite architecture defined in CLAUDE.md. This is the first real feature — it proves the Deep Module pattern, tauri-specta IPC, and "TS only does UI, Rust owns business" constraint.

## Data Model

### Todo (SQLite row)

| Column | Type | Constraint |
|--------|------|-----------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT |
| title | TEXT | NOT NULL |
| completed | INTEGER | NOT NULL DEFAULT 0 (boolean) |
| created_at | TEXT | NOT NULL DEFAULT (datetime('now')) ISO 8601 |

### Rust structs

```rust
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub completed: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub enum TodoFilter {
    All,
    Active,
    Completed,
}
```

## Module Structure (Deep Module)

```
src-tauri/src/
├── lib.rs              # Tauri Builder + State<Mutex<Connection>> + specta
├── app_error.rs        # Top-level AppError (thiserror + Serialize + specta::Type)
├── modules/
│   ├── mod.rs          # pub use todo::commands; pub use todo::models;
│   └── todo/
│       ├── mod.rs      # pub use commands::*; pub use models::*;
│       ├── models.rs   # Todo, TodoFilter (pub — crosses IPC)
│       ├── errors.rs   # TodoError (pub(super), Into<AppError>)
│       ├── db.rs       # init_db(conn) — CREATE TABLE IF NOT EXISTS
│       ├── domain.rs   # 7 pure functions + #[cfg(test)] mod tests
│       └── commands.rs # 7 thin #[tauri::command] + #[specta::specta]
```

## Command Contracts (7 commands)

### list_todos(filter: TodoFilter) -> Vec<Todo>
- Returns all/active/completed todos ordered by created_at DESC

### create_todo(title: String) -> Todo
- Trims title, rejects empty → TodoError::EmptyTitle
- INSERTs, returns the created Todo with id

### update_todo(id: i64, title: String) -> Todo
- Trims title, rejects empty
- UPDATEs title WHERE id, returns updated Todo
- Not found → TodoError::NotFound(id)

### toggle_todo(id: i64) -> Todo
- Flips completed boolean, returns updated Todo
- Not found → TodoError::NotFound(id)

### delete_todo(id: i64) -> ()
- DELETEs WHERE id
- Not found → TodoError::NotFound(id)

### toggle_all(completed: bool) -> Vec<Todo>
- SETs all todos to completed value, returns full list

### clear_completed() -> Vec<Todo>
- DELETEs all completed todos, returns remaining list

## Error Hierarchy

```
AppError (top-level, Serialize + specta::Type)
  └── TodoError (thiserror, pub(super))
        ├── EmptyTitle
        ├── NotFound(i64)
        └── Database(String)  // rusqlite::Error mapped
```

## React Components

| Component | Responsibility |
|-----------|---------------|
| App | Route filter state, orchestrate layout |
| TodoHeader | Input field, calls commands.createTodo |
| TodoList | Renders filtered list, toggle-all checkbox |
| TodoItem | Single todo: display, edit, toggle, delete |
| TodoFooter | Active count, filter links, clear completed |

All components import from `src/bindings.ts`. No raw `invoke()`.

## Test Strategy

- **Unit tests** (domain.rs): in-memory SQLite, test each of 7 functions for happy + error paths
- **Bindings test**: `cargo test export_bindings` verifies bindings.ts generation
- **TS type check**: `pnpm build` (tsc + vite) catches IPC type mismatches

## Verification Checklist

- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` — all domain tests + export_bindings pass
- [ ] `pnpm build` — TS compiles with no errors
- [ ] bindings.ts contains all 7 typed command wrappers
- [ ] TodoMVC renders, CRUD operations work end-to-end
- [ ] No raw `invoke()` in TS code
- [ ] No business logic in TS (id generation, validation, etc.)
