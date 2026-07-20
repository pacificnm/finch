#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use finch_core::{run_command, CliCommand};
use nest_cache::Cache;
use nest_cache_file::{FileCacheAdapter, FileCacheConfig};
use nest_image::ImageModule;
use nest_tauri::TauriApp;
use nest_theme::ThemeModule;

// This desktop app is a thin client. The same command surface lives in
// `crates/core` and is reused by the CLI and TUI surfaces.
#[tauri::command]
async fn run_cli(command: CliCommand) -> Result<String, String> {
    run_command(command)
}

fn main() {
    let cache_root = std::env::temp_dir().join("finch-cache");
    let cache = Cache::new(Arc::new(
        FileCacheAdapter::new(FileCacheConfig::new(&cache_root))
            .expect("failed to open image cache directory"),
    ));

    TauriApp::new("finch")
        .module(ThemeModule::default())
        .module(ImageModule::with_cache(cache))
        .with_builder(|builder| {
            builder.plugin(
                tauri::plugin::Builder::<tauri::Wry>::new("finch")
                    .invoke_handler(tauri::generate_handler![run_cli])
                    .build(),
            )
        })
        .run(tauri::generate_context!());
}
