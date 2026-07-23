//! AI stock assistant for the Trade screen's chat panel.
//!
//! Backed directly by `nest-ai-ollama` (no `nest-ai::AiService` indirection —
//! there's only ever one provider here, so the DI wrapper adds nothing).
//! Grounded with the real Schwab quote and recent price history for the
//! symbol in question, reusing `crate::schwab`'s existing pretty-printed
//! JSON accessors rather than re-fetching/re-parsing raw responses.
//!
//! Supports multi-turn conversations with context window management and
//! tool use (web search for research, etc.).

use futures_util::StreamExt;
use nest_ai::{
    AiProvider, ChatMessage, ChatRole, CompletionChunk, CompletionRequest, ToolCall, ToolDefinition,
};
use nest_ai_ollama::{OllamaConfig, OllamaProvider};
use nest_http_client::HttpClientService;
use serde_json::json;

use crate::schwab;

/// Maximum number of messages to keep in conversation history.
const MAX_CONVERSATION_HISTORY: usize = 20;

/// Maximum characters in system prompt context (reserves space in context window).
const MAX_CONTEXT_CHARS: usize = 4000;

/// Creates the system prompt for the trading assistant.
fn create_system_prompt(symbol: &str, context: &str) -> String {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    format!(
        "You are a quantitative trading analyst for Finch. Today's date is {today}. \
         You analyze {symbol} using live market data returned by the tools below. \
         The user is an experienced trader who has explicitly requested numbers, calculations, and trade setups.\n\n\
         Current focus: {symbol}\n\n\
         {context}\n\n\
         Tool usage rules:\n\
         - Use fetch_quote_details when the user asks about current price, volume, fundamentals (PE, EPS, market cap, beta, etc.), or today's quote stats.\n\
         - Use fetch_candlestick_data when the user asks about price action, chart patterns, historical prices, or wants data for a specific timeframe.\n\
         - Use analyze_price_trend when the user asks about performance, trend direction, moving averages, or support/resistance levels.\n\
         - Use calculate_rsi when the user asks about RSI, overbought/oversold conditions, or momentum.\n\
         - Use calculate_trade_setup when the user asks for a buy price, stop loss, limit/target price, position size, or risk/reward math for a trade.\n\
         - Use web_search ONLY when the user explicitly asks for news, recent events, external research, or information not available in market data. Do not use web_search for general price or analysis questions.\n\n\
         Response rules:\n\
         - Use the provided market data as ground truth\n\
         - Provide actionable output with specific numbers, prices, and math\n\
         - After receiving tool results, output the answer immediately. Do not describe that you called a tool, do not output slash commands, and do not add meta-commentary\n\
         - Do not include disclaimers, warnings, caveats, or statements about 'not being investment advice', 'past performance', 'risk tolerance', or 'consult a professional'\n\
         - Do not mention your knowledge cutoff, training data, or past dates such as 2023. The data you have is live.\n\
         - If a market-data tool fails, state briefly that live data is unavailable and do not invent numbers\n\
         - For trade setups, first output the clean formatted setup details, then ask if the user wants to populate the order ticket, then include the <TRADE_SETUP> JSON block on its own line\n\
         - The calculate_trade_setup tool returns JSON. Parse it and present the values in a clean human-readable format before asking to populate the order ticket\n\
         - Be concise and factual\n\n\
         Example for a trade setup question:\n\
         User: I have $10,000, want 3% gain, 1% risk, what's the setup for AMD?\n\
         Assistant: [uses calculate_trade_setup]\n\
         AMD trade setup (long):\n\
         Entry: $553.95\n\
         Stop: $548.41\n\
         Target: $570.57\n\
         Shares: 18\n\
         Position size: $9,971.10\n\
         Risk: $99.72 (1.0% of account)\n\
         Reward: $299.16\n\
         Risk/reward: 1:3.0\n\n\
         Would you like me to populate the order ticket with this setup?\n\
         <TRADE_SETUP>{{\"symbol\":\"AMD\",\"entry\":553.95,\"stop\":548.41,\"target\":570.57,\"shares\":18,\"risk\":99.72,\"reward\":299.16,\"position_size\":9971.10,\"risk_percent_of_account\":1.0,\"risk_reward_ratio\":3.0}}</TRADE_SETUP>"
    )
}

/// Builds grounding context from Schwab market data.
async fn build_context(symbol: &str) -> String {
    let quote = schwab::quote(symbol)
        .await
        .unwrap_or_else(|err| format!("(quote unavailable: {err})"));
    let history = schwab::price_history(
        symbol,
        Some("month"),
        Some("1"),
        Some("daily"),
        None,
        None,
        None,
    )
    .await
    .unwrap_or_else(|err| format!("(price history unavailable: {err})"));

    format!(
        "Current quote for {symbol}:\n{quote}\n\n\
         Recent price history for {symbol} (last month, daily):\n{history}"
    )
}

/// Truncates context to fit within character limit while preserving important data.
fn truncate_context(context: String, max_chars: usize) -> String {
    if context.len() <= max_chars {
        return context;
    }

    // Try to keep the quote section intact, truncate history if needed
    if let Some(history_start) = context.find("Recent price history") {
        let quote_part = &context[..history_start];
        let remaining = max_chars.saturating_sub(quote_part.len());
        if remaining > 100 {
            return format!("{}{}", quote_part, &context[history_start..].chars().take(remaining).collect::<String>());
        }
    }

    // Fallback: simple truncation with warning
    format!("{}...(truncated)", context.chars().take(max_chars).collect::<String>())
}

/// Web search tool definition for finding news and research.
fn web_search_tool_definition() -> ToolDefinition {
    ToolDefinition::new(
        "web_search",
        "Search the web for recent news, press releases, or external information. \
         ONLY use this when the user explicitly asks for news, recent events, or external research about a company or market. \
         Do NOT use this for price, chart, or fundamental analysis questions that can be answered with market data tools.".to_string(),
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query - be specific and include relevant keywords like company names, ticker symbols, or topics"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (default: 5, max: 10)",
                    "default": 5
                }
            },
            "required": ["query"]
        }),
    )
}

/// Fetches current quote and fundamental data for the symbol.
fn fetch_quote_details_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "fetch_quote_details",
        format!("Fetch current quote and fundamental data for {symbol}. \
         Use this when the user asks about current price, volume, PE ratio, EPS, market cap, beta, dividend yield, or other quote stats."),
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    )
}

/// Fetches OHLC candlestick data for a specific timeframe.
fn fetch_candlestick_data_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "fetch_candlestick_data",
        format!("Fetch candlestick (OHLC) price history for {symbol}. \
         Use this when the user asks about price action, chart patterns, historical prices, or wants data for a specific timeframe."),
        json!({
            "type": "object",
            "properties": {
                "timeframe": {
                    "type": "string",
                    "description": "Timeframe to fetch, e.g. '1d', '1w', '1m', '3m', '6m', '1y'",
                    "default": "1m"
                },
                "interval": {
                    "type": "string",
                    "description": "Candle interval, e.g. '1m', '5m', '15m', '30m', '1h', '1d', '1w', '1mo'",
                    "default": "1d"
                }
            },
            "required": []
        }),
    )
}

/// Analyzes recent price trend for the symbol.
fn analyze_price_trend_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "analyze_price_trend",
        format!("Analyze recent price trend for {symbol}. \
         Use this when the user asks about performance, trend direction, momentum, moving averages, or support/resistance levels."),
        json!({
            "type": "object",
            "properties": {
                "timeframe": {
                    "type": "string",
                    "description": "Timeframe to analyze, e.g. '1w', '1m', '3m', '6m', '1y'",
                    "default": "1m"
                }
            },
            "required": []
        }),
    )
}

/// Calculates RSI for the symbol.
fn calculate_rsi_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "calculate_rsi",
        format!("Calculate the Relative Strength Index (RSI) for {symbol} over a lookback period. \
         Use this when the user asks about RSI, overbought/oversold conditions, or momentum."),
        json!({
            "type": "object",
            "properties": {
                "timeframe": {
                    "type": "string",
                    "description": "Timeframe to analyze, e.g. '1m', '3m', '6m', '1y'",
                    "default": "3m"
                },
                "period": {
                    "type": "integer",
                    "description": "RSI lookback period in days (default: 14)",
                    "default": 14
                }
            },
            "required": []
        }),
    )
}

/// Calculates a trade setup with entry, stop, target, position size, and risk/reward.
fn calculate_trade_setup_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "calculate_trade_setup",
        format!("Calculate a trade setup for {symbol} based on the user's risk/reward parameters. \
         Use this when the user asks for a buy price, stop loss, limit/target price, how many shares to buy, or risk/reward math."),
        json!({
            "type": "object",
            "properties": {
                "account_size": {
                    "type": "number",
                    "description": "Total account size or capital available for the trade in USD"
                },
                "risk_percent": {
                    "type": "number",
                    "description": "Maximum risk as a percentage of account size (e.g. 1.0 for 1%)"
                },
                "target_percent": {
                    "type": "number",
                    "description": "Target gain as a percentage of the entry price (e.g. 3.0 for 3%)"
                },
                "entry_price": {
                    "type": "number",
                    "description": "Optional specific entry price. If omitted, the current market price is used."
                }
            },
            "required": ["account_size", "risk_percent", "target_percent"]
        }),
    )
}

/// Executes a web search using DuckDuckGo's instant answer API.
async fn execute_web_search(
    http_client: &HttpClientService,
    query: &str,
    _num_results: Option<u32>,
) -> Result<String, String> {
    // Use DuckDuckGo's instant answer API (no HTML parsing needed)
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://api.duckduckgo.com/?q={}&format=json&pretty=1", encoded_query);
    
    match http_client.get_json::<serde_json::Value>(&url).await {
        Ok(response) => {
            let mut results = Vec::new();
            
            // Extract abstract if available
            if let Some(abstract_text) = response.get("Abstract").and_then(|v| v.as_str()) {
                if !abstract_text.is_empty() {
                    results.push(format!("Summary: {}", abstract_text));
                }
            }
            
            // Extract related topics
            if let Some(topics) = response.get("RelatedTopics").and_then(|v| v.as_array()) {
                for (i, topic) in topics.iter().take(5).enumerate() {
                    if let Some(text) = topic.get("Text").and_then(|v| v.as_str()) {
                        results.push(format!("{}. {}", i + 1, text));
                    }
                }
            }
            
            if results.is_empty() {
                Ok(format!("No direct results found for '{}'. Try rephrasing your query.", query))
            } else {
                Ok(format!("Research results for '{}':\n\n{}", query, results.join("\n")))
            }
        }
        Err(e) => Err(format!("Web search failed: {}", e))
    }
}

/// Maps a natural-language timeframe to Schwab price-history parameters.
fn map_timeframe_args(
    timeframe: Option<&str>,
    interval: Option<&str>,
) -> (&'static str, &'static str, &'static str, &'static str) {
    let timeframe = timeframe.unwrap_or("1m");
    let interval = interval.unwrap_or("1d");

    let (period_type, period) = match timeframe {
        "1d" => ("day", "1"),
        "3d" => ("day", "3"),
        "1w" => ("day", "5"),
        "2w" => ("day", "10"),
        "1m" => ("month", "1"),
        "3m" => ("month", "3"),
        "6m" => ("month", "6"),
        "ytd" => ("ytd", "1"),
        "1y" => ("year", "1"),
        "3y" => ("year", "3"),
        "5y" => ("year", "5"),
        "15y" => ("year", "15"),
        "max" => ("year", "20"),
        _ => ("month", "1"),
    };

    let (mut frequency_type, mut frequency) = match interval {
        "1m" => ("minute", "1"),
        "5m" => ("minute", "5"),
        "15m" => ("minute", "15"),
        "30m" => ("minute", "30"),
        "1h" => ("minute", "30"), // 30m data, can be aggregated if needed
        "1d" => ("daily", "1"),
        "1w" => ("weekly", "1"),
        "1mo" => ("monthly", "1"),
        _ => ("daily", "1"),
    };

    // Enforce Schwab's valid periodType/frequencyType combinations.
    // day -> minute only; month/year/ytd -> daily/weekly/monthly only.
    if period_type == "day" && frequency_type != "minute" {
        frequency_type = "minute";
        frequency = "30";
    } else if period_type != "day" && frequency_type == "minute" {
        frequency_type = "daily";
        frequency = "1";
    }

    (period_type, period, frequency_type, frequency)
}

/// Fetches current quote details as a JSON string.
async fn execute_fetch_quote_details(symbol: &str) -> Result<String, String> {
    schwab::quote_json(symbol).await
}

/// Fetches candlestick data and returns a concise summary.
async fn execute_fetch_candlestick_data(
    symbol: &str,
    timeframe: Option<&str>,
    interval: Option<&str>,
) -> Result<String, String> {
    let (period_type, period, frequency_type, frequency) = map_timeframe_args(timeframe, interval);

    let history_str = schwab::price_history(
        symbol,
        Some(period_type),
        Some(period),
        Some(frequency_type),
        Some(frequency),
        None,
        None,
    )
    .await?;

    let history: serde_json::Value = serde_json::from_str(&history_str)
        .map_err(|e| format!("Failed to parse price history: {e}"))?;

    // Summarize the data for the model.
    let candles = history
        .get("candles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if candles.is_empty() {
        return Ok("No candlestick data returned for the requested timeframe.".to_string());
    }

    let first = candles.first().and_then(|c| c.as_object());
    let last = candles.last().and_then(|c| c.as_object());

    let first_close = first.and_then(|c| c.get("close")).and_then(|v| v.as_f64());
    let last_close = last.and_then(|c| c.get("close")).and_then(|v| v.as_f64());
    let high = candles
        .iter()
        .filter_map(|c| c.get("high").and_then(|v| v.as_f64()))
        .fold(f64::NEG_INFINITY, f64::max);
    let low = candles
        .iter()
        .filter_map(|c| c.get("low").and_then(|v| v.as_f64()))
        .fold(f64::INFINITY, f64::min);

    let change_pct = match (first_close, last_close) {
        (Some(first), Some(last)) if first != 0.0 => Some(((last - first) / first) * 100.0),
        _ => None,
    };

    let timeframe_label = timeframe.unwrap_or("1m");
    let interval_label = interval.unwrap_or("1d");

    Ok(format!(
        "Candlestick data for {symbol} ({timeframe_label}, {interval_label}): {} candles. \
         Period high: {high:.2}, period low: {low:.2}, first close: {first}, last close: {last}, change: {change}",
        candles.len(),
        first = first_close.map(|v| format!("{v:.2}")).unwrap_or_else(|| "N/A".to_string()),
        last = last_close.map(|v| format!("{v:.2}")).unwrap_or_else(|| "N/A".to_string()),
        change = change_pct.map(|v| format!("{v:+.2}%")).unwrap_or_else(|| "N/A".to_string()),
    ))
}

/// Analyzes price trend and returns key metrics.
async fn execute_analyze_price_trend(
    symbol: &str,
    timeframe: Option<&str>,
) -> Result<String, String> {
    let (period_type, period, frequency_type, frequency) =
        map_timeframe_args(timeframe, Some("1d"));

    let history_str = schwab::price_history(
        symbol,
        Some(period_type),
        Some(period),
        Some(frequency_type),
        Some(frequency),
        None,
        None,
    )
    .await?;

    let history: serde_json::Value = serde_json::from_str(&history_str)
        .map_err(|e| format!("Failed to parse price history: {e}"))?;

    let candles = history
        .get("candles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if candles.len() < 2 {
        return Ok("Not enough price history to analyze trend.".to_string());
    }

    let closes: Vec<f64> = candles
        .iter()
        .filter_map(|c| c.get("close").and_then(|v| v.as_f64()))
        .collect();

    let first = closes.first().copied().unwrap_or(0.0);
    let last = closes.last().copied().unwrap_or(0.0);
    let high = closes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let low = closes.iter().copied().fold(f64::INFINITY, f64::min);

    let sma_20 = if closes.len() >= 20 {
        let sum: f64 = closes.iter().rev().take(20).sum();
        Some(sum / 20.0)
    } else {
        None
    };

    let sma_50 = if closes.len() >= 50 {
        let sum: f64 = closes.iter().rev().take(50).sum();
        Some(sum / 50.0)
    } else {
        None
    };

    let change_pct = if first != 0.0 {
        Some(((last - first) / first) * 100.0)
    } else {
        None
    };

    let above_sma20 = sma_20.map(|sma| last > sma);
    let above_sma50 = sma_50.map(|sma| last > sma);

    let timeframe_label = timeframe.unwrap_or("1m");

    Ok(format!(
        "Trend analysis for {symbol} ({timeframe_label}):\n\
         - Latest close: {last:.2}\n\
         - Period high: {high:.2}\n\
         - Period low: {low:.2}\n\
         - Period change: {change}\n\
         - 20-day SMA: {sma20} ({above20})\n\
         - 50-day SMA: {sma50} ({above50})",
        change = change_pct.map(|v| format!("{v:+.2}%")).unwrap_or_else(|| "N/A".to_string()),
        sma20 = sma_20.map(|v| format!("{v:.2}")).unwrap_or_else(|| "N/A".to_string()),
        above20 = above_sma20
            .map(|b| if b { "price above" } else { "price below" })
            .unwrap_or("N/A"),
        sma50 = sma_50.map(|v| format!("{v:.2}")).unwrap_or_else(|| "N/A".to_string()),
        above50 = above_sma50
            .map(|b| if b { "price above" } else { "price below" })
            .unwrap_or("N/A"),
    ))
}

/// Calculates RSI from daily closing prices using the Wilder smoothing method.
async fn execute_calculate_rsi(
    symbol: &str,
    timeframe: Option<&str>,
    period: Option<u64>,
) -> Result<String, String> {
    let period = period.unwrap_or(14) as usize;
    let (period_type, period_str, frequency_type, frequency) =
        map_timeframe_args(timeframe, Some("1d"));

    let history_str = schwab::price_history(
        symbol,
        Some(period_type),
        Some(period_str),
        Some(frequency_type),
        Some(frequency),
        None,
        None,
    )
    .await?;

    let history: serde_json::Value = serde_json::from_str(&history_str)
        .map_err(|e| format!("Failed to parse price history: {e}"))?;

    let closes: Vec<f64> = history
        .get("candles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|c| c.get("close").and_then(|v| v.as_f64()))
        .collect();

    if closes.len() <= period {
        return Ok(format!(
            "Not enough price history to calculate RSI (need > {period} days)."
        ));
    }

    let rsi_values = calculate_rsi_values(&closes, period);
    let current = rsi_values.last().copied();
    let avg = if !rsi_values.is_empty() {
        Some(rsi_values.iter().sum::<f64>() / rsi_values.len() as f64)
    } else {
        None
    };
    let min = rsi_values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = rsi_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    Ok(format!(
        "RSI analysis for {symbol} ({period}-day lookback):\n\
         - Current RSI: {current}\n\
         - Average RSI over period: {avg}\n\
         - Minimum RSI: {min:.2}\n\
         - Maximum RSI: {max:.2}",
        current = current.map(|v| format!("{v:.2}")).unwrap_or_else(|| "N/A".to_string()),
        avg = avg.map(|v| format!("{v:.2}")).unwrap_or_else(|| "N/A".to_string()),
    ))
}

fn calculate_rsi_values(closes: &[f64], period: usize) -> Vec<f64> {
    if closes.len() <= period {
        return Vec::new();
    }

    let mut gains = Vec::new();
    let mut losses = Vec::new();

    for w in closes.windows(2) {
        let change = w[1] - w[0];
        gains.push(change.max(0.0));
        losses.push((-change).max(0.0));
    }

    let mut avg_gain = gains.iter().take(period).sum::<f64>() / period as f64;
    let mut avg_loss = losses.iter().take(period).sum::<f64>() / period as f64;

    let mut rsi_values = Vec::new();
    if avg_loss == 0.0 {
        rsi_values.push(100.0);
    } else {
        let rs = avg_gain / avg_loss;
        rsi_values.push(100.0 - (100.0 / (1.0 + rs)));
    }

    for i in period..gains.len() {
        avg_gain = (avg_gain * (period as f64 - 1.0) + gains[i]) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + losses[i]) / period as f64;
        if avg_loss == 0.0 {
            rsi_values.push(100.0);
        } else {
            let rs = avg_gain / avg_loss;
            rsi_values.push(100.0 - (100.0 / (1.0 + rs)));
        }
    }

    rsi_values
}

/// Calculates a trade setup based on risk/reward parameters and current market data.
async fn execute_calculate_trade_setup(
    symbol: &str,
    account_size: f64,
    risk_percent: f64,
    target_percent: f64,
    entry_price: Option<f64>,
) -> Result<String, String> {
    // The model may pass percentages as decimals (0.015) or as percent values (1.5).
    // Normalize to percent values (1.5).
    let risk_percent = if risk_percent <= 1.0 { risk_percent * 100.0 } else { risk_percent };
    let target_percent = if target_percent <= 1.0 { target_percent * 100.0 } else { target_percent };

    let quote_str = schwab::quote_json(symbol).await?;
    let quote: serde_json::Value = serde_json::from_str(&quote_str)
        .map_err(|e| format!("Failed to parse quote: {e}"))?;

    let last_price = quote
        .get("lastPrice")
        .and_then(|v| v.as_f64())
        .or_else(|| quote.get("closePrice").and_then(|v| v.as_f64()));

    let entry = entry_price.or(last_price).unwrap_or(0.0);
    if entry <= 0.0 {
        return Ok("Could not determine a valid entry price for the trade setup.".to_string());
    }

    // For a long position.
    let stop = entry * (1.0 - risk_percent / 100.0);
    let target = entry * (1.0 + target_percent / 100.0);
    let risk_per_share = entry - stop;
    let reward_per_share = target - entry;

    if risk_per_share <= 0.0 {
        return Ok("Invalid trade setup: risk per share must be greater than zero.".to_string());
    }

    let max_risk_dollars = account_size * (risk_percent / 100.0);
    let shares_by_risk = (max_risk_dollars / risk_per_share).floor();
    let shares_by_account = (account_size / entry).floor();
    let shares = shares_by_risk.min(shares_by_account) as u64;

    if shares == 0 {
        return Ok(format!(
            "With ${account_size:.2} and a {risk_percent:.2}% risk target, the account is too small to take that risk position in {symbol} at ${entry:.2}."
        ));
    }

    let position_size = shares as f64 * entry;
    let actual_risk = shares as f64 * risk_per_share;
    let potential_reward = shares as f64 * reward_per_share;
    let risk_reward_ratio = reward_per_share / risk_per_share;
    let risk_of_account = (actual_risk / account_size) * 100.0;

    let setup_json = serde_json::json!({
        "symbol": symbol,
        "entry": round2(entry),
        "stop": round2(stop),
        "target": round2(target),
        "shares": shares,
        "risk": round2(actual_risk),
        "reward": round2(potential_reward),
        "position_size": round2(position_size),
        "risk_percent_of_account": round2(risk_of_account),
        "risk_reward_ratio": round2(risk_reward_ratio),
    });

    // Return a compact JSON payload for the model. The model is responsible for
    // formatting the human-readable summary and appending the <TRADE_SETUP> block.
    Ok(setup_json.to_string())
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Executes a tool call and returns the result.
async fn execute_tool_call(
    tool_call: &ToolCall,
    symbol: &str,
    http_client: &HttpClientService,
) -> Result<String, String> {
    match tool_call.name.as_str() {
        "web_search" => {
            let query = tool_call.arguments
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'query' argument for web_search".to_string())?;

            let num_results = tool_call.arguments
                .get("num_results")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);

            execute_web_search(http_client, query, num_results).await
        }
        "fetch_quote_details" => execute_fetch_quote_details(symbol).await,
        "fetch_candlestick_data" => {
            let timeframe = tool_call.arguments.get("timeframe").and_then(|v| v.as_str());
            let interval = tool_call.arguments.get("interval").and_then(|v| v.as_str());
            execute_fetch_candlestick_data(symbol, timeframe, interval).await
        }
        "analyze_price_trend" => {
            let timeframe = tool_call.arguments.get("timeframe").and_then(|v| v.as_str());
            execute_analyze_price_trend(symbol, timeframe).await
        }
        "calculate_rsi" => {
            let timeframe = tool_call.arguments.get("timeframe").and_then(|v| v.as_str());
            let period = tool_call.arguments.get("period").and_then(|v| v.as_u64());
            execute_calculate_rsi(symbol, timeframe, period).await
        }
        "calculate_trade_setup" => {
            let account_size = tool_call
                .arguments
                .get("account_size")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "Missing 'account_size' argument".to_string())?;
            let risk_percent = tool_call
                .arguments
                .get("risk_percent")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "Missing 'risk_percent' argument".to_string())?;
            let target_percent = tool_call
                .arguments
                .get("target_percent")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "Missing 'target_percent' argument".to_string())?;
            let entry_price = tool_call.arguments.get("entry_price").and_then(|v| v.as_f64());
            execute_calculate_trade_setup(symbol, account_size, risk_percent, target_percent, entry_price).await
        }
        _ => Err(format!("Unknown tool: {}", tool_call.name)),
    }
}

/// Manages conversation history with context window limits.
pub struct ConversationHistory {
    messages: Vec<ChatMessage>,
    system_prompt: String,
}

impl ConversationHistory {
    pub fn new(system_prompt: String) -> Self {
        Self {
            messages: Vec::new(),
            system_prompt,
        }
    }

    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(ChatMessage::user(content));
        self.trim_if_needed();
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.messages.push(ChatMessage::assistant(content));
        self.trim_if_needed();
    }

    pub fn add_assistant_tool_calls(&mut self, tool_calls: Vec<ToolCall>) {
        self.messages.push(ChatMessage::assistant_tool_calls(tool_calls));
        self.trim_if_needed();
    }

    pub fn add_tool_result(&mut self, tool_name: String, content: String) {
        self.messages.push(ChatMessage::tool_result(tool_name, content));
        self.trim_if_needed();
    }

    fn trim_if_needed(&mut self) {
        // Keep most recent messages, remove oldest first (except we always keep system prompt separate)
        while self.messages.len() > MAX_CONVERSATION_HISTORY {
            // Remove oldest non-system message
            if let Some(first_non_system) = self.messages.iter().position(|m| m.role != ChatRole::System) {
                self.messages.remove(first_non_system);
            } else {
                break;
            }
        }
    }

    pub fn build_request(&self, tools: Vec<ToolDefinition>) -> CompletionRequest {
        let mut messages = vec![ChatMessage::system(self.system_prompt.clone())];
        messages.extend(self.messages.clone());
        
        CompletionRequest {
            model: None,
            messages,
            format: None,
            tools,
        }
    }
}

/// Streams an answer to a free-form question about `symbol`, invoking
/// `on_chunk` with each incremental text fragment as it arrives. Returns
/// once the model signals it is done.
///
/// This version supports multi-turn conversations with tool use.
pub async fn ask_stock_question_stream(
    ollama_config: &OllamaConfig,
    http_client: &HttpClientService,
    symbol: &str,
    initial_question: &str,
    mut on_chunk: impl FnMut(String) + Send,
) -> Result<(), String> {
    let context = build_context(symbol).await;
    let truncated_context = truncate_context(context, MAX_CONTEXT_CHARS);
    let system_prompt = create_system_prompt(symbol, &truncated_context);
    
    let mut conversation = ConversationHistory::new(system_prompt);
    conversation.add_user_message(initial_question.to_string());
    
    let tools = vec![
        web_search_tool_definition(),
        fetch_quote_details_tool_definition(symbol),
        fetch_candlestick_data_tool_definition(symbol),
        analyze_price_trend_tool_definition(symbol),
        calculate_rsi_tool_definition(symbol),
        calculate_trade_setup_tool_definition(symbol),
    ];
    let provider = OllamaProvider::new(ollama_config.clone())
        .map_err(|err| err.to_string())?;

    // Main conversation loop - handles tool calls and multi-turn
    let mut iteration = 0;
    const MAX_ITERATIONS: usize = 5; // Prevent infinite loops
    
    while iteration < MAX_ITERATIONS {
        iteration += 1;
        
        let request = conversation.build_request(tools.clone());
        
        let mut stream = provider
            .stream_complete(request)
            .await
            .map_err(|err| err.to_string())?;
        
        let mut response_content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        
        // Stream the response
        while let Some(chunk_result) = stream.next().await {
            let chunk: CompletionChunk = chunk_result.map_err(|err| err.to_string())?;
            
            // Accumulate text chunks
            if !chunk.content_delta.is_empty() {
                response_content.push_str(&chunk.content_delta);
                on_chunk(chunk.content_delta);
            }
            
            // Accumulate tool call fragments
            if !chunk.tool_calls.is_empty() {
                nest_ai::tools::merge_tool_calls(&mut tool_calls, &chunk.tool_calls);
            }
            
            if chunk.done {
                break;
            }
        }
        
        // If no tool calls, we're done
        if tool_calls.is_empty() {
            conversation.add_assistant_message(response_content);
            return Ok(());
        }

        // Execute tool calls and feed results back to model.
        // Record the assistant message with the tool calls so the model can
        // match results to the correct invocation on the next turn.
        if response_content.trim().is_empty() {
            conversation.add_assistant_tool_calls(tool_calls.clone());
        } else {
            conversation.add_assistant_message(response_content);
        }
        
        for tool_call in tool_calls {
            // Let user know a tool is being used.
            let tool_label = match tool_call.name.as_str() {
                "web_search" => "Researching",
                "fetch_quote_details" => "Fetching quote details",
                "fetch_candlestick_data" => "Fetching candlestick data",
                "analyze_price_trend" => "Analyzing price trend",
                "calculate_rsi" => "Calculating RSI",
                "calculate_trade_setup" => "Calculating trade setup",
                _ => "Using tool",
            };
            on_chunk(format!("\n\n🔍 {tool_label}..."));

            match execute_tool_call(&tool_call, symbol, http_client).await {
                Ok(result) => {
                    conversation.add_tool_result(tool_call.name.clone(), result.clone());

                    // Continue loop to get model's response with the tool result
                }
                Err(err) => {
                    let message = format!(
                        "Live market data is currently unavailable for {symbol} (tool: {}, error: {}). \
                         Do not invent numbers; tell the user the data could not be fetched.",
                        tool_call.name, err
                    );
                    conversation.add_tool_result(tool_call.name.clone(), message.clone());
                    on_chunk(format!("\n\n⚠️ {message}"));
                }
            }
        }
        
        // Loop continues - model will see tool results and respond
    }
    
    Err("Exceeded maximum conversation iterations".to_string())
}
