//! Schwab domain dispatch — shared by every host's `schwab` command tree.
//!
//! Credentials (`SCHWAB_APP_KEY`/`SCHWAB_APP_SECRET`) are read from the
//! environment only, never from config files or arguments — same rule
//! `nest-schwab`'s own `fetch_shapes` example follows. The acquired OAuth
//! token is persisted to `~/.config/finch/schwab-tokens.json` via
//! `nest_auth::FileTokenStore` so a login done once survives across CLI
//! invocations (each `finch-cli` run is a fresh process).

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nest_auth::{FileTokenStore, TokenStore};
use nest_auth_oauth_client::{OAuthClient, OAuthTokenAuth};
use nest_http::HttpResponse;
use nest_schwab::{SchwabClient, SchwabConfig};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const TOKEN_KEY: &str = "schwab";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

/// Pending OAuth state (CSRF token + PKCE verifier) stored between
/// generating the authorization URL and completing the login with the code.
/// This is a simple in-memory store for single-user desktop use.
static PENDING_LOGIN: Lazy<Mutex<Option<PendingLogin>>> = Lazy::new(|| Mutex::new(None));

struct PendingLogin {
    state: String,
    verifier: String,
}

fn set_pending_login(state: String, verifier: String) {
    if let Ok(mut guard) = PENDING_LOGIN.lock() {
        *guard = Some(PendingLogin { state, verifier });
    }
}

fn take_pending_login() -> Option<PendingLogin> {
    PENDING_LOGIN.lock().ok()?.take()
}

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
    auth_login_inner(false).await
}

/// `schwab auth login --manual` — prints the authorization URL and prompts
/// for the authorization code instead of running the local HTTPS loopback
/// listener. Useful when the browser rejects the self-signed certificate
/// used by the loopback callback.
pub async fn auth_login_manual() -> Result<String, String> {
    auth_login_inner(true).await
}

async fn auth_login_inner(manual: bool) -> Result<String, String> {
    let config = schwab_config()?;
    let oauth_client =
        OAuthClient::new(&config.to_oauth_client_config()).map_err(|err| err.to_string())?;
    let request = oauth_client.authorization_request();
    println!("Open this URL in a browser and log in:\n{}", request.url);

    let token = if manual {
        println!("\nAfter logging in, your browser will redirect to a URL like:");
        println!("  https://127.0.0.1:{}/callback?code=<CODE>&state=<STATE>", config.redirect_port);
        println!("Paste the authorization code (<CODE>) below and press Enter.");
        let code = read_line("Authorization code: ").await?;
        // The browser address bar may show the code URL-decoded or encoded.
        // Decode it so we always send the original code Schwab issued.
        let code = urlencoding::decode(code.trim()).map_err(|err| err.to_string())?;
        oauth_client
            .exchange_code_from_request(request, code.as_ref())
            .await
            .map_err(|err| err.to_string())?
    } else {
        println!(
            "(Your browser will warn about the self-signed certificate on the 127.0.0.1 redirect \
             — that's expected, click through it.)"
        );
        println!("Waiting up to 5 minutes for the redirect...");
        oauth_client
            .complete_login(request, LOGIN_TIMEOUT)
            .await
            .map_err(|err| err.to_string())?
    };
    token_store()?
        .put(TOKEN_KEY, &token)
        .await
        .map_err(|err| err.to_string())?;
    Ok("Login succeeded, token stored.".to_string())
}

async fn read_line(prompt: &str) -> Result<String, String> {
    use std::io::Write;
    print!("{prompt}");
    std::io::stdout().flush().map_err(|err| err.to_string())?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|err| err.to_string())?;
    Ok(line)
}

/// Begins a UI-driven OAuth login: generates an authorization URL and stores
/// the PKCE verifier in memory. The caller must pass the resulting code to
/// [`auth_complete`] to finish the flow.
pub async fn auth_begin() -> Result<String, String> {
    let config = schwab_config()?;
    let oauth_client =
        OAuthClient::new(&config.to_oauth_client_config()).map_err(|err| err.to_string())?;
    let request = oauth_client.authorization_request();
    let (url, state, verifier) = request.into_parts();
    set_pending_login(state, verifier.secret().to_string());
    Ok(url.to_string())
}

/// The redirect URI Schwab will send the browser back to once login
/// completes (e.g. `https://127.0.0.1:8443`).
pub fn redirect_uri_prefix() -> Result<String, String> {
    Ok(schwab_config()?.redirect_uri())
}

/// UI-driven equivalent of [`auth_login`]/`auth_login_inner`'s non-manual
/// path: generates the authorization URL, hands it to `on_url` (so the
/// caller can open it in the system browser — Schwab rejects logins
/// attempted inside an embedded webview), then waits on the local HTTPS
/// loopback listener for the redirect and completes the token exchange
/// automatically. No code copy/paste required; the only user-visible
/// friction is the browser's one-time self-signed certificate warning on
/// the `127.0.0.1` redirect.
pub async fn auth_login_loopback<F>(on_url: F) -> Result<String, String>
where
    F: FnOnce(String) + Send,
{
    let config = schwab_config()?;
    let oauth_client =
        OAuthClient::new(&config.to_oauth_client_config()).map_err(|err| err.to_string())?;
    let request = oauth_client.authorization_request();
    on_url(request.url.to_string());
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

/// Completes a UI-driven OAuth login: exchanges the authorization code for a
/// token using the verifier stored by [`auth_begin`].
pub async fn auth_complete(code: &str, state: &str) -> Result<String, String> {
    let pending = take_pending_login()
        .ok_or_else(|| "no pending login — generate an authorization URL first".to_string())?;

    if pending.state != state {
        return Err("OAuth state mismatch — possible CSRF attack".to_string());
    }

    let config = schwab_config()?;
    let oauth_client =
        OAuthClient::new(&config.to_oauth_client_config()).map_err(|err| err.to_string())?;
    let token = oauth_client
        .exchange_code(
            urlencoding::decode(code).map_err(|err| err.to_string())?,
            oauth2::PkceCodeVerifier::new(pending.verifier),
        )
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

/// `schwab auth status` — reports whether a usable token is stored.
/// Expired tokens are reported as not-logged-in so the UI prompts for login.
pub async fn auth_status() -> Result<String, String> {
    let token = token_store()?
        .get(TOKEN_KEY)
        .await
        .map_err(|err| err.to_string())?;
    match token {
        None => Ok("Not logged in. Run `schwab auth login`.".to_string()),
        Some(token) => {
            if token.is_expired(now_ms()) {
                Ok("Token expired. Run `schwab auth login`.".to_string())
            } else {
                Ok(format!(
                    "Logged in.\nexpired: false\nhas_refresh_token: {}",
                    token.refresh_token.is_some()
                ))
            }
        }
    }
}

async fn oauth_client() -> Result<OAuthClient, String> {
    let config = schwab_config()?;
    OAuthClient::new(&config.to_oauth_client_config()).map_err(|err| err.to_string())
}

async fn client() -> Result<SchwabClient, String> {
    let config = schwab_config()?;
    let store = token_store()?;
    let mut token = store
        .get(TOKEN_KEY)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "not logged in — run `schwab auth login` first".to_string())?;

    // Refresh the access token if it has expired and a refresh token is available.
    if token.is_expired(now_ms()) {
        if let Some(refresh_token) = token.refresh_token.clone() {
            match oauth_client().await?.refresh(&refresh_token).await {
                Ok(new_token) => {
                    if let Err(err) = store.put(TOKEN_KEY, &new_token).await {
                        eprintln!("[schwab] failed to persist refreshed token: {err}");
                    }
                    token = new_token;
                }
                Err(err) => {
                    return Err(format!(
                        "Schwab token expired and refresh failed: {err}. Run `schwab auth login`."
                    ));
                }
            }
        } else {
            return Err("Schwab token expired and no refresh token is available. Run `schwab auth login`.".to_string());
        }
    }

    let auth = OAuthTokenAuth::new(token);
    SchwabClient::new(&config, auth).map_err(|err| err.to_string())
}

/// Merged Schwab account information for UI selectors and account-scoped calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchwabAccount {
    /// Display account number (last few digits masked by Schwab).
    pub account_number: String,
    /// Account hash used for all account-scoped API calls.
    pub hash: String,
    /// User-defined nickname, if available from `/userPreference`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// Display account id (e.g. "...688"), if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_account_id: Option<String>,
    /// Account type, e.g. "BROKERAGE".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_type: Option<String>,
    /// Whether this is the user's primary account.
    #[serde(default)]
    pub primary_account: bool,
    /// User-defined color, e.g. "Green".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_color: Option<String>,
}

fn preference_for_account<'a>(
    account_number: &str,
    preferences: &'a Value,
) -> Option<&'a Value> {
    preferences
        .get("accounts")
        .and_then(Value::as_array)
        .and_then(|accounts| {
            accounts
                .iter()
                .find(|pref| pref.get("accountNumber").and_then(Value::as_str) == Some(account_number))
        })
}

/// Lists linked Schwab accounts, merging `/accounts/accountNumbers` (hash mapping)
/// with `/userPreference` (nicknames, display ids, etc.).
pub async fn list_accounts() -> Result<Vec<SchwabAccount>, String> {
    let client = client().await?;
    eprintln!("[list_accounts] fetching account numbers");
    let numbers = client
        .account_numbers()
        .await
        .map_err(|e| {
            eprintln!("[list_accounts] account_numbers failed: {e}");
            e.to_string()
        })?;
    eprintln!("[list_accounts] account_numbers response: {numbers}");
    let preferences = client
        .user_preference()
        .await
        .unwrap_or_else(|_| Value::Null);

    let items = numbers.as_array().ok_or("unexpected accountNumbers response")?;
    let mut accounts = Vec::with_capacity(items.len());
    for item in items {
        let account_number = item
            .get("accountNumber")
            .and_then(Value::as_str)
            .ok_or("missing accountNumber in response")?
            .to_string();
        let hash = item
            .get("hashValue")
            .and_then(Value::as_str)
            .ok_or("missing hashValue in response")?
            .to_string();

        let pref = preference_for_account(&account_number, &preferences);
        accounts.push(SchwabAccount {
            account_number: account_number.clone(),
            hash,
            nickname: pref
                .and_then(|p| p.get("nickName"))
                .and_then(Value::as_str)
                .map(String::from),
            display_account_id: pref
                .and_then(|p| p.get("displayAcctId"))
                .and_then(Value::as_str)
                .map(String::from),
            account_type: pref
                .and_then(|p| p.get("type"))
                .and_then(Value::as_str)
                .map(String::from),
            primary_account: pref
                .and_then(|p| p.get("primaryAccount"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            account_color: pref
                .and_then(|p| p.get("accountColor"))
                .and_then(Value::as_str)
                .map(String::from),
        });
    }
    Ok(accounts)
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

/// Display-ready account summary values for the Account panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSummary {
    /// Total account value (liquidation value).
    pub account_value: String,
    /// Buying power.
    pub buying_power: String,
    /// Cash available for withdrawal.
    pub cash_for_withdrawal: String,
    /// Day P/L as a percentage string.
    pub pl_day_percent: String,
}

fn format_money(value: f64) -> String {
    let formatted = format!("{:.2}", value);
    let (sign, rest) = if formatted.starts_with('-') {
        ("-$", &formatted[1..])
    } else {
        ("$", formatted.as_str())
    };
    let parts: Vec<&str> = rest.split('.').collect();
    let whole = parts[0];
    let mut with_commas = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            with_commas.push(',');
        }
        with_commas.push(c);
    }
    format!("{}{}.{}", sign, with_commas, parts[1])
}

fn format_percent(value: f64) -> String {
    format!("{:.2}%", value)
}

fn current_balances(value: &Value) -> Option<&Value> {
    value
        .get("securitiesAccount")
        .or_else(|| value.get("account"))
        .and_then(|v| v.get("currentBalances"))
}

fn balance_f64(balance: &Value, key: &str) -> f64 {
    current_balances(balance)
        .and_then(|b| b.get(key))
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(0.0)
}

/// Fetches and formats `/accounts/{account_hash}` balance fields.
pub async fn account_summary(account_hash: &str) -> Result<AccountSummary, String> {
    let value = client()
        .await?
        .account(account_hash)
        .await
        .map_err(|e| e.to_string())?;
    eprintln!("[account_summary] raw response for {account_hash}: {value}");

    // Account value: prefer currentBalances liquidationValue, fallback to
    // aggregatedBalance currentLiquidationValue.
    let account_value = balance_f64(&value, "liquidationValue")
        .max(value.get("aggregatedBalance").and_then(|b| b.get("currentLiquidationValue")).and_then(|v| v.as_f64()).unwrap_or(0.0));

    // Buying power: margin accounts expose `buyingPower`; CASH accounts do
    // not, so fall back to cash available for trading.
    let buying_power = {
        let explicit = balance_f64(&value, "buyingPower");
        if explicit != 0.0 {
            explicit
        } else {
            balance_f64(&value, "cashAvailableForTrading")
        }
    };

    let cash_for_withdrawal = balance_f64(&value, "cashAvailableForWithdrawal");
    eprintln!("[account_summary] parsed: account_value={account_value}, buying_power={buying_power}, cash_for_withdrawal={cash_for_withdrawal}");

    // Schwab's /accounts/{hash} endpoint does not expose a day P/L percent
    // directly. Calculating it requires position-level day P/L or a prior-day
    // account value. Default to 0% until we add position-based calculation.
    let pl_day_percent = 0.0;

    Ok(AccountSummary {
        account_value: format_money(account_value),
        buying_power: format_money(buying_power),
        cash_for_withdrawal: format_money(cash_for_withdrawal),
        pl_day_percent: format_percent(pl_day_percent),
    })
}

/// Display-ready order row for the Positions Activity tabs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRow {
    /// Order id.
    pub order_id: String,
    /// Entry time (ISO 8601).
    pub time: String,
    /// Side, e.g. "BUY" or "SELL".
    pub side: String,
    /// Position effect, e.g. "OPENING" or "CLOSING".
    pub pos_effect: String,
    /// Quantity.
    pub qty: String,
    /// Notional amount (qty * price).
    pub amount: String,
    /// Symbol.
    pub symbol: String,
    /// Order type, e.g. "MARKET" or "LIMIT".
    pub desc: String,
    /// Price.
    pub price: String,
    /// Time in force, e.g. "DAY".
    pub tif: String,
    /// Mark (best-effort; empty if unavailable).
    pub mark: String,
    /// Net price (best-effort; empty if unavailable).
    pub net_prc: String,
    /// Order status, e.g. "WORKING", "FILLED", "CANCELED".
    pub status: String,
}

fn format_qty(value: f64) -> String {
    if value == value.trunc() {
        format!("{:.0}", value)
    } else {
        format!("{:.2}", value)
    }
}

fn parse_orders(value: Value) -> Vec<OrderRow> {
    let items = value.as_array().cloned().unwrap_or_default();
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        rows.extend(parse_order_item(&item, false));
    }
    rows
}

/// Parses a single order or flattens a parent strategy (OCO, TRIGGER, etc.)
/// into its child order rows. `is_oco_child` is set when this item is a child
/// of an OCO parent so the `[OCO]` annotation can be shown.
fn parse_order_item(item: &Value, is_oco_child: bool) -> Vec<OrderRow> {
    // Schwab returns parent strategies with childOrderStrategies containing
    // the actual executable legs. Flatten those so the Activity table shows
    // each working order separately.
    let strategy_type = item
        .get("orderStrategyType")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_uppercase();
    if let Some(children) = item
        .get("childOrderStrategies")
        .and_then(Value::as_array)
        .filter(|c| !c.is_empty())
    {
        let child_is_oco = strategy_type == "OCO";
        let mut child_rows = Vec::new();
        for child in children {
            child_rows.extend(parse_order_item(child, child_is_oco));
        }
        return child_rows;
    }

    let leg = item
        .get("orderLegCollection")
        .and_then(Value::as_array)
        .and_then(|legs| legs.first());
    let instrument = leg.and_then(|l| l.get("instrument"));
    let symbol = instrument
        .and_then(|i| i.get("symbol"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let side = leg
        .and_then(|l| l.get("instruction"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let pos_effect_raw = leg
        .and_then(|l| l.get("positionEffect"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let pos_effect = match pos_effect_raw.as_str() {
        "OPENING" => "TO OPEN".to_string(),
        "CLOSING" => "TO CLOSE".to_string(),
        _ => pos_effect_raw,
    };

    // Use the leg quantity when the top-level order quantity is unset (common
    // for child orders inside OCO strategies).
    let top_level_qty = item
        .get("quantity")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(0.0);
    let leg_qty = leg
        .and_then(|l| l.get("quantity"))
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(0.0);
    let qty = if top_level_qty != 0.0 {
        top_level_qty
    } else {
        leg_qty
    };
    // thinkorswim displays sell quantities as negative.
    let qty = if side.to_uppercase() == "SELL" { -qty.abs() } else { qty };
    let filled_qty = item
        .get("filledQuantity")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(0.0);

    let order_type = item
        .get("orderType")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_uppercase();
    let price = item
        .get("price")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(0.0);
    let stop_price = item
        .get("stopPrice")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(0.0);

    // Effective price for notional amount: stop orders expose stopPrice, not price.
    let effective_price = if price != 0.0 {
        price
    } else {
        stop_price
    };

    // Display string for the Price column, matching thinkorswim:
    // "35.00 LIMIT", "31.96 STOP", "MARKET", etc.
    let price_display = if order_type == "MARKET" {
        "MARKET".to_string()
    } else if effective_price == 0.0 {
        order_type.clone()
    } else {
        format!("{} {}", format_money(effective_price).replace('$', ""), order_type)
    };

    let status = item
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("WORKING")
        .to_string();
    let time = item
        .get("enteredTime")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    // Desc column: show the strategy annotation for OCO children.
    let desc = if is_oco_child || has_oco_parent(item) {
        "[OCO]".to_string()
    } else {
        String::new()
    };

    let tif = item
        .get("duration")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let order_id = item
        .get("orderId")
        .map(|v| {
            v.as_i64()
                .map(|i| i.to_string())
                .or_else(|| v.as_f64().map(|f| f.to_string()))
                .unwrap_or_else(|| v.to_string())
        })
        .unwrap_or_default();

    let display_qty = if status.to_uppercase() == "FILLED" {
        filled_qty
    } else {
        qty
    };
    let amount = display_qty.abs() * effective_price;

    vec![OrderRow {
        order_id,
        time: time.replace('T', " ").trim_end_matches("+0000").to_string(),
        side,
        pos_effect,
        qty: format_qty(display_qty),
        amount: format_money(amount),
        symbol,
        desc,
        price: price_display,
        tif,
        mark: String::new(),
        net_prc: String::new(),
        status: status.clone(),
    }]
}

/// Best-effort detection of an OCO child whose parent was flattened above.
fn has_oco_parent(item: &Value) -> bool {
    item.get("statusDescription")
        .and_then(Value::as_str)
        .map(|s| s.to_uppercase().contains("OCO"))
        .unwrap_or(false)
}

/// Fetches and formats `/accounts/{account_hash}/orders` for the Activity tabs.
/// Defaults to the last 30 days.
pub async fn list_orders(account_hash: &str) -> Result<Vec<OrderRow>, String> {
    let now = chrono::Utc::now();
    let from = now - chrono::Duration::days(30);
    let from_str = from.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let to_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let query = [
        ("fromEnteredTime", from_str.as_str()),
        ("toEnteredTime", to_str.as_str()),
    ];
    let value = client()
        .await?
        .orders_for_account_query(account_hash, &query)
        .await
        .map_err(|e| e.to_string())?;
    Ok(parse_orders(value))
}

/// Display-ready position row for the Positions table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionRow {
    /// Symbol or "Cash" for cash entries.
    pub position: String,
    /// Quantity.
    pub qty: String,
    /// Day P/L dollar amount.
    pub pl_day: String,
    /// Open P/L dollar amount.
    pub pl_open: String,
    /// YTD P/L dollar amount (best-effort; empty if unavailable).
    pub pl_ytd: String,
    /// Cost basis.
    pub cost: String,
    /// Net liquidation value / market value.
    pub net_liq: String,
    /// Average trade price.
    pub trade_price: String,
    /// Buying power effect.
    pub bp_effect: String,
    /// Delta (empty if unavailable).
    pub delta: String,
    /// Gamma (empty if unavailable).
    pub gamma: String,
    /// Theta (empty if unavailable).
    pub theta: String,
    /// Vega (empty if unavailable).
    pub vega: String,
}

fn parse_positions(value: &Value) -> Vec<PositionRow> {
    let items = value
        .get("securitiesAccount")
        .and_then(|v| v.get("positions"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut rows = Vec::with_capacity(items.len().saturating_add(2));

    // Running sums for the synthetic Totals row appended below. Only
    // fields with real per-row numeric data get summed (pl_day, pl_open,
    // cost, net_liq, bp_effect) — qty/pl_ytd/greeks stay "—" in the total,
    // same as thinkorswim, since summing quantities across different
    // symbols isn't meaningful and pl_ytd/greeks aren't populated per-row
    // yet (nothing to honestly sum).
    let mut total_pl_day = 0.0;
    let mut total_open_pl = 0.0;
    let mut total_cost = 0.0;
    let mut total_net_liq = 0.0;
    let mut total_bp_effect = 0.0;

    // Add a synthetic Cash row from the account's cash balance.
    if let Some(cash) = current_balances(value)
        .and_then(|b| b.get("cashBalance"))
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
    {
        if cash != 0.0 {
            total_cost += cash;
            total_net_liq += cash;
            rows.push(PositionRow {
                position: "Cash".to_string(),
                qty: "—".to_string(),
                pl_day: "—".to_string(),
                pl_open: "—".to_string(),
                pl_ytd: "—".to_string(),
                cost: format_money(cash),
                net_liq: format_money(cash),
                trade_price: "—".to_string(),
                bp_effect: "—".to_string(),
                delta: "—".to_string(),
                gamma: "—".to_string(),
                theta: "—".to_string(),
                vega: "—".to_string(),
            });
        }
    }

    for item in items {
        let instrument = item.get("instrument");
        let symbol = instrument
            .and_then(|i| i.get("symbol"))
            .and_then(Value::as_str)
            .unwrap_or("Unknown")
            .to_string();

        let long_qty = item
            .get("longQuantity")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);
        let short_qty = item
            .get("shortQuantity")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);
        let qty = if short_qty > 0.0 { -short_qty } else { long_qty };

        let avg_price = item
            .get("averagePrice")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);
        let market_value = item
            .get("marketValue")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);
        let day_pl = item
            .get("currentDayProfitLoss")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);
        let open_pl = item
            .get("longOpenProfitLoss")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);
        let amount_paid = item
            .get("amountPaid")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);
        let cost = if amount_paid != 0.0 {
            amount_paid
        } else {
            avg_price * qty.abs()
        };
        let bp_effect = item
            .get("maintenanceRequirement")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);

        total_pl_day += day_pl;
        total_open_pl += open_pl;
        total_cost += cost;
        total_net_liq += market_value;
        total_bp_effect += bp_effect;

        rows.push(PositionRow {
            position: symbol,
            qty: format_qty(qty),
            pl_day: format_money(day_pl),
            pl_open: format_money(open_pl),
            pl_ytd: String::new(),
            cost: format_money(cost),
            net_liq: format_money(market_value),
            trade_price: format_money(avg_price),
            bp_effect: format_money(bp_effect),
            delta: String::new(),
            gamma: String::new(),
            theta: String::new(),
            vega: String::new(),
        });
    }

    if !rows.is_empty() {
        rows.push(PositionRow {
            position: "Totals:".to_string(),
            qty: "—".to_string(),
            pl_day: format_money(total_pl_day),
            pl_open: format_money(total_open_pl),
            pl_ytd: "—".to_string(),
            cost: format_money(total_cost),
            net_liq: format_money(total_net_liq),
            trade_price: "—".to_string(),
            bp_effect: format_money(total_bp_effect),
            delta: "—".to_string(),
            gamma: "—".to_string(),
            theta: "—".to_string(),
            vega: "—".to_string(),
        });
    }

    rows
}

/// Fetches and formats `/accounts/{account_hash}?fields=positions` for the
/// Positions table.
pub async fn list_positions(account_hash: &str) -> Result<Vec<PositionRow>, String> {
    let value = client()
        .await?
        .account_with_fields(account_hash, &["positions"])
        .await
        .map_err(|e| e.to_string())?;
    eprintln!("[positions] raw response for {account_hash}: {value}");
    let rows = parse_positions(&value);
    eprintln!("[positions] parsed {} rows", rows.len());
    Ok(rows)
}

/// `schwab orders <account_hash>` — GET /accounts/{account_hash}/orders (raw JSON).
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

/// Fetches quote data in JSON format for UI consumption.
pub async fn quote_json(symbol: &str) -> Result<String, String> {
    let result = client()
        .await?
        .quote(symbol)
        .await
        .map_err(|e| e.to_string())?;

    // Parse Schwab response - structure is {symbol: {quote: {...}, reference: {...}, fundamental: {...}}}
    let mut quote_data = if let Some(obj) = result.as_object() {
        // Get the inner object (could be directly the symbol key or nested)
        let inner = obj.get(symbol)
            .or_else(|| obj.get("symbols").and_then(|s| s.as_object().and_then(|syms| syms.get(symbol))))
            .unwrap_or(&result);

        if let Some(q) = inner.as_object() {
            // Extract from 'quote' section
            let quote_section = q.get("quote").and_then(|v| v.as_object());
            let ref_section = q.get("reference").and_then(|v| v.as_object());
            let fund_section = q.get("fundamental").and_then(|v| v.as_object());
            
            let description = ref_section
                .and_then(|r| r.get("description").and_then(|v| v.as_str()))
                .unwrap_or("");
            
            let last_price = quote_section
                .and_then(|q| q.get("lastPrice").and_then(|v| v.as_f64()));
            
            let open_price = quote_section
                .and_then(|q| q.get("openPrice").and_then(|v| v.as_f64()));
            
            let high_price = quote_section
                .and_then(|q| q.get("highPrice").and_then(|v| v.as_f64()));
            
            let low_price = quote_section
                .and_then(|q| q.get("lowPrice").and_then(|v| v.as_f64()));
            
            let close_price = quote_section
                .and_then(|q| q.get("closePrice").and_then(|v| v.as_f64()));
            
            let net_change = quote_section
                .and_then(|q| q.get("netChange").and_then(|v| v.as_f64()));
            
            let percent_change = quote_section
                .and_then(|q| q.get("netPercentChange").and_then(|v| v.as_f64()));
            
            let bid_size = quote_section
                .and_then(|q| q.get("bidSize").and_then(|v| v.as_u64()))
                .map(|n| n.to_string());
            
            let ask_size = quote_section
                .and_then(|q| q.get("askSize").and_then(|v| v.as_u64()))
                .map(|n| n.to_string());
            
            let volume = quote_section
                .and_then(|q| q.get("totalVolume").and_then(|v| v.as_u64()));
            
            // Calculate VWAP (simplified: average of high, low, close)
            let vwap = match (high_price, low_price, close_price) {
                (Some(h), Some(l), Some(c)) => Some((h + l + c) / 3.0),
                _ => None,
            };

            // Helper: try multiple possible field names for the same concept.
            let f64_from = |section: Option<&serde_json::Map<String, serde_json::Value>>, names: &[&str]| {
                for name in names {
                    if let Some(v) = section.and_then(|s| s.get(*name)).and_then(|v| v.as_f64()) {
                        return Some(v);
                    }
                }
                None
            };

            let string_from = |section: Option<&serde_json::Map<String, serde_json::Value>>, names: &[&str]| {
                for name in names {
                    if let Some(v) = section.and_then(|s| s.get(*name)).and_then(|v| v.as_str()) {
                        return Some(v.to_string());
                    }
                }
                None
            };

            let market_cap = f64_from(fund_section, &["marketCap", "market_cap", "marketCapitalization"])
                .or_else(|| {
                    // Calculate market cap if not provided: shares * price
                    match (f64_from(fund_section, &["sharesOutstanding", "shares_outstanding"]), last_price) {
                        (Some(shares), Some(price)) => Some(shares * price),
                        _ => None,
                    }
                });

            let pe_ratio = f64_from(fund_section, &["peRatio", "pe_ratio", "pe"]);

            let dividend_yield = f64_from(fund_section, &["divYield", "dividendYield", "yield"]);

            let eps = f64_from(fund_section, &["eps", "epsTTM"]);

            let div_amount = f64_from(fund_section, &["divAmount", "dividendAmount"]);

            let avg_10day_volume = f64_from(fund_section, &["avg10DaysVolume", "avg10DayVolume", "average10DayVolume"])
                .map(|n| n as u64);

            let avg_1year_volume = f64_from(fund_section, &["avg1YearVolume", "avg1YrVolume", "average1YearVolume"])
                .map(|n| n as u64);

            let shares_outstanding = f64_from(fund_section, &["sharesOutstanding", "shares_outstanding"]);

            // Optional fields that may be present in the Schwab response depending on the symbol/asset type.
            let beta = f64_from(fund_section, &["beta", "betaCoefficient"]);

            let iv = f64_from(quote_section, &["volatility", "impliedVolatility", "iv"])
                .or_else(|| f64_from(fund_section, &["volatility", "impliedVolatility", "iv"]));

            let hv = f64_from(fund_section, &["historicalVolatility", "histVolatility", "hv"])
                .or_else(|| f64_from(quote_section, &["historicalVolatility", "histVolatility", "hv"]));

            let mmm = f64_from(quote_section, &["mmmv", "marketMakerMove", "mmm"])
                .or_else(|| f64_from(fund_section, &["mmmv", "marketMakerMove", "mmm"]));

            let ex_date = string_from(fund_section, &["exDate", "exDividendDate", "dividendDate"])
                .or_else(|| string_from(ref_section, &["exDate", "exDividendDate", "dividendDate"]))
                .map(format_schwab_date);

            let earnings_date = string_from(ref_section, &["earningsDate", "nextEarningsDate", "earningsAnnounceDate", "lastEarningsDate"])
                .or_else(|| string_from(fund_section, &["earningsDate", "nextEarningsDate", "earningsAnnounceDate", "lastEarningsDate"]))
                .map(format_schwab_date);

            serde_json::json!({
                "symbol": symbol,
                "description": description,
                "lastPrice": last_price,
                "openPrice": open_price,
                "highPrice": high_price,
                "lowPrice": low_price,
                "closePrice": close_price,
                "netChange": net_change,
                "percentChange": percent_change,
                "bidSize": bid_size,
                "askSize": ask_size,
                "volume": volume,
                "marketCap": market_cap,
                "peRatio": pe_ratio,
                "dividendYield": dividend_yield,
                "beta": beta,
                "eps": eps,
                "divAmount": div_amount,
                "avg10DayVolume": avg_10day_volume,
                "avg1YearVolume": avg_1year_volume,
                "sharesOutstanding": shares_outstanding,
                "vwap": vwap,
                "iv": iv,
                "hv": hv,
                "mmm": mmm,
                "exDate": ex_date,
                "earningsDate": earnings_date,
            })
        } else {
            serde_json::json!({ "symbol": symbol, "error": "Invalid quote format" })
        }
    } else {
        serde_json::json!({ "symbol": symbol, "error": "Invalid quote response" })
    };

    // If fields are still missing, try to calculate or fetch them from other endpoints.
    if let Some(obj) = quote_data.as_object_mut() {
        let last_price = obj.get("lastPrice").and_then(|v| v.as_f64());

        // Market cap fallback: shares outstanding * last price.
        if obj.get("marketCap").is_none() || obj.get("marketCap").and_then(|v| v.as_f64()).is_none() {
            if let (Some(shares), Some(price)) = (
                obj.get("sharesOutstanding").and_then(|v| v.as_f64()).or_else(|| {
                    obj.get("fundamental")
                        .and_then(|f| f.get("sharesOutstanding"))
                        .and_then(|v| v.as_f64())
                }),
                last_price,
            ) {
                obj.insert("marketCap".to_string(), serde_json::json!(shares * price));
            }
        }

        // Yield fallback: annual dividend / last price.
        let div_amount = obj.get("divAmount").and_then(|v| v.as_f64());
        if obj.get("dividendYield").is_none() || obj.get("dividendYield").and_then(|v| v.as_f64()).is_none() {
            if let (Some(div), Some(price)) = (div_amount, last_price) {
                if div > 0.0 && price > 0.0 {
                    // Schwab's divAmount is the most recent quarterly dividend for most equities.
                    // Annualize it by multiplying by 4 as a practical approximation.
                    let annual_yield = (div * 4.0) / price;
                    obj.insert("dividendYield".to_string(), serde_json::json!(annual_yield));
                }
            }
        }

        // Fetch option chain to calculate IV and MMM when not provided by the quote endpoint.
        let need_iv = obj.get("iv").is_none() || obj.get("iv").and_then(|v| v.as_f64()).is_none();
        let need_mmm = obj.get("mmm").is_none() || obj.get("mmm").and_then(|v| v.as_f64()).is_none();
        if need_iv || need_mmm {
            if let Ok(Some((chain_iv, dte))) = fetch_option_chain_iv_and_dte(symbol, last_price.unwrap_or(0.0)).await {
                if need_iv {
                    obj.insert("iv".to_string(), serde_json::json!(chain_iv));
                }
                if need_mmm {
                    let iv_decimal = if chain_iv > 1.0 { chain_iv / 100.0 } else { chain_iv };
                    // MMM is the expected one-sided move (±) to the nearest expiration.
                    // Approximate it as half the ATM straddle price.
                    let dte = dte.max(1);
                    let mmm = last_price.unwrap_or(0.0) * iv_decimal * (dte as f64 / 365.0).sqrt() / 2.0;
                    if mmm > 0.0 {
                        obj.insert("mmm".to_string(), serde_json::json!(mmm));
                    }
                }
            }
        }

        // Fetch price history once and derive HV, 50-day avg volume, and beta from it.
        let need_hv = obj.get("hv").is_none() || obj.get("hv").and_then(|v| v.as_f64()).is_none();
        let need_50d = obj.get("avg50DayVolume").is_none() || obj.get("avg50DayVolume").and_then(|v| v.as_u64()).is_none();
        let need_beta = obj.get("beta").is_none() || obj.get("beta").and_then(|v| v.as_f64()).is_none();

        if need_hv || need_50d {
            if let Ok(candles) = fetch_price_history_candles(symbol, "year", "1").await {
                if need_hv {
                    if let Some(hv) = calculate_historical_volatility(&candles) {
                        obj.insert("hv".to_string(), serde_json::json!(hv));
                    }
                }
                if need_50d {
                    if let Some(avg) = calculate_50day_avg_volume(&candles) {
                        obj.insert("avg50DayVolume".to_string(), serde_json::json!(avg));
                    }
                }
            }
        }

        if need_beta {
            if let Ok(Some(beta)) = calculate_beta(symbol).await {
                obj.insert("beta".to_string(), serde_json::json!(beta));
            }
        }

        // Earnings date from instruments fundamental projection.
        if obj.get("earningsDate").is_none() || obj.get("earningsDate").and_then(|v| v.as_str()).is_none() {
            if let Ok(Some(date)) = fetch_earnings_date(symbol).await {
                obj.insert("earningsDate".to_string(), serde_json::json!(date));
            }
        }
    }

    Ok(serde_json::to_string_pretty(&quote_data).unwrap_or_else(|_| "{\"error\": \"Failed to serialize\"}".to_string()))
}

/// Fetches the option chain and extracts implied volatility plus days to expiration.
/// Returns (iv as a percent value, days to expiration).
async fn fetch_option_chain_iv_and_dte(symbol: &str, underlying_price: f64) -> Result<Option<(f64, u64)>, String> {
    let response = client()
        .await?
        .option_chain(symbol)
        .await
        .map_err(|e| e.to_string())?;

    // Schwab often includes a top-level volatility figure.
    if let Some(vol) = response.get("volatility").and_then(|v| v.as_f64()) {
        let dte = response.get("daysToExpiration").and_then(|v| v.as_u64()).unwrap_or(30);
        return Ok(Some((vol, dte)));
    }

    // Otherwise find the nearest expiration and the ATM strike's call+put IV.
    let mut nearest_date: Option<String> = None;
    let mut min_dte: u64 = u64::MAX;

    for map_name in ["callExpDateMap", "putExpDateMap"] {
        if let Some(map) = response.get(map_name).and_then(|v| v.as_object()) {
            for key in map.keys() {
                // Schwab keys look like "2026-07-24:1" where the suffix is DTE.
                let parts: Vec<&str> = key.split(':').collect();
                if let Some(dte_str) = parts.get(1) {
                    if let Ok(dte) = dte_str.parse::<u64>() {
                        if dte < min_dte {
                            min_dte = dte;
                            nearest_date = Some(key.clone());
                        }
                    }
                }
            }
        }
    }

    let nearest_date = match nearest_date {
        Some(d) => d,
        None => return Ok(None),
    };

    // Collect strikes and their IVs from calls and puts, then pick the ATM strike.
    let mut strike_ivs: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
    for map_name in ["callExpDateMap", "putExpDateMap"] {
        if let Some(map) = response.get(map_name).and_then(|v| v.as_object()) {
            if let Some(strikes) = map.get(&nearest_date).and_then(|v| v.as_object()) {
                for (strike_str, option_list) in strikes {
                    if let Some(options) = option_list.as_array() {
                        for opt in options {
                            if let Some(iv) = opt.get("volatility").and_then(|v| v.as_f64()) {
                                strike_ivs.entry(strike_str.clone()).or_default().push(iv);
                            }
                        }
                    }
                }
            }
        }
    }

    if strike_ivs.is_empty() {
        return Ok(None);
    }

    let atm_strike = strike_ivs
        .keys()
        .filter_map(|s| s.parse::<f64>().ok())
        .min_by(|a, b| {
            let da = (a - underlying_price).abs();
            let db = (b - underlying_price).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

    let atm_strike_str = match atm_strike {
        Some(s) => format!("{}", s),
        None => return Ok(None),
    };

    let ivs = strike_ivs.get(&atm_strike_str).cloned().unwrap_or_default();
    if ivs.is_empty() {
        return Ok(None);
    }

    let avg_iv = ivs.iter().sum::<f64>() / ivs.len() as f64;
    Ok(Some((avg_iv, min_dte)))
}

/// Fetches daily price history candles for a symbol.
async fn fetch_price_history_candles(
    symbol: &str,
    period_type: &str,
    period: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let response = client()
        .await?
        .price_history(&[
            ("symbol", symbol),
            ("periodType", period_type),
            ("period", period),
            ("frequencyType", "daily"),
            ("frequency", "1"),
        ])
        .await
        .map_err(|e| e.to_string())?;

    Ok(response
        .get("candles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default())
}

/// Calculates annualized historical volatility from daily closing prices.
fn calculate_historical_volatility(candles: &[serde_json::Value]) -> Option<f64> {
    let closes: Vec<f64> = candles
        .iter()
        .filter_map(|c| c.get("close").and_then(|v| v.as_f64()))
        .collect();

    if closes.len() < 2 {
        return None;
    }

    let log_returns: Vec<f64> = closes
        .windows(2)
        .filter_map(|w| {
            if w[0] > 0.0 && w[1] > 0.0 {
                Some((w[1] / w[0]).ln())
            } else {
                None
            }
        })
        .collect();

    if log_returns.is_empty() {
        return None;
    }

    let mean = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
    let variance = log_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / log_returns.len() as f64;
    let daily_std = variance.sqrt();
    let annualized = daily_std * (252.0_f64).sqrt();

    // Return as a percent value (e.g. 25.0 for 25%).
    Some(annualized * 100.0)
}

/// Calculates the simple average volume over the most recent 50 daily candles.
fn calculate_50day_avg_volume(candles: &[serde_json::Value]) -> Option<u64> {
    let volumes: Vec<u64> = candles
        .iter()
        .rev()
        .take(50)
        .filter_map(|c| c.get("volume").and_then(|v| v.as_u64()))
        .collect();

    if volumes.is_empty() {
        return None;
    }

    Some(volumes.iter().sum::<u64>() / volumes.len() as u64)
}

/// Calculates beta against SPY using up to the last ~252 trading days of daily returns.
async fn calculate_beta(symbol: &str) -> Result<Option<f64>, String> {
    let symbol_candles = fetch_price_history_candles(symbol, "year", "2").await?;
    let benchmark_candles = fetch_price_history_candles("SPY", "year", "2").await?;

    let symbol_map = close_by_date(&symbol_candles);
    let benchmark_map = close_by_date(&benchmark_candles);

    let mut common_dates: Vec<i64> = symbol_map
        .keys()
        .filter(|k| benchmark_map.contains_key(*k))
        .filter_map(|k| k.parse::<i64>().ok())
        .collect();
    common_dates.sort_unstable();

    if common_dates.len() < 60 {
        return Ok(None);
    }

    // Use the most recent 252 trading days (~1 year), matching the typical beta window.
    let dates = &common_dates[common_dates.len().saturating_sub(252)..];

    let symbol_closes: Vec<f64> = dates
        .iter()
        .filter_map(|d| symbol_map.get(&d.to_string()).copied())
        .collect();
    let benchmark_closes: Vec<f64> = dates
        .iter()
        .filter_map(|d| benchmark_map.get(&d.to_string()).copied())
        .collect();

    let symbol_returns = log_returns(&symbol_closes);
    let benchmark_returns = log_returns(&benchmark_closes);

    if symbol_returns.len() < 30 || benchmark_returns.len() < 30 {
        return Ok(None);
    }

    let mean_s = symbol_returns.iter().sum::<f64>() / symbol_returns.len() as f64;
    let mean_b = benchmark_returns.iter().sum::<f64>() / benchmark_returns.len() as f64;

    let covariance = symbol_returns
        .iter()
        .zip(benchmark_returns.iter())
        .map(|(si, bi)| (si - mean_s) * (bi - mean_b))
        .sum::<f64>()
        / symbol_returns.len() as f64;
    let variance = benchmark_returns
        .iter()
        .map(|bi| (bi - mean_b).powi(2))
        .sum::<f64>()
        / benchmark_returns.len() as f64;

    if variance == 0.0 || variance.is_nan() {
        return Ok(None);
    }

    Ok(Some(covariance / variance))
}

fn close_by_date(candles: &[serde_json::Value]) -> std::collections::HashMap<String, f64> {
    let mut map = std::collections::HashMap::new();
    for c in candles {
        if let (Some(ts), Some(close)) = (
            c.get("datetime")
                .and_then(|v| v.as_i64())
                .or_else(|| c.get("date").and_then(|v| v.as_i64())),
            c.get("close").and_then(|v| v.as_f64()),
        ) {
            map.insert(ts.to_string(), close);
        }
    }
    map
}

fn log_returns(closes: &[f64]) -> Vec<f64> {
    closes
        .windows(2)
        .filter_map(|w| {
            if w[0] > 0.0 && w[1] > 0.0 {
                Some((w[1] / w[0]).ln())
            } else {
                None
            }
        })
        .collect()
}

/// Fetches the instrument fundamental projection and tries to find an earnings date.
async fn fetch_earnings_date(symbol: &str) -> Result<Option<String>, String> {
    let response = client()
        .await?
        .instruments(&[("symbol", symbol), ("projection", "fundamental")])
        .await
        .map_err(|e| e.to_string())?;

    let instruments = response
        .get("instruments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for inst in instruments {
        if let Some(fund) = inst.get("fundamental").and_then(|v| v.as_object()) {
            for key in ["earningsDate", "nextEarningsDate", "earningsAnnounceDate", "lastEarningsDate", "dividendDate", "exDate"] {
                if let Some(date) = fund.get(key).and_then(|v| v.as_str()) {
                    return Ok(Some(format_schwab_date(date.to_string())));
                }
            }
        }
        for key in ["earningsDate", "nextEarningsDate", "earningsAnnounceDate", "lastEarningsDate"] {
            if let Some(date) = inst.get(key).and_then(|v| v.as_str()) {
                return Ok(Some(format_schwab_date(date.to_string())));
            }
        }
    }

    Ok(None)
}

/// Formats a Schwab ISO-like date string (e.g. "2026-05-05T00:00:00Z") as MM/DD/YY.
/// Non-ISO strings are returned unchanged.
fn format_schwab_date(date_str: String) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&date_str) {
        return dt.format("%m/%d/%y").to_string();
    }
    if let Ok(naive) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
        return naive.format("%m/%d/%y").to_string();
    }
    date_str
}

/// Search result for an instrument (stock/ETF/etc.).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstrumentSearchResult {
    /// Ticker symbol.
    pub symbol: String,
    /// Company/fund name.
    pub description: String,
    /// Asset type (Equity, ETF, etc.).
    pub asset_type: String,
    /// Exchange.
    pub exchange: String,
}

/// `schwab search-instruments <query>` — Search instruments returning JSON array.
pub async fn search_instruments_json(query: &str) -> Result<String, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok("[]".to_string());
    }

    let results = client()
        .await?
        .instruments(&[("symbol", query), ("projection", "symbol-search")])
        .await
        .map_err(|e| e.to_string())?;

    let mut instruments = Vec::new();
    
    // Schwab returns {"instruments": [...]} - extract the array
    if let Some(obj) = results.as_object() {
        if let Some(instruments_array) = obj.get("instruments").and_then(|v| v.as_array()) {
            for inst_value in instruments_array {
                if let Some(inst) = inst_value.as_object() {
                    let symbol = inst
                        .get("symbol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let description = inst
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let asset_type = inst
                        .get("assetType")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let exchange = inst
                        .get("exchange")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    instruments.push(serde_json::json!({
                        "symbol": symbol,
                        "description": description,
                        "asset_type": asset_type,
                        "exchange": exchange,
                    }));
                }
            }
        }
    }

    instruments.sort_by(|a, b| {
        a["symbol"].as_str().unwrap_or("").cmp(b["symbol"].as_str().unwrap_or(""))
    });

    Ok(serde_json::to_string_pretty(&instruments).unwrap_or_else(|_| "[]".to_string()))
}

/// Searches for instruments by symbol or company name (internal use).
pub async fn search_instruments(query: &str) -> Result<Vec<InstrumentSearchResult>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let results = client()
        .await?
        .instruments(&[("symbol", query), ("projection", "symbol-search")])
        .await
        .map_err(|e| e.to_string())?;

    let mut instruments = Vec::new();
    
    // Schwab returns {"instruments": [...]} - extract the array
    if let Some(obj) = results.as_object() {
        if let Some(instruments_array) = obj.get("instruments").and_then(|v| v.as_array()) {
            for inst_value in instruments_array {
                if let Some(inst) = inst_value.as_object() {
                    let symbol = inst
                        .get("symbol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let description = inst
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let asset_type = inst
                        .get("assetType")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let exchange = inst
                        .get("exchange")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    instruments.push(InstrumentSearchResult {
                        symbol,
                        description,
                        asset_type,
                        exchange,
                    });
                }
            }
        }
    }

    instruments.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    
    Ok(instruments)
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn preference_for_account_matches_by_account_number() {
        let preferences = json!({
            "accounts": [
                {"accountNumber": "17763688", "nickName": "Day Trading", "displayAcctId": "...688", "primaryAccount": true},
                {"accountNumber": "23417011", "nickName": "Darcy Roll", "displayAcctId": "...011"}
            ]
        });

        let first = super::preference_for_account("17763688", &preferences).unwrap();
        assert_eq!(first["nickName"], "Day Trading");
        assert_eq!(first["displayAcctId"], "...688");
        assert!(super::preference_for_account("99999999", &preferences).is_none());
    }

    #[test]
    fn account_summary_parses_cash_account_response() {
        let response = json!({
            "securitiesAccount": {
                "type": "CASH",
                "accountNumber": "80544751",
                "currentBalances": {
                    "cashBalance": 4745.2,
                    "liquidationValue": 4745.2,
                    "cashAvailableForTrading": 4745.2,
                    "cashAvailableForWithdrawal": 4745.2,
                    "longNonMarginableMarketValue": 4745.2,
                    "totalCash": 4745.2
                },
                "projectedBalances": {
                    "cashAvailableForTrading": 4745.2,
                    "cashAvailableForWithdrawal": 4745.2
                }
            },
            "aggregatedBalance": {
                "currentLiquidationValue": 4745.2,
                "liquidationValue": 4745.2
            }
        });

        assert_eq!(super::balance_f64(&response, "liquidationValue"), 4745.2);
        assert_eq!(super::balance_f64(&response, "cashAvailableForWithdrawal"), 4745.2);
        assert_eq!(super::balance_f64(&response, "cashAvailableForTrading"), 4745.2);
    }

    #[test]
    fn parse_positions_includes_cash_and_security_rows() {
        let response = json!({
            "securitiesAccount": {
                "type": "MARGIN",
                "accountNumber": "23417011",
                "currentBalances": {
                    "cashBalance": 8367.51,
                    "liquidationValue": 11314.30
                },
                "positions": [
                    {
                        "instrument": {"symbol": "SCHG", "assetType": "ETF"},
                        "longQuantity": 86,
                        "shortQuantity": 0,
                        "averagePrice": 33.15,
                        "marketValue": 2946.79,
                        "currentDayProfitLoss": 9.89,
                        "longOpenProfitLoss": 95.89,
                        "amountPaid": 2850.90,
                        "maintenanceRequirement": 0.0
                    }
                ]
            }
        });

        let rows = super::parse_positions(&response);
        assert_eq!(rows.len(), 3);

        let cash = rows.iter().find(|r| r.position == "Cash").unwrap();
        assert_eq!(cash.net_liq, "$8,367.51");

        let schg = rows.iter().find(|r| r.position == "SCHG").unwrap();
        assert_eq!(schg.qty, "86");
        assert_eq!(schg.cost, "$2,850.90");
        assert_eq!(schg.net_liq, "$2,946.79");
        assert_eq!(schg.trade_price, "$33.15");
        assert_eq!(schg.pl_day, "$9.89");
        assert_eq!(schg.pl_open, "$95.89");

        let totals = rows.iter().find(|r| r.position == "Totals:").unwrap();
        assert_eq!(totals.qty, "—");
        assert_eq!(totals.pl_day, "$9.89");
        assert_eq!(totals.pl_open, "$95.89");
        assert_eq!(totals.cost, "$11,218.41");
        assert_eq!(totals.net_liq, "$11,314.30");
    }

    #[test]
    fn parse_orders_flattens_oco_stop_and_limit_children() {
        let response = json!([
            {
                "orderId": 1001,
                "orderStrategyType": "OCO",
                "enteredTime": "2026-06-09T05:52:38+0000",
                "status": "WORKING",
                "childOrderStrategies": [
                    {
                        "orderId": 1002,
                        "orderStrategyType": "SINGLE",
                        "orderType": "STOP",
                        "stopPrice": 31.96,
                        "price": 0,
                        "duration": "GTC",
                        "status": "WORKING",
                        "enteredTime": "2026-06-09T05:52:38+0000",
                        "quantity": 0,
                        "orderLegCollection": [
                            {
                                "instruction": "SELL",
                                "positionEffect": "CLOSING",
                                "quantity": 86,
                                "instrument": {"symbol": "SCHG"}
                            }
                        ]
                    },
                    {
                        "orderId": 1003,
                        "orderStrategyType": "SINGLE",
                        "orderType": "LIMIT",
                        "price": 35.00,
                        "duration": "GTC",
                        "status": "WORKING",
                        "enteredTime": "2026-06-09T05:52:38+0000",
                        "quantity": 0,
                        "orderLegCollection": [
                            {
                                "instruction": "SELL",
                                "positionEffect": "CLOSING",
                                "quantity": 86,
                                "instrument": {"symbol": "SCHG"}
                            }
                        ]
                    }
                ]
            }
        ]);

        let rows = super::parse_orders(response);
        assert_eq!(rows.len(), 2);

        let stop = rows.iter().find(|r| r.price == "31.96 STOP").unwrap();
        assert_eq!(stop.side, "SELL");
        assert_eq!(stop.pos_effect, "TO CLOSE");
        assert_eq!(stop.qty, "-86");
        assert_eq!(stop.amount, "$2,748.56");
        assert_eq!(stop.symbol, "SCHG");
        assert_eq!(stop.desc, "[OCO]");
        assert_eq!(stop.tif, "GTC");
        assert_eq!(stop.status, "WORKING");

        let limit = rows.iter().find(|r| r.price == "35.00 LIMIT").unwrap();
        assert_eq!(limit.qty, "-86");
        assert_eq!(limit.amount, "$3,010.00");
        assert_eq!(limit.desc, "[OCO]");
    }
}
