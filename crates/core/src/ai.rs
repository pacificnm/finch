//! AI stock assistant for the Trade screen's chat panel.
//!
//! Backed directly by `nest-ai-ollama` (no `nest-ai::AiService` indirection —
//! there's only ever one provider here, so the DI wrapper adds nothing).
//! Grounded with the real Schwab quote and recent price history for the
//! symbol in question, reusing `crate::schwab`'s existing pretty-printed
//! JSON accessors rather than re-fetching/re-parsing raw responses.
//!
//! Supports multi-turn conversations with context window management and
//! tool use (stock news for research, etc.).

use futures_util::StreamExt;
use nest_ai::{
    AiProvider, ChatMessage, ChatRole, CompletionChunk, CompletionRequest, ToolCall, ToolDefinition,
};
use nest_ai_ollama::{OllamaConfig, OllamaProvider};
use nest_http::HttpRequest;
use nest_http_client::HttpClientService;
use serde::Deserialize;
use serde_json::json;

use crate::chart_patterns;
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
         - Use calculate_macd when the user asks about MACD, trend momentum, bullish/bearish crossovers, or trend strength/reversal signals.\n\
         - Use analyze_volume when the user asks about trading volume, unusual activity, liquidity, or whether a price move is confirmed by volume.\n\
         - Use calculate_atr when the user asks about volatility, ATR, or how to size a stop loss based on the stock's normal range instead of a flat percentage.\n\
         - Use calculate_vwap when the user asks about VWAP or whether price is trading above/below the volume-weighted average today.\n\
         - Use calculate_trade_setup when the user asks for a buy price, stop loss, limit/target price, position size, or risk/reward math for a trade.\n\
         - Use set_chart_studies when the user asks you to show, hide, add, remove, or turn on/off a study or indicator overlay on the chart (Volume, moving average, RSI, MACD, ATR, VWAP).\n\
         - Use detect_chart_patterns when the user asks you to find, identify, spot, or explain a chart pattern (head and shoulders, double top/bottom, triangles), or asks things like 'what pattern is this' or 'is there a pattern here'.\n\
         - Use fetch_stock_news when the user asks for news, recent events, press releases, or 'what's going on with' {symbol}. It only returns headlines for {symbol} — it cannot search the general web or unrelated topics. Call it at most once per question; if it returns no headlines, tell the user immediately instead of retrying.\n\n\
         Response rules:\n\
         - Use the provided market data as ground truth\n\
         - Provide actionable output with specific numbers, prices, and math\n\
         - After receiving tool results, output the answer immediately. Do not describe that you called a tool, do not output slash commands, and do not add meta-commentary\n\
         - Do not include disclaimers, warnings, caveats, or statements about 'not being investment advice', 'past performance', 'risk tolerance', or 'consult a professional'\n\
         - Do not mention your knowledge cutoff, training data, or past dates such as 2023. The data you have is live.\n\
         - If a market-data tool fails, state briefly that live data is unavailable and do not invent numbers\n\
         - For trade setups, first output the clean formatted setup details, then ask if the user wants to populate the order ticket, then include the <TRADE_SETUP> JSON block on its own line\n\
         - The calculate_trade_setup tool returns JSON. Parse it and present the values in a clean human-readable format before asking to populate the order ticket\n\
         - For the \"Risk/reward\" line, copy the tool's risk_reward_display field verbatim (e.g. \"1:3.0\") instead of recomputing or reformatting it\n\
         - For chart study requests, call set_chart_studies, then briefly explain what the study shows and why it's relevant right now (this chat is also how the user is learning to read charts), then copy the tool's JSON result verbatim into a <CHART_STUDIES> block on its own line. Only include the studies that changed, exactly as the tool returned them — never recompute or reformat that JSON\n\
         - For chart pattern requests, call detect_chart_patterns, then explain in plain language why each pattern qualifies using the tool's precomputed points and note (never invent or recompute the numbers), then copy the tool's JSON result verbatim into a <CHART_PATTERNS> block on its own line. If no patterns are found, say so plainly and omit the block\n\
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
         <TRADE_SETUP>{{\"symbol\":\"AMD\",\"entry\":553.95,\"stop\":548.41,\"target\":570.57,\"shares\":18,\"risk\":99.72,\"reward\":299.16,\"position_size\":9971.10,\"risk_percent_of_account\":1.0,\"risk_reward_ratio\":3.0,\"risk_reward_display\":\"1:3.0\"}}</TRADE_SETUP>\n\n\
         Example for a chart study question:\n\
         User: can you show me the RSI on the chart?\n\
         Assistant: [uses set_chart_studies with {{\"rsi\":true}}]\n\
         Sure — RSI (Relative Strength Index) measures momentum on a 0-100 scale; readings above 70 suggest overbought, below 30 oversold. I've turned it on below the price chart so you can watch it alongside price action.\n\
         <CHART_STUDIES>{{\"rsi\":true}}</CHART_STUDIES>\n\n\
         Example for a chart pattern question:\n\
         User: is there a pattern forming on this chart?\n\
         Assistant: [uses detect_chart_patterns]\n\
         Yes — a double top. Two peaks near $182.40 (Jun 3) and $181.90 (Jun 21), within 0.3% of each other, separated by a pullback to $171.20 (Jun 12). Price closed below that $171.20 neckline on Jun 24, confirming the pattern — that's usually read as a bearish reversal signal after an uptrend.\n\
         <CHART_PATTERNS>{{\"patterns\":[{{\"kind\":\"double_top\",\"status\":\"confirmed\",\"label\":\"Double Top\",\"note\":\"Two peaks near $182.40 on 2026-06-03 and $181.90 on 2026-06-21 (within 0.3% of each other), separated by a pullback to $171.20 on 2026-06-12.\",\"points\":[{{\"date\":\"2026-06-03\",\"price\":182.40,\"role\":\"first_peak\",\"kind\":\"high\"}},{{\"date\":\"2026-06-12\",\"price\":171.20,\"role\":\"trough\",\"kind\":\"low\"}},{{\"date\":\"2026-06-21\",\"price\":181.90,\"role\":\"second_peak\",\"kind\":\"high\"}}],\"lines\":[{{\"role\":\"neckline\",\"from\":{{\"date\":\"2026-06-03\",\"price\":171.20}},\"to\":{{\"date\":\"2026-06-21\",\"price\":171.20}}}}]}}]}}</CHART_PATTERNS>"
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

/// Stock news tool definition, backed by Yahoo Finance's per-ticker RSS feed.
///
/// Earlier this used DuckDuckGo's Instant Answer API as a general web-search
/// stand-in, but that API almost never returns anything for ticker/news
/// queries (it's built for infobox-style facts, not search), which sent the
/// model into a loop of retrying reworded queries against a backend that was
/// never going to return results. Yahoo's RSS feed is ticker-scoped, free,
/// requires no API key, and returns real, current headlines — matching what
/// this assistant actually needs (news about the loaded symbol) rather than
/// unscoped web search.
fn fetch_stock_news_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "fetch_stock_news",
        format!("Fetch recent news headlines for {symbol}. \
         Use this when the user asks for news, recent events, press releases, or external research about {symbol}. \
         This only returns headlines for {symbol} — it cannot search the general web or other topics/companies."),
        json!({
            "type": "object",
            "properties": {
                "keyword": {
                    "type": "string",
                    "description": "Optional keyword to filter headlines (e.g. 'earnings', 'AI chip', 'guidance'). Omit to get the most recent headlines."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of headlines to return (default 5, max 10)",
                    "default": 5
                }
            },
            "required": []
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

/// Calculates MACD for the symbol.
fn calculate_macd_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "calculate_macd",
        format!("Calculate MACD (Moving Average Convergence Divergence) for {symbol} using the standard 12/26/9 EMA setup. \
         Use this when the user asks about MACD, trend momentum, bullish/bearish crossovers, or trend strength/reversal signals."),
        json!({
            "type": "object",
            "properties": {
                "timeframe": {
                    "type": "string",
                    "description": "Timeframe of daily candles to analyze, e.g. '3m', '6m', '1y'. MACD needs enough history for its 26-day EMA to stabilize.",
                    "default": "6m"
                }
            },
            "required": []
        }),
    )
}

/// Analyzes trading volume for the symbol.
fn analyze_volume_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "analyze_volume",
        format!("Analyze recent trading volume for {symbol} — average volume, latest volume vs. average, and whether volume is unusually high or low. \
         Use this when the user asks about volume, unusual activity, liquidity, or whether a price move is confirmed by volume."),
        json!({
            "type": "object",
            "properties": {
                "timeframe": {
                    "type": "string",
                    "description": "Timeframe to analyze, e.g. '1w', '1m', '3m', '6m'",
                    "default": "1m"
                }
            },
            "required": []
        }),
    )
}

/// Calculates ATR for the symbol.
fn calculate_atr_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "calculate_atr",
        format!("Calculate the Average True Range (ATR) for {symbol} — a volatility measure in price units, useful for sizing a stop loss based on how much {symbol} actually moves rather than a flat percentage. \
         Use this when the user asks about volatility, ATR, or how to size a stop loss based on the stock's normal daily range."),
        json!({
            "type": "object",
            "properties": {
                "timeframe": {
                    "type": "string",
                    "description": "Timeframe of daily candles to analyze, e.g. '1m', '3m', '6m'",
                    "default": "3m"
                },
                "period": {
                    "type": "integer",
                    "description": "ATR lookback period in days (default: 14)",
                    "default": 14
                }
            },
            "required": []
        }),
    )
}

/// Calculates VWAP for the symbol.
fn calculate_vwap_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "calculate_vwap",
        format!("Calculate the Volume Weighted Average Price (VWAP) for {symbol} — the standard intraday reference price traders use to judge whether the current price is rich or cheap relative to today's volume-weighted average. \
         Use this when the user asks about VWAP or wants to know if price is trading above/below the volume-weighted average today."),
        json!({
            "type": "object",
            "properties": {
                "timeframe": {
                    "type": "string",
                    "description": "How many trading days back to include, e.g. '1d' for just today. VWAP is canonically an intraday measure — '1d' is standard.",
                    "default": "1d"
                }
            },
            "required": []
        }),
    )
}

/// Shows or hides study overlays on the chart panel next to this chat.
fn set_chart_studies_tool_definition() -> ToolDefinition {
    ToolDefinition::new(
        "set_chart_studies",
        "Shows or hides study overlays on the price chart displayed next to this chat: a Volume histogram, \
         a 20-day moving average line, an RSI pane, a MACD pane (12/26/9), an ATR pane (14-day), and a VWAP line. \
         Use this when the user asks you to show, hide, add, remove, turn on/off, or plot a chart study or indicator overlay — \
         e.g. 'show me the RSI on the chart', 'turn off volume', 'add the moving average', 'plot MACD', 'show ATR', 'add VWAP'. \
         Only pass the studies that should change; omit ones the user didn't mention so they stay as they are.".to_string(),
        json!({
            "type": "object",
            "properties": {
                "volume": { "type": "boolean", "description": "Show (true) or hide (false) the volume histogram pane" },
                "moving_average": { "type": "boolean", "description": "Show (true) or hide (false) the 20-day simple moving average line" },
                "rsi": { "type": "boolean", "description": "Show (true) or hide (false) the RSI pane" },
                "macd": { "type": "boolean", "description": "Show (true) or hide (false) the MACD pane (line, signal, and histogram)" },
                "atr": { "type": "boolean", "description": "Show (true) or hide (false) the ATR (Average True Range) pane" },
                "vwap": { "type": "boolean", "description": "Show (true) or hide (false) the VWAP line overlaid on price" }
            },
            "required": []
        }),
    )
}

/// Detects classical chart patterns (head & shoulders, double top/bottom,
/// triangles) using deterministic swing-pivot geometry, and returns the
/// exact points/lines to draw for each match.
fn detect_chart_patterns_tool_definition(symbol: &str) -> ToolDefinition {
    ToolDefinition::new(
        "detect_chart_patterns",
        format!(
            "Detects classical chart patterns in {symbol}'s recent daily price history: double top, double bottom, \
             head and shoulders, inverse head and shoulders, and ascending/descending/symmetrical triangles. \
             Detection is rule-based geometry over swing highs/lows (not a black-box model) — every match includes the \
             exact points and lines that define it, and a fact-based note explaining why it qualifies. \
             Use this when the user asks you to find, identify, or explain a chart pattern, or asks what pattern is \
             forming. Returns an empty pattern list if nothing currently qualifies — that's a normal, valid result."
        ),
        json!({
            "type": "object",
            "properties": {
                "timeframe": {
                    "type": "string",
                    "description": "How much daily history to scan: 1m, 3m, 6m, 1y (default 3m). Patterns need enough history to form, so prefer 3m or more unless the user asks for a shorter window.",
                    "enum": ["1m", "3m", "6m", "1y"]
                },
                "pattern_types": {
                    "type": "array",
                    "description": "Optional filter to only look for specific pattern kinds. Omit to check all kinds.",
                    "items": {
                        "type": "string",
                        "enum": [
                            "double_top",
                            "double_bottom",
                            "head_and_shoulders",
                            "inverse_head_and_shoulders",
                            "ascending_triangle",
                            "descending_triangle",
                            "symmetrical_triangle"
                        ]
                    }
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

/// One headline from a Yahoo Finance per-ticker RSS feed.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct NewsItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default, rename = "pubDate")]
    pub_date: String,
}

#[derive(Debug, Deserialize)]
struct RssChannel {
    #[serde(rename = "item", default)]
    items: Vec<NewsItem>,
}

#[derive(Debug, Deserialize)]
struct Rss {
    channel: RssChannel,
}

/// Parses Yahoo Finance's per-ticker RSS feed into headline items.
fn parse_yahoo_finance_rss(xml: &str) -> Vec<NewsItem> {
    quick_xml::de::from_str::<Rss>(xml)
        .map(|rss| rss.channel.items)
        .unwrap_or_default()
}

/// Fetches recent headlines for `symbol` from Yahoo Finance's RSS feed,
/// optionally narrowed to headlines matching `keyword`. Falls back to the
/// unfiltered top headlines if a keyword filter matches nothing, rather than
/// returning an empty result the model would otherwise be tempted to retry.
async fn execute_fetch_stock_news(
    http_client: &HttpClientService,
    symbol: &str,
    keyword: Option<&str>,
    limit: Option<u32>,
) -> Result<String, String> {
    let limit = limit.unwrap_or(5).clamp(1, 10) as usize;
    let encoded_symbol = urlencoding::encode(symbol);
    let url = format!(
        "https://feeds.finance.yahoo.com/rss/2.0/headline?s={encoded_symbol}&region=US&lang=en-US"
    );

    let response = http_client
        .send(HttpRequest::get(&url))
        .await
        .map_err(|e| format!("Failed to fetch news for {symbol}: {e}"))?;
    let body = String::from_utf8_lossy(&response.body);

    let mut items = parse_yahoo_finance_rss(&body);

    if let Some(keyword) = keyword.map(str::trim).filter(|k| !k.is_empty()) {
        let needle = keyword.to_lowercase();
        let filtered: Vec<NewsItem> = items
            .iter()
            .filter(|item| {
                item.title.to_lowercase().contains(&needle)
                    || item.description.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        if !filtered.is_empty() {
            items = filtered;
        }
    }

    items.truncate(limit);

    if items.is_empty() {
        return Err(format!("No recent news headlines found for {symbol}."));
    }

    let formatted = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            format!(
                "{}. {} ({})\n   {}",
                i + 1,
                item.title,
                item.pub_date,
                item.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(format!("Recent news for {symbol}:\n\n{formatted}"))
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

/// Standard MACD fast/slow/signal periods.
const MACD_FAST_PERIOD: usize = 12;
const MACD_SLOW_PERIOD: usize = 26;
const MACD_SIGNAL_PERIOD: usize = 9;

/// Calculates an exponential moving average. Returns values aligned to
/// `values[period - 1..]` — the first output corresponds to the EMA seeded
/// by a simple average of the first `period` values, so the result is
/// `values.len() - period + 1` long (empty if there aren't enough values).
fn calculate_ema(values: &[f64], period: usize) -> Vec<f64> {
    if values.len() < period || period == 0 {
        return Vec::new();
    }

    let k = 2.0 / (period as f64 + 1.0);
    let seed = values[..period].iter().sum::<f64>() / period as f64;

    let mut ema = Vec::with_capacity(values.len() - period + 1);
    ema.push(seed);
    for &value in &values[period..] {
        let prev = *ema.last().expect("just pushed a seed value");
        ema.push(value * k + prev * (1.0 - k));
    }
    ema
}

/// Calculates the MACD line (fast EMA minus slow EMA), aligned to the slow
/// EMA's range (the shorter of the two series).
fn calculate_macd_line(closes: &[f64], fast: usize, slow: usize) -> Vec<f64> {
    let ema_fast = calculate_ema(closes, fast);
    let ema_slow = calculate_ema(closes, slow);
    if ema_slow.is_empty() || slow < fast || ema_fast.len() < slow - fast {
        return Vec::new();
    }

    let offset = slow - fast;
    ema_fast[offset..]
        .iter()
        .zip(ema_slow.iter())
        .map(|(fast_ema, slow_ema)| fast_ema - slow_ema)
        .collect()
}

/// Calculates MACD from daily closing prices using the standard 12/26/9 setup.
async fn execute_calculate_macd(symbol: &str, timeframe: Option<&str>) -> Result<String, String> {
    let (period_type, period, frequency_type, frequency) =
        map_timeframe_args(timeframe.or(Some("6m")), Some("1d"));

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

    let closes: Vec<f64> = history
        .get("candles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|c| c.get("close").and_then(|v| v.as_f64()))
        .collect();

    let min_required = MACD_SLOW_PERIOD + MACD_SIGNAL_PERIOD;
    if closes.len() < min_required {
        return Ok(format!(
            "Not enough price history to calculate MACD (need at least {min_required} daily closes, have {})."
            , closes.len()
        ));
    }

    let macd_line = calculate_macd_line(&closes, MACD_FAST_PERIOD, MACD_SLOW_PERIOD);
    let signal_line = calculate_ema(&macd_line, MACD_SIGNAL_PERIOD);

    if signal_line.is_empty() {
        return Ok("Not enough price history to calculate MACD's signal line.".to_string());
    }

    // Histogram values aligned to the signal line's range.
    let hist_offset = macd_line.len() - signal_line.len();
    let histogram: Vec<f64> = macd_line[hist_offset..]
        .iter()
        .zip(signal_line.iter())
        .map(|(macd, signal)| macd - signal)
        .collect();

    let current_macd = *macd_line.last().expect("checked non-empty via signal_line");
    let current_signal = *signal_line.last().expect("checked non-empty above");
    let current_hist = *histogram.last().expect("same length as signal_line");
    let previous_hist = histogram.len().checked_sub(2).map(|i| histogram[i]);

    let momentum = if current_hist > 0.0 { "bullish (MACD above signal)" } else { "bearish (MACD below signal)" };

    let crossover = match previous_hist {
        Some(prev) if prev <= 0.0 && current_hist > 0.0 => "Bullish crossover just occurred (MACD crossed above signal).",
        Some(prev) if prev >= 0.0 && current_hist < 0.0 => "Bearish crossover just occurred (MACD crossed below signal).",
        Some(_) => "No crossover on the most recent bar.",
        None => "Not enough history to detect a crossover.",
    };

    let timeframe_label = timeframe.unwrap_or("6m");

    Ok(format!(
        "MACD analysis for {symbol} ({timeframe_label} daily, 12/26/9):\n\
         - MACD line: {current_macd:.4}\n\
         - Signal line: {current_signal:.4}\n\
         - Histogram: {current_hist:.4}\n\
         - Momentum: {momentum}\n\
         - {crossover}"
    ))
}

/// Analyzes recent trading volume from daily candles.
async fn execute_analyze_volume(symbol: &str, timeframe: Option<&str>) -> Result<String, String> {
    let (period_type, period, frequency_type, frequency) =
        map_timeframe_args(timeframe.or(Some("1m")), Some("1d"));

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

    let volumes: Vec<u64> = history
        .get("candles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|c| c.get("volume").and_then(|v| v.as_u64()))
        .collect();

    if volumes.is_empty() {
        return Ok(format!("No volume data available for {symbol} in the requested timeframe."));
    }

    let latest = *volumes.last().expect("checked non-empty above");
    let average = volumes.iter().sum::<u64>() as f64 / volumes.len() as f64;
    let relative_pct = if average > 0.0 { (latest as f64 / average) * 100.0 } else { 0.0 };

    let recent_n = volumes.len().min(5);
    let recent_average =
        volumes.iter().rev().take(recent_n).sum::<u64>() as f64 / recent_n as f64;
    let trend = if recent_average > average * 1.1 {
        "increasing"
    } else if recent_average < average * 0.9 {
        "decreasing"
    } else {
        "steady"
    };

    let activity = if relative_pct >= 150.0 {
        "unusually high"
    } else if relative_pct <= 50.0 {
        "unusually low"
    } else {
        "normal"
    };

    let timeframe_label = timeframe.unwrap_or("1m");

    Ok(format!(
        "Volume analysis for {symbol} ({timeframe_label}):\n\
         - Latest session volume: {latest}\n\
         - Average volume over period: {average:.0}\n\
         - Latest vs. average: {relative_pct:.0}% ({activity})\n\
         - Recent volume trend: {trend}"
    ))
}

/// Computes the true range for each candle. The first candle has no prior
/// close, so it falls back to a plain high-low range — a standard, minor
/// simplification that doesn't meaningfully bias a multi-day ATR.
fn calculate_true_ranges(candles: &[serde_json::Value]) -> Vec<f64> {
    let mut true_ranges = Vec::new();
    let mut prev_close: Option<f64> = None;
    for candle in candles {
        let high = candle.get("high").and_then(|v| v.as_f64());
        let low = candle.get("low").and_then(|v| v.as_f64());
        let close = candle.get("close").and_then(|v| v.as_f64());
        if let (Some(high), Some(low), Some(close)) = (high, low, close) {
            let tr = match prev_close {
                Some(prev) => (high - low).max((high - prev).abs()).max((low - prev).abs()),
                None => high - low,
            };
            true_ranges.push(tr);
            prev_close = Some(close);
        }
    }
    true_ranges
}

/// Wilder's smoothing (the standard method behind ATR and RSI-style
/// indicators): seeds with a simple average of the first `period` values,
/// then applies `(prev * (period - 1) + value) / period` for each
/// subsequent value.
fn wilder_smooth(values: &[f64], period: usize) -> Vec<f64> {
    if values.len() < period || period == 0 {
        return Vec::new();
    }
    let mut smoothed = Vec::with_capacity(values.len() - period + 1);
    let seed = values[..period].iter().sum::<f64>() / period as f64;
    smoothed.push(seed);
    for &value in &values[period..] {
        let prev = *smoothed.last().expect("just pushed a seed value");
        smoothed.push((prev * (period as f64 - 1.0) + value) / period as f64);
    }
    smoothed
}

/// Calculates ATR (Average True Range) from daily candles using Wilder smoothing.
async fn execute_calculate_atr(
    symbol: &str,
    timeframe: Option<&str>,
    period: Option<u64>,
) -> Result<String, String> {
    let period = period.unwrap_or(14) as usize;
    let (period_type, period_str, frequency_type, frequency) =
        map_timeframe_args(timeframe.or(Some("3m")), Some("1d"));

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
    let candles = history
        .get("candles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let true_ranges = calculate_true_ranges(&candles);
    let atr_values = wilder_smooth(&true_ranges, period);
    let Some(&current_atr) = atr_values.last() else {
        return Ok(format!(
            "Not enough price history to calculate ATR (need > {period} days)."
        ));
    };

    let last_close = candles
        .last()
        .and_then(|c| c.get("close"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let atr_pct = if last_close > 0.0 { (current_atr / last_close) * 100.0 } else { 0.0 };
    let timeframe_label = timeframe.unwrap_or("3m");

    Ok(format!(
        "ATR analysis for {symbol} ({timeframe_label}, {period}-day):\n\
         - Current ATR: ${current_atr:.2}\n\
         - ATR as % of price: {atr_pct:.2}%\n\
         - Typical daily range: about ${current_atr:.2}"
    ))
}

/// Calculates the volume-weighted average price across the given candles.
fn calculate_vwap_value(candles: &[serde_json::Value]) -> Option<f64> {
    let mut cumulative_pv = 0.0;
    let mut cumulative_volume = 0.0;
    for candle in candles {
        let high = candle.get("high").and_then(|v| v.as_f64());
        let low = candle.get("low").and_then(|v| v.as_f64());
        let close = candle.get("close").and_then(|v| v.as_f64());
        let volume = candle.get("volume").and_then(|v| v.as_f64());
        if let (Some(high), Some(low), Some(close), Some(volume)) = (high, low, close, volume) {
            let typical_price = (high + low + close) / 3.0;
            cumulative_pv += typical_price * volume;
            cumulative_volume += volume;
        }
    }
    (cumulative_volume > 0.0).then_some(cumulative_pv / cumulative_volume)
}

/// Calculates VWAP, defaulting to today's session at 5-minute resolution
/// (the canonical intraday use of VWAP).
async fn execute_calculate_vwap(symbol: &str, timeframe: Option<&str>) -> Result<String, String> {
    let (period_type, period, frequency_type, frequency) =
        map_timeframe_args(timeframe.or(Some("1d")), Some("5m"));

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

    let Some(vwap) = calculate_vwap_value(&candles) else {
        return Ok(format!("Not enough price/volume history to calculate VWAP for {symbol}."));
    };

    let last_close = candles.last().and_then(|c| c.get("close")).and_then(|v| v.as_f64());
    let timeframe_label = timeframe.unwrap_or("1d");

    let position = match last_close {
        Some(close) if close > vwap => format!(
            "Price is trading ABOVE VWAP (${close:.2} vs ${vwap:.2}) — premium territory intraday."
        ),
        Some(close) if close < vwap => format!(
            "Price is trading BELOW VWAP (${close:.2} vs ${vwap:.2}) — discount territory intraday."
        ),
        Some(close) => format!("Price is trading right at VWAP (${close:.2})."),
        None => "Could not determine the current price relative to VWAP.".to_string(),
    };

    Ok(format!(
        "VWAP analysis for {symbol} ({timeframe_label}):\n\
         - VWAP: ${vwap:.2}\n\
         - {position}"
    ))
}

/// Detects chart patterns over recent daily candles and returns them as a
/// compact JSON payload the model is instructed to echo verbatim into a
/// `<CHART_PATTERNS>` tag (see `chart_patterns` module for the detection
/// logic itself).
async fn execute_detect_chart_patterns(
    symbol: &str,
    timeframe: Option<&str>,
    pattern_types: Option<Vec<&str>>,
) -> Result<String, String> {
    let (period_type, period, frequency_type, frequency) =
        map_timeframe_args(timeframe.or(Some("3m")), Some("1d"));

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
    let raw_candles = history
        .get("candles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if raw_candles.len() < 10 {
        return Ok(build_chart_patterns_result(Vec::new()));
    }

    let candles: Vec<chart_patterns::Candle> = raw_candles
        .iter()
        .filter_map(|c| {
            let high = c.get("high").and_then(|v| v.as_f64())?;
            let low = c.get("low").and_then(|v| v.as_f64())?;
            let close = c.get("close").and_then(|v| v.as_f64())?;
            let ms = c
                .get("datetime")
                .and_then(|v| v.as_i64())
                .or_else(|| c.get("date").and_then(|v| v.as_i64()))?;
            let date = chrono::DateTime::from_timestamp_millis(ms)?
                .format("%Y-%m-%d")
                .to_string();
            Some(chart_patterns::Candle { date, high, low, close })
        })
        .collect();

    let true_ranges = calculate_true_ranges(&raw_candles);
    let atr = wilder_smooth(&true_ranges, 14).last().copied();

    let patterns = chart_patterns::detect_patterns(&candles, atr, pattern_types.as_deref());
    Ok(build_chart_patterns_result(patterns))
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

    // Pre-formatted as "1:X" (risk normalized to 1) so the model echoes it
    // verbatim instead of re-deriving/reformatting the ratio itself — local
    // models are unreliable at keeping "risk:reward" in the right order.
    let risk_reward_display = format!("1:{:.1}", round2(risk_reward_ratio));

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
        "risk_reward_display": risk_reward_display,
    });

    // Return a compact JSON payload for the model. The model is responsible for
    // formatting the human-readable summary and appending the <TRADE_SETUP> block.
    Ok(setup_json.to_string())
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Builds the `detect_chart_patterns` result: the detected patterns as
/// compact JSON, or an explicit empty list. Unlike `set_chart_studies`,
/// finding zero patterns is a valid, common answer, not an error.
fn build_chart_patterns_result(patterns: Vec<chart_patterns::PatternMatch>) -> String {
    json!({ "patterns": patterns }).to_string()
}

/// Builds the `set_chart_studies` result: a compact JSON object of only the
/// studies the caller specified. Purely a UI directive — no external data,
/// so no async I/O — the frontend applies it by parsing the `<CHART_STUDIES>`
/// tag the model is instructed to echo this JSON into verbatim, merging it
/// into whatever studies are already on (see the system prompt rule).
fn build_chart_studies_result(
    volume: Option<bool>,
    moving_average: Option<bool>,
    rsi: Option<bool>,
    macd: Option<bool>,
    atr: Option<bool>,
    vwap: Option<bool>,
) -> Result<String, String> {
    if volume.is_none()
        && moving_average.is_none()
        && rsi.is_none()
        && macd.is_none()
        && atr.is_none()
        && vwap.is_none()
    {
        return Err(
            "No study changes specified — pass at least one of volume, moving_average, rsi, macd, atr, or vwap."
                .to_string(),
        );
    }

    let mut changes = serde_json::Map::new();
    if let Some(value) = volume {
        changes.insert("volume".to_string(), json!(value));
    }
    if let Some(value) = moving_average {
        changes.insert("moving_average".to_string(), json!(value));
    }
    if let Some(value) = rsi {
        changes.insert("rsi".to_string(), json!(value));
    }
    if let Some(value) = macd {
        changes.insert("macd".to_string(), json!(value));
    }
    if let Some(value) = atr {
        changes.insert("atr".to_string(), json!(value));
    }
    if let Some(value) = vwap {
        changes.insert("vwap".to_string(), json!(value));
    }

    Ok(serde_json::Value::Object(changes).to_string())
}

/// Executes a tool call and returns the result.
async fn execute_tool_call(
    tool_call: &ToolCall,
    symbol: &str,
    http_client: &HttpClientService,
) -> Result<String, String> {
    match tool_call.name.as_str() {
        "fetch_stock_news" => {
            let keyword = tool_call.arguments.get("keyword").and_then(|v| v.as_str());
            let limit = tool_call.arguments.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32);

            execute_fetch_stock_news(http_client, symbol, keyword, limit).await
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
        "calculate_macd" => {
            let timeframe = tool_call.arguments.get("timeframe").and_then(|v| v.as_str());
            execute_calculate_macd(symbol, timeframe).await
        }
        "analyze_volume" => {
            let timeframe = tool_call.arguments.get("timeframe").and_then(|v| v.as_str());
            execute_analyze_volume(symbol, timeframe).await
        }
        "calculate_atr" => {
            let timeframe = tool_call.arguments.get("timeframe").and_then(|v| v.as_str());
            let period = tool_call.arguments.get("period").and_then(|v| v.as_u64());
            execute_calculate_atr(symbol, timeframe, period).await
        }
        "calculate_vwap" => {
            let timeframe = tool_call.arguments.get("timeframe").and_then(|v| v.as_str());
            execute_calculate_vwap(symbol, timeframe).await
        }
        "set_chart_studies" => {
            let volume = tool_call.arguments.get("volume").and_then(|v| v.as_bool());
            let moving_average = tool_call.arguments.get("moving_average").and_then(|v| v.as_bool());
            let rsi = tool_call.arguments.get("rsi").and_then(|v| v.as_bool());
            let macd = tool_call.arguments.get("macd").and_then(|v| v.as_bool());
            let atr = tool_call.arguments.get("atr").and_then(|v| v.as_bool());
            let vwap = tool_call.arguments.get("vwap").and_then(|v| v.as_bool());
            build_chart_studies_result(volume, moving_average, rsi, macd, atr, vwap)
        }
        "detect_chart_patterns" => {
            let timeframe = tool_call.arguments.get("timeframe").and_then(|v| v.as_str());
            let pattern_types: Option<Vec<&str>> = tool_call
                .arguments
                .get("pattern_types")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());
            execute_detect_chart_patterns(symbol, timeframe, pattern_types).await
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
        fetch_stock_news_tool_definition(symbol),
        fetch_quote_details_tool_definition(symbol),
        fetch_candlestick_data_tool_definition(symbol),
        analyze_price_trend_tool_definition(symbol),
        calculate_rsi_tool_definition(symbol),
        calculate_macd_tool_definition(symbol),
        analyze_volume_tool_definition(symbol),
        calculate_atr_tool_definition(symbol),
        calculate_vwap_tool_definition(symbol),
        set_chart_studies_tool_definition(),
        detect_chart_patterns_tool_definition(symbol),
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
        // Tracks whether we're currently mid-"thinking" output, so the UI
        // gets a header when reasoning starts and a divider when it ends —
        // without either, a `think`-enabled model goes silent for however
        // long it reasons before emitting real content or a tool call,
        // which reads as a hang rather than a still-streaming response.
        let mut in_thinking = false;

        // Stream the response
        while let Some(chunk_result) = stream.next().await {
            let chunk: CompletionChunk = chunk_result.map_err(|err| err.to_string())?;

            // Reasoning text is shown live but never persisted to
            // conversation history — it's scratch work, not the answer, and
            // feeding it back to the model next turn would waste context.
            if !chunk.thinking_delta.is_empty() {
                if !in_thinking {
                    on_chunk("🤔 _Thinking…_\n\n".to_string());
                    in_thinking = true;
                }
                on_chunk(chunk.thinking_delta);
            }

            // Accumulate text chunks
            if !chunk.content_delta.is_empty() {
                if in_thinking {
                    on_chunk("\n\n---\n\n".to_string());
                    in_thinking = false;
                }
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
                "fetch_stock_news" => "Fetching news",
                "fetch_quote_details" => "Fetching quote details",
                "fetch_candlestick_data" => "Fetching candlestick data",
                "analyze_price_trend" => "Analyzing price trend",
                "calculate_rsi" => "Calculating RSI",
                "calculate_macd" => "Calculating MACD",
                "analyze_volume" => "Analyzing volume",
                "calculate_atr" => "Calculating ATR",
                "calculate_vwap" => "Calculating VWAP",
                "set_chart_studies" => "Updating chart",
                "detect_chart_patterns" => "Detecting chart patterns",
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
                    let message = if tool_call.name == "fetch_stock_news" {
                        format!(
                            "No news headlines are available for {symbol} right now (error: {err}). \
                             Tell the user no recent news was found; do not call fetch_stock_news again for this question."
                        )
                    } else {
                        format!(
                            "Live market data is currently unavailable for {symbol} (tool: {}, error: {}). \
                             Do not invent numbers; tell the user the data could not be fetched.",
                            tool_call.name, err
                        )
                    };
                    conversation.add_tool_result(tool_call.name.clone(), message.clone());
                    on_chunk(format!("\n\n⚠️ {message}"));
                }
            }
        }
        
        // Loop continues - model will see tool results and respond
    }
    
    Err("Exceeded maximum conversation iterations".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed fixture based on a real response from
    // https://feeds.finance.yahoo.com/rss/2.0/headline?s=INTC — includes an
    // XML entity in a title (`&amp;`) and a `<guid>` element with an
    // attribute that `NewsItem` doesn't model, to prove both are handled.
    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<rss version="2.0">
    <channel>
        <description>Latest Financial News for INTC</description>
        <item>
            <description>The Nasdaq undercut key levels while Intel jumped late.</description>
            <guid isPermaLink="false">96fdd7c6-3e7b-334a-a08e-b91b60ced4b6</guid>
            <link>https://finance.yahoo.com/m/example.html?.tsrc=rss</link>
            <pubDate>Fri, 24 Jul 2026 02:07:45 +0000</pubDate>
            <title>Dow Jones Futures: Market Sells Off; Intel Jumps Late</title>
        </item>
        <item>
            <description>Markets focused on earnings amid Nasdaq &amp; S&amp;P moves.</description>
            <guid isPermaLink="false">78fb0fca-f68b-3e78-ab5b-1f4b39e24986</guid>
            <link>https://example.com/article</link>
            <pubDate>Fri, 24 Jul 2026 01:46:05 +0000</pubDate>
            <title>Nasdaq &amp; S&amp;P 500 Futures Shake Off Jitters: INTC In Focus</title>
        </item>
        <item>
            <description>Intel reported strong AI-driven sales growth this quarter.</description>
            <guid isPermaLink="false">047a75ed-3dde-3ec6-9863-680ce6759bb1</guid>
            <link>https://example.com/earnings</link>
            <pubDate>Thu, 23 Jul 2026 23:09:00 +0000</pubDate>
            <title>Intel Earnings: AI Driven Demand Leads to Decade High Sales Growth</title>
        </item>
        <language>en-US</language>
        <lastBuildDate>Fri, 24 Jul 2026 02:12:46 +0000</lastBuildDate>
        <title>Yahoo! Finance: INTC News</title>
    </channel>
</rss>"#;

    #[test]
    fn parse_yahoo_finance_rss_extracts_items_and_unescapes_entities() {
        let items = parse_yahoo_finance_rss(SAMPLE_RSS);

        assert_eq!(items.len(), 3);
        assert_eq!(
            items[0].title,
            "Dow Jones Futures: Market Sells Off; Intel Jumps Late"
        );
        assert_eq!(items[0].pub_date, "Fri, 24 Jul 2026 02:07:45 +0000");
        assert_eq!(
            items[1].title,
            "Nasdaq & S&P 500 Futures Shake Off Jitters: INTC In Focus"
        );
        assert!(items[1].description.contains("Nasdaq & S&P moves"));
    }

    #[test]
    fn parse_yahoo_finance_rss_returns_empty_for_malformed_input() {
        assert!(parse_yahoo_finance_rss("not xml at all").is_empty());
        assert!(parse_yahoo_finance_rss("<rss><channel></channel></rss>").is_empty());
    }

    #[test]
    fn keyword_filter_matches_case_insensitively_by_title() {
        let items = parse_yahoo_finance_rss(SAMPLE_RSS);

        let matching: Vec<&NewsItem> = items
            .iter()
            .filter(|item| item.title.to_lowercase().contains("earnings"))
            .collect();
        assert_eq!(matching.len(), 1);
        assert!(matching[0].title.contains("Intel Earnings"));

        let no_match: Vec<&NewsItem> = items
            .iter()
            .filter(|item| item.title.to_lowercase().contains("nonexistent-topic-xyz"))
            .collect();
        assert!(no_match.is_empty());
    }

    #[test]
    fn calculate_ema_matches_hand_worked_values() {
        // period=3, k=0.5: seed = mean(1,2,3) = 2.0, then each step is
        // value*0.5 + prev*0.5.
        let ema = calculate_ema(&[1.0, 2.0, 3.0, 4.0, 5.0], 3);
        assert_eq!(ema, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn calculate_ema_returns_empty_when_not_enough_values() {
        assert!(calculate_ema(&[1.0, 2.0], 3).is_empty());
        assert!(calculate_ema(&[1.0, 2.0, 3.0], 0).is_empty());
    }

    #[test]
    fn calculate_macd_line_matches_hand_worked_values() {
        // fast=2 (k=2/3), slow=3 (k=0.5), closes ramp linearly by 1 each bar.
        // ema_fast = [1.5, 2.5, 3.5, 4.5, 5.5] aligned to closes[1..]
        // ema_slow = [2.0, 3.0, 4.0, 5.0]      aligned to closes[2..]
        // macd = ema_fast[1..] - ema_slow = [0.5, 0.5, 0.5, 0.5]
        let closes = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let macd = calculate_macd_line(&closes, 2, 3);
        assert_eq!(macd.len(), 4, "expected 4 aligned MACD values, got {macd:?}");
        for value in macd {
            assert!((value - 0.5).abs() < 1e-9, "expected ~0.5, got {value}");
        }
    }

    #[test]
    fn calculate_macd_line_returns_empty_when_not_enough_values() {
        assert!(calculate_macd_line(&[1.0, 2.0], 12, 26).is_empty());
    }

    fn candle(high: f64, low: f64, close: f64, volume: f64) -> serde_json::Value {
        json!({ "high": high, "low": low, "close": close, "volume": volume })
    }

    #[test]
    fn calculate_true_ranges_matches_hand_worked_values() {
        // Bar 1 has no prior close: TR = high - low = 2.
        // Bar 2: max(11-9, |11-9|, |9-9|) = 2.
        // Bar 3: max(9-6, |9-10|, |6-10|) = max(3, 1, 4) = 4.
        let candles = vec![
            candle(10.0, 8.0, 9.0, 0.0),
            candle(11.0, 9.0, 10.0, 0.0),
            candle(9.0, 6.0, 7.0, 0.0),
        ];
        let true_ranges = calculate_true_ranges(&candles);
        assert_eq!(true_ranges, vec![2.0, 2.0, 4.0]);
    }

    #[test]
    fn wilder_smooth_matches_hand_worked_values() {
        // period=3: seed = mean(1,2,3) = 2.0
        // next: (2.0*2 + 4) / 3 = 2.6667
        // next: (2.6667*2 + 5) / 3 = 3.4444
        let smoothed = wilder_smooth(&[1.0, 2.0, 3.0, 4.0, 5.0], 3);
        assert_eq!(smoothed.len(), 3);
        assert!((smoothed[0] - 2.0).abs() < 1e-9);
        assert!((smoothed[1] - 2.6667).abs() < 1e-3);
        assert!((smoothed[2] - 3.4444).abs() < 1e-3);
    }

    #[test]
    fn wilder_smooth_returns_empty_when_not_enough_values() {
        assert!(wilder_smooth(&[1.0, 2.0], 3).is_empty());
    }

    #[test]
    fn calculate_vwap_value_matches_hand_worked_values() {
        // Bar 1: typical=(10+8+9)/3=9.0, pv=900, volume=100
        // Bar 2: typical=(12+10+11)/3=11.0, pv=2200, volume=200
        // vwap = (900+2200)/(100+200) = 3100/300 = 10.3333
        let candles = vec![
            candle(10.0, 8.0, 9.0, 100.0),
            candle(12.0, 10.0, 11.0, 200.0),
        ];
        let vwap = calculate_vwap_value(&candles).unwrap();
        assert!((vwap - 10.3333).abs() < 1e-3);
    }

    #[test]
    fn calculate_vwap_value_returns_none_for_zero_volume() {
        let candles = vec![candle(10.0, 8.0, 9.0, 0.0)];
        assert!(calculate_vwap_value(&candles).is_none());
    }

    #[test]
    fn build_chart_studies_result_includes_only_specified_studies() {
        let result =
            build_chart_studies_result(Some(true), None, Some(false), None, None, None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["volume"], true);
        assert_eq!(parsed["rsi"], false);
        assert!(parsed.get("moving_average").is_none());
        assert!(parsed.get("macd").is_none());
        assert!(parsed.get("atr").is_none());
        assert!(parsed.get("vwap").is_none());
    }

    #[test]
    fn build_chart_studies_result_includes_macd() {
        let result = build_chart_studies_result(None, None, None, Some(true), None, None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["macd"], true);
    }

    #[test]
    fn build_chart_studies_result_includes_atr_and_vwap() {
        let result =
            build_chart_studies_result(None, None, None, None, Some(true), Some(false)).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["atr"], true);
        assert_eq!(parsed["vwap"], false);
    }

    #[test]
    fn build_chart_studies_result_errors_when_nothing_specified() {
        assert!(build_chart_studies_result(None, None, None, None, None, None).is_err());
    }

    #[test]
    fn build_chart_patterns_result_serializes_an_empty_list() {
        let result = build_chart_patterns_result(Vec::new());
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["patterns"], json!([]));
    }

    #[test]
    fn build_chart_patterns_result_serializes_detected_patterns() {
        let patterns = vec![chart_patterns::PatternMatch {
            kind: "double_top",
            status: "confirmed",
            label: "Double Top".to_string(),
            note: "Two peaks near $100.00...".to_string(),
            points: vec![],
            lines: vec![],
        }];
        let result = build_chart_patterns_result(patterns);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["patterns"][0]["kind"], "double_top");
        assert_eq!(parsed["patterns"][0]["status"], "confirmed");
    }
}
