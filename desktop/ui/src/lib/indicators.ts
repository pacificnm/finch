import type { CandlestickData, LineData, HistogramData, Time } from "lightweight-charts";

/** Simple moving average of close price, aligned to `candles`' time axis. */
export function simpleMovingAverage(candles: CandlestickData[], period: number): LineData[] {
  const result: LineData[] = [];
  let sum = 0;
  for (let i = 0; i < candles.length; i += 1) {
    sum += candles[i]!.close;
    if (i >= period) {
      sum -= candles[i - period]!.close;
    }
    if (i >= period - 1) {
      result.push({ time: candles[i]!.time, value: round2(sum / period) });
    }
  }
  return result;
}

/** Wilder's RSI, aligned to `candles`' time axis (starts after the warm-up period). */
export function relativeStrengthIndex(candles: CandlestickData[], period = 14): LineData[] {
  if (candles.length <= period) {
    return [];
  }
  const result: LineData[] = [];
  let avgGain = 0;
  let avgLoss = 0;

  for (let i = 1; i <= period; i += 1) {
    const change = candles[i]!.close - candles[i - 1]!.close;
    avgGain += Math.max(change, 0);
    avgLoss += Math.max(-change, 0);
  }
  avgGain /= period;
  avgLoss /= period;
  result.push({ time: candles[period]!.time, value: rsiFromAverages(avgGain, avgLoss) });

  for (let i = period + 1; i < candles.length; i += 1) {
    const change = candles[i]!.close - candles[i - 1]!.close;
    const gain = Math.max(change, 0);
    const loss = Math.max(-change, 0);
    avgGain = (avgGain * (period - 1) + gain) / period;
    avgLoss = (avgLoss * (period - 1) + loss) / period;
    result.push({ time: candles[i]!.time, value: rsiFromAverages(avgGain, avgLoss) });
  }

  return result;
}

function rsiFromAverages(avgGain: number, avgLoss: number): number {
  if (avgLoss === 0) {
    return 100;
  }
  const rs = avgGain / avgLoss;
  return round2(100 - 100 / (1 + rs));
}

/** Synthetic volume bars, colored by candle direction — placeholder until real volume is wired. */
export function mockVolume(candles: CandlestickData[], seed = 11): HistogramData[] {
  const random = mulberry32(seed);
  return candles.map((candle) => ({
    time: candle.time,
    value: Math.round(500_000 + random() * 2_000_000),
    color: candle.close >= candle.open ? "rgba(0, 106, 77, 0.5)" : "rgba(185, 28, 28, 0.5)",
  }));
}

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

function round2(value: number): number {
  return Math.round(value * 100) / 100;
}

export type { Time };
