# Overview

Finch includes an AI trading assistant you can chat with directly next to the chart. It answers questions about the symbol you currently have loaded, using live market data, and can take a few actions on your behalf — like toggling chart studies or drawing detected chart patterns — but it never places or modifies a real order without you explicitly clicking to do so.

## Where to find it

The assistant lives in the right-hand panel on two screens:

- **Trade** — chat alongside the order ticket and quote details.
- **Charts** — chat alongside the full-size candlestick chart.

It does **not** appear on the Positions or Scans screens.

## It's scoped to whatever symbol is loaded

Whichever symbol is currently loaded in the Trade or Charts screen is what the assistant is talking about — there's no need to mention the ticker in every message once a conversation is going. Each symbol keeps its **own separate chat history**, saved automatically, so switching from AAPL to MSFT and back doesn't lose either conversation. Use the trash icon at the top of the chat panel to clear the history for the symbol you're currently viewing.

## Reading the response as it streams in

Answers stream in token by token rather than appearing all at once. Two things you'll see along the way:

- **🤔 Thinking…** — the model's reasoning, shown live while it works through the question. This scratch work isn't saved to history and disappears once the real answer starts.
- **🔍 *Tool label*…** — a short status line (e.g. "Fetching news...", "Calculating RSI...", "Detecting chart patterns...") that appears while the assistant is actually calling one of its tools to get real data, rather than just generating text.

## What kind of answers to expect

- **No boilerplate disclaimers.** The assistant is deliberately configured to skip "this isn't investment advice" / "past performance" / "consult a professional" language — every answer is expected to be direct and to the point, with real numbers.
- **No invented numbers.** If a data source is unavailable, the assistant is instructed to say so plainly rather than make something up.
- **Long positions only.** The trade-setup calculator (see **Trade Setup Workflow** in the table of contents) only computes long entries — it doesn't do short-side math.
- **It's read-only on the data side.** Every tool the assistant has either reads market data/news or changes something cosmetic on your screen (a chart overlay). Nothing it does submits a real trade — see **Trade Setup Workflow** for how order tickets actually get populated.

## What the assistant can do

Its capabilities fall into four groups, each with its own page in this guide (see the table of contents on the left):

- **Market Data & News Tools** — current quotes, fundamentals, price history, and headlines.
- **Technical Analysis Tools** — trend, RSI, MACD, volume, ATR, VWAP, and rule-based chart pattern detection.
- **Chart Overlays: Studies & Patterns** — how asking about an indicator or a pattern actually changes what's drawn on your chart.
- **Trade Setup Workflow** — turning a risk/reward question into a ready-to-review order ticket.

When you're ready for concrete examples, see **Tips & Example Prompts**.
