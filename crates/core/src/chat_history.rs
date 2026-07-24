//! Per-symbol AI chat history: persists the Trade screen's chat panel so
//! returning to a symbol reloads past exchanges instead of starting blank.

use chrono::{DateTime, Utc};
use nest_data::{DataError, DataResult, SqlMigration};
use sqlx::Row;

use nest_data_postgres::PostgresConnection;

fn sqlx_to_data_error(err: sqlx::Error) -> DataError {
    use sqlx::Error;
    match err {
        Error::PoolTimedOut | Error::PoolClosed => {
            DataError::connection_error("database pool unavailable").with_source(err)
        }
        _ => DataError::query("database query failed").with_source(err),
    }
}

/// Maximum messages returned per symbol — enough history to be useful
/// without an unbounded response payload for symbols chatted about a lot.
const MAX_HISTORY_MESSAGES: i64 = 200;

/// Migration that creates the `ai_chat_messages` table.
pub fn chat_history_migration() -> SqlMigration {
    SqlMigration::new(
        "002_create_ai_chat_messages",
        "CREATE TABLE ai_chat_messages (
            id BIGSERIAL PRIMARY KEY,
            symbol TEXT NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'error')),
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX ai_chat_messages_symbol_created_at_idx
            ON ai_chat_messages (symbol, created_at);",
        "DROP TABLE ai_chat_messages;",
    )
}

/// One persisted chat message.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ChatMessageRow {
    /// Row id.
    pub id: i64,
    /// Symbol this message belongs to.
    pub symbol: String,
    /// `user`, `assistant`, or `error`.
    pub role: String,
    /// Message text (assistant messages include the same "thinking"/tool
    /// preamble the chat panel showed live, so history matches what you saw).
    pub content: String,
    /// When the message was recorded.
    pub created_at: DateTime<Utc>,
}

/// PostgreSQL-backed AI chat history, one row per message.
#[derive(Clone)]
pub struct ChatHistoryRepository {
    db: PostgresConnection,
}

impl ChatHistoryRepository {
    /// Creates a repository over the given connection.
    pub fn new(db: PostgresConnection) -> Self {
        Self { db }
    }

    /// Appends one message and returns the persisted row.
    pub async fn append(&self, symbol: &str, role: &str, content: &str) -> DataResult<ChatMessageRow> {
        let row = sqlx::query(
            "INSERT INTO ai_chat_messages (symbol, role, content)
             VALUES ($1, $2, $3)
             RETURNING id, symbol, role, content, created_at",
        )
        .bind(symbol)
        .bind(role)
        .bind(content)
        .fetch_one(self.db.pool())
        .await
        .map_err(sqlx_to_data_error)?;

        Ok(row_to_message(row))
    }

    /// Returns the most recent messages for `symbol`, oldest first.
    pub async fn list_for_symbol(&self, symbol: &str) -> DataResult<Vec<ChatMessageRow>> {
        let rows = sqlx::query(
            "SELECT id, symbol, role, content, created_at
             FROM (
                 SELECT id, symbol, role, content, created_at
                 FROM ai_chat_messages
                 WHERE symbol = $1
                 ORDER BY created_at DESC, id DESC
                 LIMIT $2
             ) recent
             ORDER BY created_at ASC, id ASC",
        )
        .bind(symbol)
        .bind(MAX_HISTORY_MESSAGES)
        .fetch_all(self.db.pool())
        .await
        .map_err(sqlx_to_data_error)?;

        Ok(rows.into_iter().map(row_to_message).collect())
    }

    /// Deletes all history for `symbol`.
    pub async fn clear_for_symbol(&self, symbol: &str) -> DataResult<()> {
        sqlx::query("DELETE FROM ai_chat_messages WHERE symbol = $1")
            .bind(symbol)
            .execute(self.db.pool())
            .await
            .map_err(sqlx_to_data_error)?;
        Ok(())
    }
}

fn row_to_message(row: sqlx::postgres::PgRow) -> ChatMessageRow {
    ChatMessageRow {
        id: row.get("id"),
        symbol: row.get("symbol"),
        role: row.get("role"),
        content: row.get("content"),
        created_at: row.get("created_at"),
    }
}
