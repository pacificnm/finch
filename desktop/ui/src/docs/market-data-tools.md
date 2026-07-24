# Market Data & News Tools

These are the assistant's read-only lookup tools — they fetch real data and summarize it, without changing anything on screen.

## Current quote & fundamentals

Ask about the current price, volume, or fundamentals (PE ratio, EPS, market cap, beta, dividend yield, today's stats) and the assistant fetches a live quote. No parameters to specify — it always pulls the current quote for whatever symbol is loaded.

> "What's the PE ratio right now?"
> "How's it trading today?"

## Price history

Ask about price action, historical prices, or chart data for a specific window and the assistant fetches OHLC candle history.

| Parameter | Values | Default |
|---|---|---|
| `timeframe` | `1d`, `1w`, `1m`, `3m`, `6m`, `1y`, and similar | `1m` |
| `interval` | `1m`, `5m`, `15m`, `30m`, `1h`, `1d`, `1w`, `1mo` | `1d` |

> "Show me the last 3 months of daily price action."

## News headlines

Ask for news, recent events, press releases, or "what's going on with" the symbol and the assistant pulls recent headlines for it via Yahoo Finance's per-ticker feed.

| Parameter | Values | Default |
|---|---|---|
| `keyword` | Any text — filters headlines, e.g. "earnings", "guidance", "AI chip" | none (most recent headlines) |
| `limit` | 1–10 | 5 |

**Important limit:** this only returns headlines for the symbol currently loaded — it's not a general web search and can't look up other companies, unrelated topics, or broader market news. If it finds nothing, the assistant will tell you directly rather than guessing or retrying repeatedly.

> "Any recent news on this one?"
> "Anything about earnings?"
