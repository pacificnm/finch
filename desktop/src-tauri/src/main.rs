#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use finch_core::chat_history::{ChatHistoryRepository, ChatMessageRow};
use finch_core::data_postgres::{finch_migrations, FinchDataModule};
use finch_core::settings::{SettingValue, SettingsRepository};
use finch_core::{run_command_async, CliCommand};
use nest_cache::Cache;
use nest_cache_file::{FileCacheAdapter, FileCacheConfig};
use nest_data::DataModule;
use nest_data_postgres::PostgresDataModule;
use nest_http_client::HttpClientModule;
use nest_image::ImageModule;
use nest_tauri::{NestHostState, TauriApp};
use nest_theme::ThemeModule;
use tauri::Emitter;

// This desktop app is a thin client. The same command surface lives in
// `crates/core` and is reused by the CLI and TUI surfaces.
#[tauri::command]
async fn run_cli(command: CliCommand) -> Result<String, String> {
    run_command_async(command).await
}

#[tauri::command]
async fn schwab_auth_begin() -> Result<String, String> {
    finch_core::schwab::auth_begin().await
}

#[tauri::command]
async fn schwab_auth_complete(code: String, state: String) -> Result<String, String> {
    finch_core::schwab::auth_complete(&code, &state).await
}

/// Opens Schwab's authorization URL in the user's system browser (Schwab
/// rejects logins attempted inside an embedded webview — "can't sign in"
/// with no further detail) and waits on the local HTTPS loopback listener
/// for the redirect, completing the OAuth exchange automatically. No code
/// copy/paste required; the browser will show a one-time warning for the
/// self-signed `127.0.0.1` redirect certificate, which is expected.
#[tauri::command]
async fn schwab_auth_login(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let result = finch_core::schwab::auth_login_loopback(|url| {
            // `open::that` blocks until the launcher (or the browser itself,
            // if it doesn't detach) exits, which can take as long as the
            // browser stays open — spawn it on a blocking thread instead of
            // awaiting it here, so the loopback listener below starts
            // immediately rather than waiting on the browser to close.
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(err) = open::that(&url) {
                    eprintln!("Failed to open system browser for Schwab login: {err}");
                }
            });
        })
        .await;
        let _ = app.emit("schwab-auth-result", result.err());
    });
    Ok(())
}

#[tauri::command]
async fn schwab_auth_status() -> Result<String, String> {
    finch_core::schwab::auth_status().await
}

#[tauri::command]
async fn schwab_auth_logout() -> Result<String, String> {
    finch_core::schwab::auth_logout().await
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

/// Chunk of a streamed AI chat answer, emitted as `ai-chat-chunk`.
#[derive(Clone, serde::Serialize)]
struct AiChatChunkEvent {
    request_id: String,
    delta: String,
}

/// Emitted as `ai-chat-done` once a streamed AI chat answer finishes.
#[derive(Clone, serde::Serialize)]
struct AiChatDoneEvent {
    request_id: String,
}

/// Emitted as `ai-chat-error` if a streamed AI chat answer fails.
#[derive(Clone, serde::Serialize)]
struct AiChatErrorEvent {
    request_id: String,
    message: String,
}

/// Streams an answer to a free-form question about `symbol` via
/// `ai-chat-chunk`/`ai-chat-done`/`ai-chat-error` events tagged with
/// `requestId`, so the frontend can match events to the question that
/// triggered them. Returns as soon as the streaming task is spawned —
/// callers must listen for the events rather than awaiting a return value.
#[tauri::command]
#[allow(non_snake_case)]
async fn ask_stock_question(
    app: tauri::AppHandle,
    state: tauri::State<'_, NestHostState>,
    symbol: String,
    question: String,
    requestId: String,
) -> Result<(), String> {
    use nest_ai_ollama::OllamaConfig;
    let config_service = state
        .context
        .service::<nest_config::ConfigService>()
        .map_err(|e| e.to_string())?;
    let ollama_config = OllamaConfig::from_config_service(&config_service)
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            let base_url = std::env::var("OLLAMA_HOST")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(|host| {
                    let host = host.trim().trim_end_matches('/');
                    if host.starts_with("http://") || host.starts_with("https://") {
                        host.to_string()
                    } else {
                        format!("http://{host}")
                    }
                })
                .unwrap_or_else(|| nest_ai_ollama::DEFAULT_BASE_URL.to_string());
            let model = std::env::var("OLLAMA_CHAT_MODEL")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "qwen3:14b-q4_K_M".to_string());
            // Mirrors config.toml's [ai] section — kept in sync as the
            // last-resort fallback when config.toml can't be loaded.
            OllamaConfig::new(base_url, model)
                .with_num_ctx(40960)
                .with_temperature(0.2)
                .with_think(true)
        });
    let http_client = state
        .context
        .service::<nest_http_client::HttpClientService>()
        .map_err(|e| e.to_string())?
        .clone();
    let chat_repo = state
        .context
        .service::<ChatHistoryRepository>()
        .map_err(|e| e.to_string())?
        .clone();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = chat_repo.append(&symbol, "user", &question).await {
            eprintln!("Failed to persist chat message (user): {err}");
        }

        // Persisted history should read the same as what was shown live —
        // including the "thinking" preamble and tool-use indicators — so
        // accumulate every chunk exactly as sent to the frontend.
        let transcript = Arc::new(Mutex::new(String::new()));
        let transcript_writer = transcript.clone();

        let result = finch_core::ai::ask_stock_question_stream(
            &ollama_config,
            &http_client,
            &symbol,
            &question,
            |delta| {
                transcript_writer
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push_str(&delta);
                let _ = app.emit(
                    "ai-chat-chunk",
                    AiChatChunkEvent {
                        request_id: requestId.clone(),
                        delta,
                    },
                );
            },
        )
        .await;
        match result {
            Ok(()) => {
                let assembled = transcript
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone();
                if let Err(err) = chat_repo.append(&symbol, "assistant", &assembled).await {
                    eprintln!("Failed to persist chat message (assistant): {err}");
                }
                let _ = app.emit(
                    "ai-chat-done",
                    AiChatDoneEvent {
                        request_id: requestId.clone(),
                    },
                );
            }
            Err(message) => {
                if let Err(err) = chat_repo.append(&symbol, "error", &message).await {
                    eprintln!("Failed to persist chat message (error): {err}");
                }
                let _ = app.emit(
                    "ai-chat-error",
                    AiChatErrorEvent {
                        request_id: requestId.clone(),
                        message,
                    },
                );
            }
        }
    });
    Ok(())
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

/// Returns persisted chat history for `symbol`, oldest first.
#[tauri::command]
async fn ai_chat_history(
    state: tauri::State<'_, NestHostState>,
    symbol: String,
) -> Result<Vec<ChatMessageRow>, String> {
    let repo = state
        .context
        .service::<ChatHistoryRepository>()
        .map_err(|e| e.to_string())?;
    repo.list_for_symbol(&symbol).await.map_err(|e| e.to_string())
}

/// Deletes all persisted chat history for `symbol`.
#[tauri::command]
async fn ai_chat_clear(
    state: tauri::State<'_, NestHostState>,
    symbol: String,
) -> Result<(), String> {
    let repo = state
        .context
        .service::<ChatHistoryRepository>()
        .map_err(|e| e.to_string())?;
    repo.clear_for_symbol(&symbol).await.map_err(|e| e.to_string())
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
        .module(HttpClientModule::default())
        .module(ThemeModule::default())
        .module(ImageModule::with_cache(cache))
        .with_builder(|builder| {
            builder.plugin(
                tauri::plugin::Builder::<tauri::Wry>::new("finch")
                    .invoke_handler(tauri::generate_handler![
                        run_cli,
                        schwab_auth_begin,
                        schwab_auth_complete,
                        schwab_auth_login,
                        schwab_auth_logout,
                        schwab_auth_status,
                        schwab_accounts,
                        schwab_account_summary,
                        schwab_orders,
                        schwab_positions,
                        ask_stock_question,
                        settings_get,
                        settings_set,
                        ai_chat_history,
                        ai_chat_clear,
                    ])
                    .build(),
            )
        })
        .run(tauri::generate_context!());
}
