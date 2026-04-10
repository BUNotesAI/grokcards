use std::sync::Mutex;

use rusqlite::Connection;
use tauri::State;

use super::domain;
use super::models::{Todo, TodoFilter};
use crate::app_error::AppError;

/// 根据筛选条件返回 todo 列表。
#[tauri::command]
#[specta::specta]
pub fn list_todos(db: State<'_, Mutex<Connection>>, filter: TodoFilter) -> Result<Vec<Todo>, AppError> {
    let conn = db.lock().unwrap();
    domain::list_todos(&conn, &filter).map_err(Into::into)
}

/// 创建新 todo。
#[tauri::command]
#[specta::specta]
pub fn create_todo(db: State<'_, Mutex<Connection>>, title: String) -> Result<Todo, AppError> {
    let conn = db.lock().unwrap();
    domain::create_todo(&conn, &title).map_err(Into::into)
}

/// 更新指定 todo 的标题。
#[tauri::command]
#[specta::specta]
pub fn update_todo(db: State<'_, Mutex<Connection>>, id: i64, title: String) -> Result<Todo, AppError> {
    let conn = db.lock().unwrap();
    domain::update_todo(&conn, id, &title).map_err(Into::into)
}

/// 切换指定 todo 的完成状态。
#[tauri::command]
#[specta::specta]
pub fn toggle_todo(db: State<'_, Mutex<Connection>>, id: i64) -> Result<Todo, AppError> {
    let conn = db.lock().unwrap();
    domain::toggle_todo(&conn, id).map_err(Into::into)
}

/// 删除指定 todo。
#[tauri::command]
#[specta::specta]
pub fn delete_todo(db: State<'_, Mutex<Connection>>, id: i64) -> Result<(), AppError> {
    let conn = db.lock().unwrap();
    domain::delete_todo(&conn, id).map_err(Into::into)
}

/// 将所有 todo 设为指定的完成状态。
#[tauri::command]
#[specta::specta]
pub fn toggle_all(db: State<'_, Mutex<Connection>>, completed: bool) -> Result<Vec<Todo>, AppError> {
    let conn = db.lock().unwrap();
    domain::toggle_all(&conn, completed).map_err(Into::into)
}

/// 清除所有已完成的 todo。
#[tauri::command]
#[specta::specta]
pub fn clear_completed(db: State<'_, Mutex<Connection>>) -> Result<Vec<Todo>, AppError> {
    let conn = db.lock().unwrap();
    domain::clear_completed(&conn).map_err(Into::into)
}
