use crate::app_error::AppError;

#[derive(Debug, thiserror::Error)]
pub(super) enum TodoError {
    #[error("Title cannot be empty")]
    EmptyTitle,

    #[error("Todo not found: {0}")]
    NotFound(i64),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

impl From<TodoError> for AppError {
    fn from(e: TodoError) -> Self {
        AppError::Todo(e.to_string())
    }
}
