# Tips & Example Prompts

## Just ask naturally

There's no special syntax to learn — plain questions work. The assistant figures out which of its tools to call based on what you ask.

## Combine tools in one question

The assistant can call more than one tool while answering a single question, so you don't need to break things into separate messages:

> "Give me the RSI and MACD, and tell me if volume confirms the current move."
>
> "What's the trend look like, and is there a chart pattern forming?"
>
> "Show me the ATR and MACD on the chart, then check for any pattern."

## Remember: history is per-symbol

Switching symbols starts (or resumes) a completely separate conversation — the assistant won't carry context over from a different ticker. If you want to compare two symbols, you'll need to ask about them in their own respective conversations rather than in one thread.

## Example prompts by category

**Market data**
> "What's the current price and PE ratio?"
> "Any recent news, especially about earnings?"

**Technical analysis**
> "Is this overbought or oversold right now?"
> "What does the trend look like over the last 6 months?"
> "Check for a head and shoulders pattern."

**Chart controls**
> "Add the moving average and turn on volume."
> "Hide everything except VWAP."

**Trade planning**
> "With $5,000 and 2% risk, what's a setup targeting a 4% gain?"

## If something seems off

If the assistant says live data is unavailable for a request, that's expected behavior when a data source fails — it's instructed to say so plainly rather than invent numbers. Try again, or ask a differently-scoped question (a shorter timeframe, a different tool) in the meantime.
