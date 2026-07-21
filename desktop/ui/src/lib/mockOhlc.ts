import type { CandlestickData, Time } from "lightweight-charts";

/** Deterministic PRNG (mulberry32) so mock chart data is stable across renders. */
function mulberry32(seed: number): () => number {
  let a = seed;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function toDateString(date: Date): Time {
  return date.toISOString().slice(0, 10) as Time;
}

/**
 * Generates a plausible-looking daily candlestick series via a random walk —
 * placeholder data until this is wired to nest-schwab's `price_history`.
 */
export function generateMockCandles(
  days = 180,
  startPrice = 100,
  seed = 42,
): CandlestickData[] {
  const random = mulberry32(seed);
  const candles: CandlestickData[] = [];
  let price = startPrice;
  const today = new Date();
  today.setUTCHours(0, 0, 0, 0);

  for (let i = days; i >= 0; i -= 1) {
    const date = new Date(today);
    date.setUTCDate(date.getUTCDate() - i);
    const day = date.getUTCDay();
    if (day === 0 || day === 6) {
      continue;
    }

    const drift = (random() - 0.48) * 2.5;
    const open = price;
    const close = Math.max(1, open + drift);
    const high = Math.max(open, close) + random() * 1.5;
    const low = Math.min(open, close) - random() * 1.5;
    price = close;

    candles.push({
      time: toDateString(date),
      open: round2(open),
      high: round2(high),
      low: round2(Math.max(0.01, low)),
      close: round2(close),
    });
  }

  return candles;
}

function round2(value: number): number {
  return Math.round(value * 100) / 100;
}

/** Aggregates daily candles into weekly bars (Mon-anchored), for the interval selector. */
export function aggregateWeekly(candles: CandlestickData[]): CandlestickData[] {
  const weeks = new Map<string, CandlestickData[]>();
  for (const candle of candles) {
    const date = new Date(`${candle.time as string}T00:00:00Z`);
    const weekStart = new Date(date);
    const isoDay = (date.getUTCDay() + 6) % 7; // Monday = 0
    weekStart.setUTCDate(date.getUTCDate() - isoDay);
    const key = toDateString(weekStart) as string;
    const bucket = weeks.get(key);
    if (bucket) {
      bucket.push(candle);
    } else {
      weeks.set(key, [candle]);
    }
  }

  return Array.from(weeks.entries())
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([weekStart, bucket]) => ({
      time: weekStart as Time,
      open: bucket[0]!.open,
      close: bucket.at(-1)!.close,
      high: Math.max(...bucket.map((c) => c.high)),
      low: Math.min(...bucket.map((c) => c.low)),
    }));
}
