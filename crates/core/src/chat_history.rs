//! Per-symbol AI chat history, grouped into sessions.
//!
//! Each symbol can have many sessions over time. New messages always append
//! to the most recent session; "start fresh" begins a new one rather than
//! deleting anything, so past sessions stay browsable via the history picker.

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

/// Maximum messages returned for a single session — enough for any
/// reasonable conversation without an unbounded response payload.
const MAX_SESSION_MESSAGES: i64 = 500;

/// Maximum sessions returned per symbol in the history picker.
const MAX_SESSIONS: i64 = 100;

/// Characters kept from the first user message as a session preview.
const PREVIEW_CHARS: i64 = 120;

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

/// Migration that groups messages into sessions: a new `ai_chat_sessions`
/// table, a `session_id` column on `ai_chat_messages`, and a backfill that
/// anchors one session per existing symbol at that symbol's earliest
/// message, so history saved before this migration stays intact and
/// browsable as that symbol's first session.
pub fn chat_sessions_migration() -> SqlMigration {
    SqlMigration::new(
        "003_add_ai_chat_sessions",
        "CREATE TABLE ai_chat_sessions (
            id BIGSERIAL PRIMARY KEY,
            symbol TEXT NOT NULL,
            started_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE INDEX ai_chat_sessions_symbol_started_at_idx
            ON ai_chat_sessions (symbol, started_at);

        ALTER TABLE ai_chat_messages
            ADD COLUMN session_id BIGINT REFERENCES ai_chat_sessions(id) ON DELETE CASCADE;

        INSERT INTO ai_chat_sessions (symbol, started_at)
        SELECT symbol, MIN(created_at) FROM ai_chat_messages GROUP BY symbol;

        UPDATE ai_chat_messages m
        SET session_id = s.id
        FROM ai_chat_sessions s
        WHERE m.symbol = s.symbol AND m.session_id IS NULL;

        ALTER TABLE ai_chat_messages ALTER COLUMN session_id SET NOT NULL;

        CREATE INDEX ai_chat_messages_session_id_created_at_idx
            ON ai_chat_messages (session_id, created_at);",
        "ALTER TABLE ai_chat_messages DROP COLUMN session_id;
        DROP TABLE ai_chat_sessions;",
    )
}

/// One persisted chat message.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ChatMessageRow {
    /// Row id.
    pub id: i64,
    /// Session this message belongs to.
    pub session_id: i64,
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

/// One past conversation, as shown in the history picker.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SessionSummary {
    /// Session id.
    pub id: i64,
    /// Symbol this session belongs to.
    pub symbol: String,
    /// When the session began.
    pub started_at: DateTime<Utc>,
    /// Number of messages in the session.
    pub message_count: i64,
    /// The first user message, truncated — a preview for the picker. `None`
    /// for a session that was started but never asked anything.
    pub preview: Option<String>,
}

/// PostgreSQL-backed AI chat history, grouped into sessions per symbol.
#[derive(Clone)]
pub struct ChatHistoryRepository {
    db: PostgresConnection,
}

impl ChatHistoryRepository {
    /// Creates a repository over the given connection.
    pub fn new(db: PostgresConnection) -> Self {
        Self { db }
    }

    /// Appends one message to `symbol`'s current (most recent) session,
    /// creating a session first if none exists yet.
    pub async fn append_to_current_session(
        &self,
        symbol: &str,
        role: &str,
        content: &str,
    ) -> DataResult<ChatMessageRow> {
        let session_id = match self.latest_session_id(symbol).await? {
            Some(id) => id,
            None => self.start_new_session(symbol).await?,
        };

        let row = sqlx::query(
            "INSERT INTO ai_chat_messages (session_id, symbol, role, content)
             VALUES ($1, $2, $3, $4)
             RETURNING id, session_id, symbol, role, content, created_at",
        )
        .bind(session_id)
        .bind(symbol)
        .bind(role)
        .bind(content)
        .fetch_one(self.db.pool())
        .await
        .map_err(sqlx_to_data_error)?;

        Ok(row_to_message(row))
    }

    /// Starts a new session for `symbol` and returns its id. Subsequent
    /// appends go to this session until another one is started.
    pub async fn start_new_session(&self, symbol: &str) -> DataResult<i64> {
        let row = sqlx::query("INSERT INTO ai_chat_sessions (symbol) VALUES ($1) RETURNING id")
            .bind(symbol)
            .fetch_one(self.db.pool())
            .await
            .map_err(sqlx_to_data_error)?;
        Ok(row.get("id"))
    }

    async fn latest_session_id(&self, symbol: &str) -> DataResult<Option<i64>> {
        let row = sqlx::query(
            "SELECT id FROM ai_chat_sessions WHERE symbol = $1 ORDER BY started_at DESC LIMIT 1",
        )
        .bind(symbol)
        .fetch_optional(self.db.pool())
        .await
        .map_err(sqlx_to_data_error)?;
        Ok(row.map(|row| row.get("id")))
    }

    /// Returns `symbol`'s current (most recent) session's messages, oldest
    /// first. Empty if the symbol has no sessions yet.
    pub async fn list_current_session(&self, symbol: &str) -> DataResult<Vec<ChatMessageRow>> {
        let Some(session_id) = self.latest_session_id(symbol).await? else {
            return Ok(Vec::new());
        };
        self.list_messages_for_session(session_id).await
    }

    /// Returns one session's messages, oldest first.
    pub async fn list_messages_for_session(&self, session_id: i64) -> DataResult<Vec<ChatMessageRow>> {
        let rows = sqlx::query(
            "SELECT id, session_id, symbol, role, content, created_at
             FROM ai_chat_messages
             WHERE session_id = $1
             ORDER BY created_at ASC, id ASC
             LIMIT $2",
        )
        .bind(session_id)
        .bind(MAX_SESSION_MESSAGES)
        .fetch_all(self.db.pool())
        .await
        .map_err(sqlx_to_data_error)?;

        Ok(rows.into_iter().map(row_to_message).collect())
    }

    /// Returns `symbol`'s past sessions, most recent first, each with a
    /// message count and a preview of its first user message.
    pub async fn list_sessions_for_symbol(&self, symbol: &str) -> DataResult<Vec<SessionSummary>> {
        let rows = sqlx::query(
            "SELECT
                 s.id,
                 s.symbol,
                 s.started_at,
                 COUNT(m.id) AS message_count,
                 (
                     SELECT LEFT(content, $3::int)
                     FROM ai_chat_messages
                     WHERE session_id = s.id AND role = 'user'
                     ORDER BY created_at ASC, id ASC
                     LIMIT 1
                 ) AS preview
             FROM ai_chat_sessions s
             LEFT JOIN ai_chat_messages m ON m.session_id = s.id
             WHERE s.symbol = $1
             GROUP BY s.id
             ORDER BY s.started_at DESC
             LIMIT $2",
        )
        .bind(symbol)
        .bind(MAX_SESSIONS)
        .bind(PREVIEW_CHARS)
        .fetch_all(self.db.pool())
        .await
        .map_err(sqlx_to_data_error)?;

        Ok(rows
            .into_iter()
            .map(|row| SessionSummary {
                id: row.get("id"),
                symbol: row.get("symbol"),
                started_at: row.get("started_at"),
                message_count: row.get("message_count"),
                preview: row.get("preview"),
            })
            .collect())
    }
}

fn row_to_message(row: sqlx::postgres::PgRow) -> ChatMessageRow {
    ChatMessageRow {
        id: row.get("id"),
        session_id: row.get("session_id"),
        symbol: row.get("symbol"),
        role: row.get("role"),
        content: row.get("content"),
        created_at: row.get("created_at"),
    }
}
