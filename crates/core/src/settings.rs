//! User settings storage: a typed key/value table backed by PostgreSQL.

use nest_data::{DataError, DataErrorKind, DataResult, SqlMigration};
use serde_json::Value;
use sqlx::Row;

use nest_data_postgres::PostgresConnection;

fn sqlx_to_data_error(err: sqlx::Error) -> DataError {
    use sqlx::Error;
    match err {
        Error::RowNotFound => DataError::not_found("setting not found"),
        Error::PoolTimedOut | Error::PoolClosed => {
            DataError::connection_error("database pool unavailable").with_source(err)
        }
        _ => DataError::query("database query failed").with_source(err),
    }
}

/// Migration that creates the `settings` table.
pub fn settings_migration() -> SqlMigration {
    SqlMigration::new(
        "001_create_settings",
        "CREATE TABLE settings (
            key TEXT PRIMARY KEY NOT NULL,
            value_type TEXT NOT NULL CHECK (value_type IN ('string', 'integer', 'float', 'boolean', 'json')),
            value TEXT NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );",
        "DROP TABLE settings;",
    )
}

/// Known setting keys. Using constants prevents typos and documents what
/// settings the app persists.
pub mod keys {
    /// Active theme identifier (string).
    pub const THEME_ID: &str = "theme.id";
    /// Default Schwab account hash (string).
    pub const DEFAULT_ACCOUNT_HASH: &str = "account.default_hash";
    /// Last selected chart period, e.g. "1y" (string).
    pub const CHART_PERIOD: &str = "chart.period";
    /// Last selected chart interval, e.g. "1d" (string).
    pub const CHART_INTERVAL: &str = "chart.interval";
    /// Active chart studies (json).
    pub const CHART_STUDIES: &str = "chart.studies";
}

/// A typed setting value.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum SettingValue {
    /// String value.
    String(String),
    /// 64-bit signed integer.
    Integer(i64),
    /// 64-bit floating point number.
    Float(f64),
    /// Boolean value.
    Boolean(bool),
    /// Arbitrary JSON value.
    Json(Value),
}

impl SettingValue {
    fn value_type(&self) -> &'static str {
        match self {
            SettingValue::String(_) => "string",
            SettingValue::Integer(_) => "integer",
            SettingValue::Float(_) => "float",
            SettingValue::Boolean(_) => "boolean",
            SettingValue::Json(_) => "json",
        }
    }

    fn to_storage(&self) -> String {
        match self {
            SettingValue::String(v) => v.clone(),
            SettingValue::Integer(v) => v.to_string(),
            SettingValue::Float(v) => v.to_string(),
            SettingValue::Boolean(v) => v.to_string(),
            SettingValue::Json(v) => v.to_string(),
        }
    }
}

/// PostgreSQL-backed user settings repository.
pub struct SettingsRepository {
    db: PostgresConnection,
}

impl SettingsRepository {
    /// Creates a repository over the given connection.
    pub fn new(db: PostgresConnection) -> Self {
        Self { db }
    }

    /// Returns a raw setting row, or `None` if the key is absent.
    pub async fn get_raw(&self, key: &str) -> DataResult<Option<(String, String)>> {
        let row = sqlx::query("SELECT value_type, value FROM settings WHERE key = $1")
            .bind(key)
            .fetch_optional(self.db.pool())
            .await
            .map_err(sqlx_to_data_error)?;
        Ok(row.map(|row| (row.get("value_type"), row.get("value"))))
    }

    /// Returns a typed setting value, or `None` if the key is absent.
    pub async fn get(&self, key: &str) -> DataResult<Option<SettingValue>> {
        let Some((value_type, value)) = self.get_raw(key).await? else {
            return Ok(None);
        };
        parse_value(&value_type, &value).map(Some)
    }

    /// Returns a string setting, or `None` if absent or not a string.
    pub async fn get_string(&self, key: &str) -> DataResult<Option<String>> {
        match self.get(key).await? {
            Some(SettingValue::String(v)) => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    /// Returns an integer setting, or `None` if absent or not an integer.
    pub async fn get_integer(&self, key: &str) -> DataResult<Option<i64>> {
        match self.get(key).await? {
            Some(SettingValue::Integer(v)) => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    /// Returns a float setting, or `None` if absent or not a float.
    pub async fn get_float(&self, key: &str) -> DataResult<Option<f64>> {
        match self.get(key).await? {
            Some(SettingValue::Float(v)) => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    /// Returns a boolean setting, or `None` if absent or not a boolean.
    pub async fn get_boolean(&self, key: &str) -> DataResult<Option<bool>> {
        match self.get(key).await? {
            Some(SettingValue::Boolean(v)) => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    /// Returns a JSON setting, or `None` if absent or not JSON.
    pub async fn get_json(&self, key: &str) -> DataResult<Option<Value>> {
        match self.get(key).await? {
            Some(SettingValue::Json(v)) => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    /// Sets a typed value for the given key.
    pub async fn set(&self, key: &str, value: SettingValue) -> DataResult<()> {
        sqlx::query(
            "INSERT INTO settings (key, value_type, value, updated_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (key)
             DO UPDATE SET value_type = EXCLUDED.value_type,
                           value = EXCLUDED.value,
                           updated_at = NOW()",
        )
        .bind(key)
        .bind(value.value_type())
        .bind(value.to_storage())
        .execute(self.db.pool())
        .await
        .map_err(sqlx_to_data_error)?;
        Ok(())
    }

    /// Removes a setting.
    pub async fn delete(&self, key: &str) -> DataResult<()> {
        sqlx::query("DELETE FROM settings WHERE key = $1")
            .bind(key)
            .execute(self.db.pool())
            .await
            .map_err(sqlx_to_data_error)?;
        Ok(())
    }
}

fn parse_value(value_type: &str, value: &str) -> DataResult<SettingValue> {
    match value_type {
        "string" => Ok(SettingValue::String(value.to_string())),
        "integer" => value
            .parse::<i64>()
            .map(SettingValue::Integer)
            .map_err(|e| DataError::new(DataErrorKind::Query, format!("invalid integer setting: {e}"))),
        "float" => value
            .parse::<f64>()
            .map(SettingValue::Float)
            .map_err(|e| DataError::new(DataErrorKind::Query, format!("invalid float setting: {e}"))),
        "boolean" => value
            .parse::<bool>()
            .map(SettingValue::Boolean)
            .map_err(|e| DataError::new(DataErrorKind::Query, format!("invalid boolean setting: {e}"))),
        "json" => serde_json::from_str(value)
            .map(SettingValue::Json)
            .map_err(|e| DataError::new(DataErrorKind::Query, format!("invalid json setting: {e}"))),
        other => Err(DataError::new(
            DataErrorKind::Query,
            format!("unknown setting value_type: {other}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_values_round_trip() {
        assert_eq!(
            parse_value("string", "dark").unwrap(),
            SettingValue::String("dark".into())
        );
        assert_eq!(
            parse_value("integer", "42").unwrap(),
            SettingValue::Integer(42)
        );
        assert_eq!(
            parse_value("float", "3.14").unwrap(),
            SettingValue::Float(3.14)
        );
        assert_eq!(
            parse_value("boolean", "true").unwrap(),
            SettingValue::Boolean(true)
        );
        assert_eq!(
            parse_value("json", r#"{"volume":true}"#).unwrap(),
            SettingValue::Json(serde_json::json!({"volume": true}))
        );
    }

    #[test]
    fn storage_round_trip() {
        let values = vec![
            SettingValue::String("dark".into()),
            SettingValue::Integer(42),
            SettingValue::Float(3.14),
            SettingValue::Boolean(true),
            SettingValue::Json(serde_json::json!({"volume": true})),
        ];
        for original in values {
            let stored = original.to_storage();
            let parsed = parse_value(original.value_type(), &stored).unwrap();
            assert_eq!(original, parsed);
        }
    }
}
