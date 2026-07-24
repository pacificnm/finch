# Chart Overlays: Studies & Patterns

Two of the assistant's tools don't just answer in text — they also change what's drawn on the chart next to the chat. This page explains what you'll actually *see* happen in each case, since the two behave differently.

## Chart studies (indicators)

Ask the assistant to show, hide, add, remove, or turn on/off a study or indicator overlay, and it toggles the corresponding pane or line directly on the chart:

- **Volume** — histogram pane
- **Moving average** — 20-day simple moving average line
- **RSI** — dedicated pane
- **MACD** — dedicated pane (line, signal, and histogram)
- **ATR** — dedicated pane (14-day)
- **VWAP** — line overlaid directly on price

> "Show me the RSI."
> "Turn off volume, add MACD."

**What happens:** the change applies **immediately and automatically** — there's no confirmation click, since this is purely cosmetic and fully reversible. Only the studies you actually mentioned change; anything already on stays on, anything already off stays off. Studies are shared between the Trade and Charts screens, so turning one on in either place shows it in both.

## Chart patterns

Ask the assistant to find or explain a chart pattern (see **Technical Analysis Tools** for the full list of pattern types it checks), and when it finds one, the chart gets the pattern's actual geometry drawn on it — the specific swing points (peaks, troughs, shoulders, a head) as labeled markers, plus the lines that define the pattern (a neckline for a double top or head & shoulders, the two converging sides of a triangle).

**What happens:** unlike chart studies, a fresh pattern answer **replaces** whatever pattern overlay is currently drawn — it doesn't add on top of a previous one. Pattern overlays are also automatically cleared the moment you switch to a different symbol, since a pattern drawn from one stock's price history would be actively misleading on another's chart.

A pattern that hasn't fully broken out yet is shown as **forming**; one that has is shown as **confirmed** — the assistant's explanation in chat will say which.

> "What pattern do you see on this chart?"
