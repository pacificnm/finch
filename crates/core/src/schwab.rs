//! Schwab domain dispatch — shared by every host's `schwab` command tree.
//!
//! Credentials (`SCHWAB_APP_KEY`/`SCHWAB_APP_SECRET`) are read from the
//! environment only, never from config files or arguments — same rule
//! `nest-schwab`'s own `fetch_shapes` example follows. The acquired OAuth
//! token is persisted to `~/.config/finch/schwab-tokens.json` via
//! `nest_auth::FileTokenStore` so a login done once survives across CLI
//! invocations (each `finch-cli` run is a fresh process).

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nest_auth::{FileTokenStore, TokenStore};
use nest_auth_oauth_client::{OAuthClient, OAuthTokenAuth};
use nest_http::HttpResponse;
use nest_schwab::{SchwabClient, SchwabConfig};
use serde::Serialize;
use serde_json::Value;

const TOKEN_KEY: &str = "schwab";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

fn schwab_config() -> Result<SchwabConfig, String> {
    let app_key =
        std::env::var("SCHWAB_APP_KEY").map_err(|_| "SCHWAB_APP_KEY is not set".to_string())?;
    let app_secret = std::env::var("SCHWAB_APP_SECRET")
        .map_err(|_| "SCHWAB_APP_SECRET is not set".to_string())?;
    Ok(SchwabConfig::new(app_key, app_secret))
}

fn token_store() -> Result<FileTokenStore, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    let path: PathBuf = [
        home.as_os_str(),
        ".config".as_ref(),
        "finch".as_ref(),
        "schwab-tokens.json".as_ref(),
    ]
    .iter()
    .collect();
    Ok(FileTokenStore::new(path))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn pretty(value: impl Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(&value).map_err(|err| err.to_string())
}

fn describe_response(response: HttpResponse) -> String {
    let mut out = format!("status: {}", response.status.code());
    if let Some(location) = response.headers.get("location") {
        out.push_str(&format!("\nlocation: {location}"));
    }
    if let Some(text) = response.body_text() {
        if !text.trim().is_empty() {
            out.push_str(&format!("\nbody: {text}"));
        }
    }
    out
}

/// Parses an order body argument: either a literal JSON string, or `@path`
/// to read the JSON from a file (the common curl-style convention, since
/// order payloads are too large to comfortably type inline).
pub fn parse_order_arg(raw: &str) -> Result<Value, String> {
    let text = match raw.strip_prefix('@') {
        Some(path) => std::fs::read_to_string(path).map_err(|err| err.to_string())?,
        None => raw.to_string(),
    };
    serde_json::from_str(&text).map_err(|err| format!("invalid order JSON: {err}"))
}

/// `schwab auth login` — runs the interactive browser OAuth flow and stores
/// the resulting token.
pub async fn auth_login() -> Result<String, String> {
    let config = schwab_config()?;
    let oauth_client =
        OAuthClient::new(&config.to_oauth_client_config()).map_err(|err| err.to_string())?;
    let request = oauth_client.authorization_request();
    println!("Open this URL in a browser and log in:\n{}", request.url);
    println!(
        "(Your browser will warn about the self-signed certificate on the 127.0.0.1 redirect \
         — that's expected, click through it.)"
    );
    println!("Waiting up to 5 minutes for the redirect...");

    let token = oauth_client
        .complete_login(request, LOGIN_TIMEOUT)
        .await
        .map_err(|err| err.to_string())?;
    token_store()?
        .put(TOKEN_KEY, &token)
        .await
        .map_err(|err| err.to_string())?;
    Ok("Login succeeded, token stored.".to_string())
}

/// `schwab auth logout` — removes the stored token.
pub async fn auth_logout() -> Result<String, String> {
    token_store()?
        .delete(TOKEN_KEY)
        .await
        .map_err(|err| err.to_string())?;
    Ok("Stored Schwab token removed.".to_string())
}

/// `schwab auth status` — reports whether a token is stored and, if so,
/// whether it's expired and whether it can be silently refreshed.
pub async fn auth_status() -> Result<String, String> {
    let token = token_store()?
        .get(TOKEN_KEY)
        .await
        .map_err(|err| err.to_string())?;
    match token {
        None => Ok("Not logged in. Run `schwab auth login`.".to_string()),
        Some(token) => Ok(format!(
            "Logged in.\nexpired: {}\nhas_refresh_token: {}",
            token.is_expired(now_ms()),
            token.refresh_token.is_some()
        )),
    }
}

async fn client() -> Result<SchwabClient, String> {
    let config = schwab_config()?;
    let token = token_store()?
        .get(TOKEN_KEY)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "not logged in — run `schwab auth login` first".to_string())?;
    let auth = OAuthTokenAuth::new(token);
    SchwabClient::new(&config, auth).map_err(|err| err.to_string())
}

/// `schwab account-numbers` — GET /accounts/accountNumbers.
pub async fn account_numbers() -> Result<String, String> {
    pretty(
        client()
            .await?
            .account_numbers()
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab accounts` — GET /accounts.
pub async fn accounts() -> Result<String, String> {
    pretty(
        client()
            .await?
            .accounts()
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab account <account_hash>` — GET /accounts/{account_hash}.
pub async fn account(account_hash: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .account(account_hash)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab orders <account_hash>` — GET /accounts/{account_hash}/orders.
pub async fn orders_for_account(account_hash: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .orders_for_account(account_hash)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab order <account_hash> <order_id>` — GET .../orders/{order_id}.
pub async fn order(account_hash: &str, order_id: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .order(account_hash, order_id)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab place-order <account_hash> <order>` — POST .../orders.
pub async fn place_order(account_hash: &str, order: Value) -> Result<String, String> {
    let response = client()
        .await?
        .place_order(account_hash, &order)
        .await
        .map_err(|e| e.to_string())?;
    Ok(describe_response(response))
}

/// `schwab replace-order <account_hash> <order_id> <order>` — PUT .../orders/{order_id}.
pub async fn replace_order(
    account_hash: &str,
    order_id: &str,
    order: Value,
) -> Result<String, String> {
    let response = client()
        .await?
        .replace_order(account_hash, order_id, &order)
        .await
        .map_err(|e| e.to_string())?;
    Ok(describe_response(response))
}

/// `schwab cancel-order <account_hash> <order_id>` — DELETE .../orders/{order_id}.
pub async fn cancel_order(account_hash: &str, order_id: &str) -> Result<String, String> {
    let response = client()
        .await?
        .cancel_order(account_hash, order_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(describe_response(response))
}

/// `schwab preview-order <account_hash> <order>` — POST .../previewOrder.
pub async fn preview_order(account_hash: &str, order: Value) -> Result<String, String> {
    pretty(
        client()
            .await?
            .preview_order(account_hash, &order)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab transactions <account_hash>` — GET .../transactions.
pub async fn transactions(account_hash: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .transactions(account_hash)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab transaction <account_hash> <transaction_id>` — GET .../transactions/{transaction_id}.
pub async fn transaction(account_hash: &str, transaction_id: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .transaction(account_hash, transaction_id)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab user-preference` — GET /userPreference.
pub async fn user_preference() -> Result<String, String> {
    pretty(
        client()
            .await?
            .user_preference()
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab quotes <symbols...>` — GET /quotes?symbols=...
pub async fn quotes(symbols: &[String]) -> Result<String, String> {
    let refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
    pretty(
        client()
            .await?
            .quotes(&refs)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab quote <symbol>` — GET /{symbol}/quotes.
pub async fn quote(symbol: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .quote(symbol)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab chains <symbol>` — GET /chains?symbol=...
pub async fn option_chain(symbol: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .option_chain(symbol)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab expiration-chain <symbol>` — GET /expirationchain?symbol=...
pub async fn expiration_chain(symbol: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .expiration_chain(symbol)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab price-history <symbol> [options]` — GET /pricehistory.
#[allow(clippy::too_many_arguments)]
pub async fn price_history(
    symbol: &str,
    period_type: Option<&str>,
    period: Option<&str>,
    frequency_type: Option<&str>,
    frequency: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<String, String> {
    let mut query: Vec<(&str, &str)> = vec![("symbol", symbol)];
    if let Some(v) = period_type {
        query.push(("periodType", v));
    }
    if let Some(v) = period {
        query.push(("period", v));
    }
    if let Some(v) = frequency_type {
        query.push(("frequencyType", v));
    }
    if let Some(v) = frequency {
        query.push(("frequency", v));
    }
    if let Some(v) = start_date {
        query.push(("startDate", v));
    }
    if let Some(v) = end_date {
        query.push(("endDate", v));
    }
    pretty(
        client()
            .await?
            .price_history(&query)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab movers <symbol_id> [--sort] [--frequency]` — GET /movers/{symbol_id}.
pub async fn movers(
    symbol_id: &str,
    sort: Option<&str>,
    frequency: Option<&str>,
) -> Result<String, String> {
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(v) = sort {
        query.push(("sort", v));
    }
    if let Some(v) = frequency {
        query.push(("frequency", v));
    }
    pretty(
        client()
            .await?
            .movers(symbol_id, &query)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab market-hours [--markets] [--date]` — GET /markets.
pub async fn market_hours(markets: Option<&str>, date: Option<&str>) -> Result<String, String> {
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(v) = markets {
        query.push(("markets", v));
    }
    if let Some(v) = date {
        query.push(("date", v));
    }
    pretty(
        client()
            .await?
            .market_hours(&query)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab instruments <symbol> [--projection]` — GET /instruments.
pub async fn instruments(symbol: &str, projection: Option<&str>) -> Result<String, String> {
    let mut query: Vec<(&str, &str)> = vec![("symbol", symbol)];
    if let Some(v) = projection {
        query.push(("projection", v));
    }
    pretty(
        client()
            .await?
            .instruments(&query)
            .await
            .map_err(|e| e.to_string())?,
    )
}

/// `schwab instrument <cusip_id>` — GET /instruments/{cusip_id}.
pub async fn instrument_by_cusip(cusip_id: &str) -> Result<String, String> {
    pretty(
        client()
            .await?
            .instrument_by_cusip(cusip_id)
            .await
            .map_err(|e| e.to_string())?,
    )
}
