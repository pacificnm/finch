import type { CandlestickData, LineData, HistogramData, Time } from "lightweight-charts";
import type { OhlcvData } from "./nest";

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

/** Volume bars from each candle's own volume, colored by candle direction. */
export function volumeHistogram(candles: OhlcvData[]): HistogramData[] {
  return candles.map((candle) => ({
    time: candle.time,
    value: candle.volume,
    color: candle.close >= candle.open ? "rgba(0, 106, 77, 0.5)" : "rgba(185, 28, 28, 0.5)",
  }));
}

/** Standard 12/26/9 MACD: line, signal, and histogram, all aligned to the signal's warm-up-adjusted range. */
export type MacdResult = {
  macdLine: LineData[];
  signalLine: LineData[];
  histogram: HistogramData[];
};

export function macd(
  candles: CandlestickData[],
  fastPeriod = 12,
  slowPeriod = 26,
  signalPeriod = 9,
): MacdResult {
  const closes = candles.map((c) => c.close);
  const emaFast = exponentialMovingAverage(closes, fastPeriod);
  const emaSlow = exponentialMovingAverage(closes, slowPeriod);
  if (emaSlow.length === 0) {
    return { macdLine: [], signalLine: [], histogram: [] };
  }

  // emaFast is aligned to closes[fastPeriod-1..]; emaSlow to closes[slowPeriod-1..].
  // Drop emaFast's lead so both start at the same close index.
  const offset = slowPeriod - fastPeriod;
  const macdValues = emaSlow.map((slowValue, i) => emaFast[i + offset]! - slowValue);
  const macdTimes = candles.slice(slowPeriod - 1).map((c) => c.time);

  const signalValues = exponentialMovingAverage(macdValues, signalPeriod);
  if (signalValues.length === 0) {
    return { macdLine: [], signalLine: [], histogram: [] };
  }
  const signalTimes = macdTimes.slice(signalPeriod - 1);

  const histogramValues = signalValues.map(
    (signalValue, i) => macdValues[i + signalPeriod - 1]! - signalValue,
  );

  return {
    macdLine: signalTimes.map((time, i) => ({
      time,
      value: round2(macdValues[i + signalPeriod - 1]!),
    })),
    signalLine: signalTimes.map((time, i) => ({ time, value: round2(signalValues[i]!) })),
    histogram: signalTimes.map((time, i) => ({
      time,
      value: round2(histogramValues[i]!),
      color: histogramValues[i]! >= 0 ? "rgba(0, 106, 77, 0.5)" : "rgba(185, 28, 28, 0.5)",
    })),
  };
}

/** Exponential moving average, aligned to `values[period - 1..]`. */
function exponentialMovingAverage(values: number[], period: number): number[] {
  if (values.length < period || period <= 0) {
    return [];
  }
  const k = 2 / (period + 1);
  const seed = values.slice(0, period).reduce((sum, value) => sum + value, 0) / period;
  const result = [seed];
  for (let i = period; i < values.length; i += 1) {
    const prev = result[result.length - 1]!;
    result.push(values[i]! * k + prev * (1 - k));
  }
  return result;
}

/** Wilder's smoothing (the method behind ATR and Wilder-style RSI): seeds
 * with a simple average of the first `period` values, then applies
 * `(prev * (period - 1) + value) / period` for each subsequent value. */
function wilderSmooth(values: number[], period: number): number[] {
  if (values.length < period || period <= 0) {
    return [];
  }
  const seed = values.slice(0, period).reduce((sum, value) => sum + value, 0) / period;
  const result = [seed];
  for (let i = period; i < values.length; i += 1) {
    const prev = result[result.length - 1]!;
    result.push((prev * (period - 1) + values[i]!) / period);
  }
  return result;
}

/** Average True Range, Wilder-smoothed, aligned to the smoothed range's start. */
export function averageTrueRange(candles: OhlcvData[], period = 14): LineData[] {
  const trueRanges: number[] = [];
  let prevClose: number | null = null;
  for (const candle of candles) {
    const tr =
      prevClose === null
        ? candle.high - candle.low
        : Math.max(
            candle.high - candle.low,
            Math.abs(candle.high - prevClose),
            Math.abs(candle.low - prevClose),
          );
    trueRanges.push(tr);
    prevClose = candle.close;
  }

  const smoothed = wilderSmooth(trueRanges, period);
  if (smoothed.length === 0) {
    return [];
  }
  const times = candles.slice(period - 1).map((c) => c.time);
  return times.map((time, i) => ({ time, value: round2(smoothed[i]!) }));
}

/**
 * Volume-weighted average price, resetting at each calendar day boundary —
 * the standard intraday behavior. On daily-bar charts each bar is its own
 * "session", so VWAP degenerates to that bar's own typical price; it's most
 * informative on intraday interval charts, matching how the backend's
 * calculate_vwap tool works.
 */
export function vwap(candles: OhlcvData[]): LineData[] {
  const result: LineData[] = [];
  let cumulativePv = 0;
  let cumulativeVolume = 0;
  let currentDay: string | null = null;

  for (const candle of candles) {
    const day = dayKeyFromTime(candle.time);
    if (day !== currentDay) {
      cumulativePv = 0;
      cumulativeVolume = 0;
      currentDay = day;
    }
    const typicalPrice = (candle.high + candle.low + candle.close) / 3;
    cumulativePv += typicalPrice * candle.volume;
    cumulativeVolume += candle.volume;
    if (cumulativeVolume > 0) {
      result.push({ time: candle.time, value: round2(cumulativePv / cumulativeVolume) });
    }
  }

  return result;
}

function dayKeyFromTime(time: Time): string {
  if (typeof time === "number") {
    return new Date(time * 1000).toISOString().slice(0, 10);
  }
  return time as string;
}

function round2(value: number): number {
  return Math.round(value * 100) / 100;
}

export type { Time };
