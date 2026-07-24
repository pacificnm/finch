import { useEffect, useRef } from "react";
import { fetchPriceHistory, type OhlcvData } from "./nest";

/** Interval values whose bars form within the current trading day. */
const INTRADAY_INTERVALS = new Set(["1m", "3m", "10m", "15m", "30m", "1h", "2h", "4h"]);

/** Refresh cadence while an intraday chart is open — fast enough to feel
 * live for day-trading without hammering the API. */
const INTRADAY_REFRESH_MS = 7_000;

/** Refresh cadence for daily/weekly/monthly charts — the one bar that's
 * still forming (today's) benefits from staying current, but there's no
 * need to poll it as aggressively as an intraday chart. */
const SLOW_REFRESH_MS = 60_000;

/**
 * Periodically refreshes `symbol`'s latest candle(s) and merges just the
 * changed tail into whatever `setCandles` currently holds — never a full
 * re-fetch of the viewed period, and never a value that would force
 * `CandlestickChart` to redraw from scratch: unchanged polls return the
 * same array reference (so React skips the re-render entirely), and changed
 * ones return a new array whose first bar's `time` is unchanged, which
 * `CandlestickChart` reads as "patch the tail in place", not "new dataset".
 *
 * Paused while `enabled` is false (e.g. the initial load is still in
 * flight, or the screen isn't visible) or `symbol` is empty.
 */
export function useLiveCandles(
  symbol: string,
  interval: string,
  enabled: boolean,
  setCandles: (updater: (current: OhlcvData[]) => OhlcvData[]) => void,
) {
  const inFlightRef = useRef(false);

  useEffect(() => {
    if (!enabled || !symbol) {
      return;
    }

    const isIntraday = INTRADAY_INTERVALS.has(interval);
    // "1d" (period_type=day) only pairs validly with minute frequencies on
    // Schwab's API; daily/weekly/monthly frequencies need a month+ period.
    // Either way this is a small, cheap window — never the full viewed range.
    const pollPeriod = isIntraday ? "1d" : "1m";
    const refreshMs = isIntraday ? INTRADAY_REFRESH_MS : SLOW_REFRESH_MS;

    const tick = async () => {
      if (inFlightRef.current) return;
      inFlightRef.current = true;
      try {
        const fresh = await fetchPriceHistory(symbol, pollPeriod, interval);
        if (fresh.length > 0) {
          setCandles((current) => mergeLatestCandles(current, fresh));
        }
      } catch (error) {
        // eslint-disable-next-line no-console
        console.error(`[useLiveCandles] refresh failed for ${symbol}:`, error);
      } finally {
        inFlightRef.current = false;
      }
    };

    const id = window.setInterval(() => void tick(), refreshMs);
    return () => window.clearInterval(id);
  }, [symbol, interval, enabled, setCandles]);
}

/**
 * Merges a small window of freshly fetched candles into an existing series:
 * replaces bars that already exist (matched by `time`) and appends any
 * that are newer, leaving everything else untouched. Returns the same
 * array reference when nothing actually changed, so callers using this as
 * a React state updater get a free no-op render.
 */
function mergeLatestCandles(existing: OhlcvData[], fresh: OhlcvData[]): OhlcvData[] {
  if (existing.length === 0) {
    return fresh;
  }

  const indexByTime = new Map(existing.map((bar, index) => [bar.time, index]));
  let merged = existing;
  let copied = false;

  for (const bar of fresh) {
    const index = indexByTime.get(bar.time);
    if (index !== undefined) {
      if (!candlesEqual(merged[index]!, bar)) {
        if (!copied) {
          merged = [...merged];
          copied = true;
        }
        merged[index] = bar;
      }
    } else if (bar.time > merged[merged.length - 1]!.time) {
      if (!copied) {
        merged = [...merged];
        copied = true;
      }
      merged.push(bar);
      indexByTime.set(bar.time, merged.length - 1);
    }
  }

  return merged;
}

function candlesEqual(a: OhlcvData, b: OhlcvData): boolean {
  return (
    a.open === b.open &&
    a.high === b.high &&
    a.low === b.low &&
    a.close === b.close &&
    a.volume === b.volume
  );
}
