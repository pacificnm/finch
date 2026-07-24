# Technical Analysis Tools

These tools compute real indicator math from live price history — nothing here is the model guessing at numbers.

## Trend analysis

Ask about performance, trend direction, momentum, or support/resistance levels.

| Parameter | Values | Default |
|---|---|---|
| `timeframe` | `1w`, `1m`, `3m`, `6m`, `1y`, and similar | `1m` |

Returns latest close, period high/low, percent change, and whether price is above or below the 20-day and 50-day simple moving averages.

## RSI (Relative Strength Index)

Ask about RSI, overbought/oversold conditions, or momentum.

| Parameter | Values | Default |
|---|---|---|
| `timeframe` | `1m`, `3m`, `6m`, `1y` | `3m` |
| `period` | Lookback in days | `14` |

## MACD

Ask about MACD, trend momentum, bullish/bearish crossovers, or reversal signals. Always computed with the standard 12/26/9 EMA setup.

| Parameter | Values | Default |
|---|---|---|
| `timeframe` | `3m`, `6m`, `1y` | `6m` — needs enough history for the 26-day EMA to stabilize |

## Volume analysis

Ask about trading volume, unusual activity, liquidity, or whether a price move is confirmed by volume.

| Parameter | Values | Default |
|---|---|---|
| `timeframe` | `1w`, `1m`, `3m`, `6m` | `1m` |

Returns latest volume vs. the period average, and whether that's unusually high, unusually low, or normal.

## ATR (Average True Range)

Ask about volatility, ATR, or how to size a stop loss based on the symbol's actual typical range instead of a flat percentage.

| Parameter | Values | Default |
|---|---|---|
| `timeframe` | `1m`, `3m`, `6m` | `3m` |
| `period` | Lookback in days | `14` |

## VWAP (Volume Weighted Average Price)

Ask about VWAP, or whether price is trading above/below today's volume-weighted average — the standard intraday reference traders use to judge whether the current price is rich or cheap relative to today's volume.

| Parameter | Values | Default |
|---|---|---|
| `timeframe` | How many trading days back, e.g. `1d` | `1d` — VWAP is canonically an intraday measure |

## Chart pattern detection

Ask the assistant to find, identify, or explain a chart pattern — "what pattern is this?", "is there a pattern here?" — and it runs rule-based geometric detection over swing highs and lows, **not** a black-box model. Every match comes with the exact points and lines that define it, and a fact-based explanation of why it qualifies (peak heights, retracement depth, trendline slope, etc.) — that explanation is what the assistant paraphrases into its answer.

| Parameter | Values | Default |
|---|---|---|
| `timeframe` | `1m`, `3m`, `6m`, `1y` | `3m` |
| `pattern_types` | One or more of: `double_top`, `double_bottom`, `head_and_shoulders`, `inverse_head_and_shoulders`, `ascending_triangle`, `descending_triangle`, `symmetrical_triangle` | all kinds checked |

A pattern search finding **nothing** is a completely normal, valid result — the assistant will say so plainly rather than force a match. See **Chart Overlays: Studies & Patterns** for what happens on the chart when a pattern *is* found.

> "Is there a double top forming?"
> "Check for any triangle patterns over the last 6 months."
