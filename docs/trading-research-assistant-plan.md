# Trading Research Assistant — Project Plan

## Overview
A locally-run research assistant to help with trend analysis and market research. It does **not** place or recommend trades autonomously — a human always evaluates and executes trades. The tool's job is to surface analysis, news, and sentiment so the human can make faster, better-informed decisions.

Core capabilities:
- Technical trend analysis (price, volume, indicators)
- Current news retrieval and summarization for a given company/ticker
- Social sentiment tracking around a company
- Historical context via semantic search over past articles

## Local LLM / Inference

**Hardware:** Tesla P40 (24GB VRAM, Pascal architecture)
- No usable FP16/tensor cores — must use quantized (Q4/Q5 GGUF) models via Ollama/llama.cpp
- ~346 GB/s memory bandwidth is the main bottleneck (token generation is bandwidth-bound)
- Sweet spot: 14B–27B models. 70B dense models are impractically slow on a single card (sub-1 tok/s).

**Model recommendation:**
- Primary: **Qwen3 14B** (Q4_K_M) — strong reasoning, good tool/function-calling support (needed for the agent loop), ~16 tok/s
- Also worth benchmarking: **Llama 4 Scout 17B** (MoE, activates fewer params/token — can be competitive on bandwidth-limited hardware)
- Consider **Qwen3.5 27B** if quality > speed for a given task, but expect single-digit tok/s
- Embeddings: **nomic-embed-text** or **mxbai-embed-large** via Ollama (cheap, fast even on P40)

## Agent Architecture

- Model orchestrates tool calls; deterministic computation (price data, indicators) stays in code, not the LLM
- Preferred pattern: **fixed pipeline** (fetch → analyze → summarize) rather than open-ended agentic tool selection — more reliable on a 14B local model
- LLM's job: synthesize/summarize retrieved data (news, sentiment, indicator readouts) into a coherent narrative for the human to review
- Human always reviews before any trade action — no autonomous execution

### Tools to build
- **Market data fetch** — OHLCV history for a ticker
- **Indicator calc** — RSI, MACD, moving averages, etc. (ta-lib or similar), exposed as a callable tool
- **News fetch** — search + article retrieval scoped to ticker/company (NewsAPI, RSS, or scraper)
- **Social sentiment fetch** — Reddit (PRAW) and/or X API, pre-filtered/pre-scored before hitting LLM context
- **Filings/fundamentals** — SEC EDGAR (10-K, 8-K) for anything beyond price action
- **Vector search** — semantic retrieval over historical articles (see below)
- **State/memory store** — track what's already been analyzed per ticker across runs

Design notes:
- Keep tools narrow/single-purpose (`get_price_history`, `get_news`, `get_indicator`) rather than one big catch-all tool — smaller models route calls more reliably that way
- Pre-fetch top-k relevant historical articles + current news and hand both to the model in one pass, rather than trusting the model to decide when to query the vector store live

## Database — PostgreSQL

### Articles / News (pgvector)
- Table: `articles` — raw text, source, ticker(s), published_at, sentiment_score, `embedding vector(N)`
- HNSW index for similarity search
- Hybrid search: pair vector embedding with a `tsvector` full-text column (ticker symbols/specific terms don't always embed distinctively)
- Use for: historical pattern matching ("similar news in the past"), dedup of republished wire stories, cross-ticker/sector relevance
- Optional future step: link articles to the price move that followed, building a labeled "did this news move the stock" dataset

### Price data
- Plain Postgres to start: `ohlcv` table `(ticker, timestamp, open, high, low, close, volume)`, composite index on `(ticker, timestamp)`
- Separate tables/granularities: `ohlcv_daily`, `ohlcv_intraday` rather than a single table with a timeframe column
- Store raw data as-fetched; compute indicators on read or via materialized view (keeps source data clean, indicator logic changeable without backfill)
- Upgrade path if needed: **TimescaleDB** (auto-partitioning hypertables, continuous aggregates) — worth it for intraday/tick-level data across many tickers, overkill for a modest daily-bar watchlist

## Frontend

**Stack:** Tauri + React

**Charting library:** TradingView **Lightweight Charts** (Apache-2.0, canvas-based, fast)
- Candlestick + volume series, multi-pane layout for RSI/other indicators synced on the same time axis
- Indicators computed in the backend/agent layer, rendered as line series in their own pane — library doesn't compute, only renders
- Alternative to evaluate: `klinecharts` (more built-in indicators out of the box, less polished docs)
- Avoided: react-financial-charts (maintenance concerns), general-purpose chart libs (ApexCharts/Chart.js/Recharts — not built for dense multi-pane financial views)

## Claude.ai ↔ Claude Code

- No direct sync between a claude.ai Project and Claude Code — separate surfaces, no shared context
- Bridge: maintain a `CLAUDE.md` (read automatically by Claude Code) in the repo with architecture/conventions, and hand off planning docs like this one directly
- GitHub integration was set up in the claude.ai Project via "Add from GitHub" (project knowledge, one-directional read, manual "Sync now" to refresh) — separate from any GitHub MCP connector used by Claude Code

## Open / Next Decisions
- Finalize tool schemas for the agent (market data, news, sentiment, vector search)
- Decide on news/social data sources and their rate limits/costs
- Confirm frontend framework details (referred to as "Nest framework" — needs clarification: Tauri + React, but worth confirming if any additional framework layer, e.g. NestJS backend, is intended)
- Decide Postgres vs. TimescaleDB based on actual data volume once scraping starts
- Build out `CLAUDE.md` for the repo once implementation begins in Claude Code
