# Trade Setup Workflow

Ask for a buy price, stop loss, target price, position size, or risk/reward math, and the assistant calculates a complete trade setup — entry, stop, target, share count, dollar risk, dollar reward, and the risk/reward ratio.

**This only computes long positions.** There's no short-side math here.

## Parameters

| Parameter | Required? | Meaning |
|---|---|---|
| `account_size` | Yes | Total account size / capital available, in USD |
| `risk_percent` | Yes | Max risk as a percent of account size (e.g. `1` for 1%) |
| `target_percent` | Yes | Target gain as a percent of entry price (e.g. `3` for 3%) |
| `entry_price` | No | A specific entry price — if you don't give one, the current market price is used |

You don't need to phrase these as literal parameter names — just state them naturally:

> "I have $10,000, want a 3% gain, and I'm willing to risk 1%. What's the setup?"

## What happens after the calculation

The assistant first presents the numbers in a clean, readable summary directly in the chat — entry, stop, target, share count, position size, dollar risk (with % of account), dollar reward, and the risk/reward ratio.

Then it asks whether you'd like to populate the order ticket, and a **highlighted card appears below the chat** showing the same entry/stop/target/shares at a glance, with two buttons:

- **Populate order ticket** — fills the real order ticket on the Trade screen with these exact values. You still review and submit the order yourself; nothing is placed automatically.
- **Dismiss** — discards the suggested setup without touching the order ticket.

Nothing about this workflow submits a trade on its own — the assistant only ever gets as far as filling in the ticket for you to review.
