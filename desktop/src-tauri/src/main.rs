#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use finch_core::data_postgres::{finch_migrations, FinchDataModule};
use finch_core::settings::{SettingValue, SettingsRepository};
use finch_core::{run_command, CliCommand};
use nest_cache::Cache;
use nest_cache_file::{FileCacheAdapter, FileCacheConfig};
use nest_data::DataModule;
use nest_data_postgres::PostgresDataModule;
use nest_image::ImageModule;
use nest_tauri::{NestHostState, TauriApp};
use nest_theme::ThemeModule;

// This desktop app is a thin client. The same command surface lives in
// `crates/core` and is reused by the CLI and TUI surfaces.
#[tauri::command]
async fn run_cli(command: CliCommand) -> Result<String, String> {
    run_command(command)
}

#[tauri::command]
async fn schwab_auth_begin() -> Result<String, String> {
    finch_core::schwab::auth_begin().await
}

#[tauri::command]
async fn schwab_auth_complete(code: String, state: String) -> Result<String, String> {
    finch_core::schwab::auth_complete(&code, &state).await
}

#[tauri::command]
async fn schwab_auth_status() -> Result<String, String> {
    finch_core::schwab::auth_status().await
}

#[tauri::command]
async fn schwab_accounts() -> Result<Vec<finch_core::schwab::SchwabAccount>, String> {
    finch_core::schwab::list_accounts().await
}

#[tauri::command]
#[allow(non_snake_case)]
async fn schwab_account_summary(
    accountHash: String,
) -> Result<finch_core::schwab::AccountSummary, String> {
    finch_core::schwab::account_summary(&accountHash).await
}

#[tauri::command]
#[allow(non_snake_case)]
async fn schwab_orders(accountHash: String) -> Result<Vec<finch_core::schwab::OrderRow>, String> {
    finch_core::schwab::list_orders(&accountHash).await
}

#[tauri::command]
#[allow(non_snake_case)]
async fn schwab_positions(
    accountHash: String,
) -> Result<Vec<finch_core::schwab::PositionRow>, String> {
    finch_core::schwab::list_positions(&accountHash).await
}

#[tauri::command]
async fn settings_get(
    state: tauri::State<'_, NestHostState>,
    key: String,
) -> Result<Option<SettingValue>, String> {
    let repo = state
        .context
        .service::<SettingsRepository>()
        .map_err(|e| e.to_string())?;
    repo.get(&key).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn settings_set(
    state: tauri::State<'_, NestHostState>,
    key: String,
    value: SettingValue,
) -> Result<(), String> {
    let repo = state
        .context
        .service::<SettingsRepository>()
        .map_err(|e| e.to_string())?;
    repo.set(&key, value).await.map_err(|e| e.to_string())
}

fn main() {
    let cache_root = std::env::temp_dir().join("finch-cache");
    let cache = Cache::new(Arc::new(
        FileCacheAdapter::new(FileCacheConfig::new(&cache_root))
            .expect("failed to open image cache directory"),
    ));

    let postgres_module = match PostgresDataModule::from_env("DATABASE_URL") {
        Ok(module) => module.with_migrations(finch_migrations()),
        Err(error) => {
            eprintln!("Failed to configure PostgreSQL: {error}");
            eprintln!("Make sure DATABASE_URL is set (e.g. via .env) and the database exists.");
            std::process::exit(1);
        }
    };

    TauriApp::new("finch")
        .module(DataModule)
        .module(postgres_module)
        .module(FinchDataModule)
        .module(ThemeModule::default())
        .module(ImageModule::with_cache(cache))
        .with_builder(|builder| {
            builder.plugin(
                tauri::plugin::Builder::<tauri::Wry>::new("finch")
                    .invoke_handler(tauri::generate_handler![
                        run_cli,
                        schwab_auth_begin,
                        schwab_auth_complete,
                        schwab_auth_status,
                        schwab_accounts,
                        schwab_account_summary,
                        schwab_orders,
                        schwab_positions,
                        settings_get,
                        settings_set,
                    ])
                    .build(),
            )
        })
        .run(tauri::generate_context!());
}
