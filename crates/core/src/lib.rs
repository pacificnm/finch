//! Finch core library.
//!
//! Domain logic and the shared command surface live here. Each host
//! (CLI / TUI / desktop) is a thin adapter that delegates to this crate.

use serde::{Deserialize, Serialize};

pub mod ai;
pub mod data_postgres;
pub mod schwab;
pub mod settings;

/// Commands shared across all product surfaces.
#[derive(Debug, Serialize, Deserialize)]
pub enum CliCommand {
    /// Return a greeting for the given name.
    Greet { name: String },
    /// Return the application version.
    AboutVersion,
    /// List recipes applied to this app.
    ListRecipes,
    /// Search instruments by symbol or description (returns JSON).
    SchwabSearchInstruments { query: String },
    /// Fetch quote data for a symbol (returns JSON).
    SchwabQuoteJson { symbol: String },
    /// Fetch price history candles for a symbol (returns JSON).
    SchwabPriceHistory {
        symbol: String,
        period_type: String,
        period: String,
        frequency_type: String,
        frequency: String,
    },
}

/// Dispatches a shared command and returns a serialisable result.
pub async fn run_command_async(command: CliCommand) -> Result<String, String> {
    match command {
        CliCommand::Greet { name } => Ok(greet(&name)),
        CliCommand::AboutVersion => {
            let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            nest_version::app_version(manifest_dir).map_err(|e| e.to_string())
        }
        CliCommand::ListRecipes => list_recipes(),
        CliCommand::SchwabSearchInstruments { query } => {
            schwab::search_instruments_json(&query).await
        }
        CliCommand::SchwabQuoteJson { symbol } => {
            schwab::quote_json(&symbol).await
        }
        CliCommand::SchwabPriceHistory {
            symbol,
            period_type,
            period,
            frequency_type,
            frequency,
        } => {
            schwab::price_history(
                &symbol,
                Some(&period_type),
                Some(&period),
                Some(&frequency_type),
                Some(&frequency),
                None,
                None,
            )
            .await
        }
    }
}

/// Dispatches a shared command and returns a serialisable result (sync wrapper).
pub fn run_command(command: CliCommand) -> Result<String, String> {
    // For async commands, we need to use tokio runtime
    match command {
        CliCommand::Greet { name } => Ok(greet(&name)),
        CliCommand::AboutVersion => {
            let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            nest_version::app_version(manifest_dir).map_err(|e| e.to_string())
        }
        CliCommand::ListRecipes => list_recipes(),
        CliCommand::SchwabSearchInstruments { query: _ } => {
            // This needs to be called from an async context
            return Err("Use run_command_async for SchwabSearchInstruments".to_string());
        }
        CliCommand::SchwabQuoteJson { symbol: _ } => {
            // This needs to be called from an async context
            return Err("Use run_command_async for SchwabQuoteJson".to_string());
        }
        CliCommand::SchwabPriceHistory { .. } => {
            // This needs to be called from an async context
            return Err("Use run_command_async for SchwabPriceHistory".to_string());
        }
    }
}

/// Reads `.nest-recipes` from the app root and returns a human-readable list
/// of applied recipes and their versions.
pub fn list_recipes() -> Result<String, String> {
    let path = find_app_root()
        .map(|root| root.join(".nest-recipes"))
        .unwrap_or_else(|| std::path::PathBuf::from(".nest-recipes"));

    if !path.exists() {
        return Ok("No recipes applied.".to_string());
    }

    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((id, version)) = line.split_once('@') {
            lines.push(format!("{} (v{})", id, version));
        }
    }

    if lines.is_empty() {
        return Ok("No recipes applied.".to_string());
    }

    Ok(lines.join("\n"))
}

/// Walks up from the current directory looking for a directory that contains
/// `.nest-recipes` or a `Cargo.toml` workspace. This lets surface binaries
/// (cli, desktop, tui) find the app root regardless of which subdirectory they
/// run from.
fn find_app_root() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;

    loop {
        if dir.join(".nest-recipes").exists() {
            return Some(dir);
        }

        if let Ok(contents) = std::fs::read_to_string(dir.join("Cargo.toml")) {
            if contents.contains("[workspace]") {
                return Some(dir);
            }
        }

        if !dir.pop() {
            break;
        }
    }

    None
}

/// Returns a greeting for the given name.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
