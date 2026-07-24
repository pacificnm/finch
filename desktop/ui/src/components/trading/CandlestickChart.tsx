import { useEffect, useRef } from "react";
import {
  createChart,
  createSeriesMarkers,
  CandlestickSeries,
  HistogramSeries,
  LineSeries,
  ColorType,
  type IChartApi,
  type ISeriesApi,
  type SeriesMarker,
  type Time,
} from "lightweight-charts";
import {
  averageTrueRange,
  macd,
  relativeStrengthIndex,
  simpleMovingAverage,
  volumeHistogram,
  vwap,
} from "../../lib/indicators";
import { TrendlinePrimitive } from "../../lib/trendlinePrimitive";
import type { OhlcvData } from "../../lib/nest";
import { makeTickMarkFormatter, makeTimeFormatter } from "../../lib/chartTimeFormat";
import { useTimezone } from "../../context/TimezoneContext";

export type ActiveStudies = {
  volume: boolean;
  movingAverage: boolean;
  rsi: boolean;
  macd: boolean;
  atr: boolean;
  vwap: boolean;
};

/** One swing point that defines a detected chart pattern (a peak, trough, shoulder, head, etc.). */
export type PatternPoint = {
  /** "YYYY-MM-DD" — patterns are daily-only for now. */
  date: string;
  price: number;
  role: string;
  kind: "high" | "low";
};

export type PatternLinePoint = {
  date: string;
  price: number;
};

/** A straight segment (neckline, trendline side) drawn between two points. */
export type PatternLine = {
  role: string;
  from: PatternLinePoint;
  to: PatternLinePoint;
};

/** A single AI-detected chart pattern, as produced by the `detect_chart_patterns` tool. */
export type ChartPattern = {
  kind: string;
  status: "forming" | "confirmed" | string;
  label: string;
  note: string;
  points: PatternPoint[];
  lines: PatternLine[];
};

type CandlestickChartProps = {
  data: OhlcvData[];
  studies: ActiveStudies;
  /** AI-detected chart pattern overlays (trendlines + labeled swing points). */
  patterns?: ChartPattern[];
};

const PATTERN_POINT_LABELS: Record<string, string> = {
  first_peak: "Peak 1",
  second_peak: "Peak 2",
  first_trough: "Trough 1",
  second_trough: "Trough 2",
  trough: "Trough",
  peak: "Peak",
  left_shoulder: "LS",
  right_shoulder: "RS",
  head: "Head",
  left_trough: "LT",
  right_trough: "RT",
  left_peak: "LP",
  right_peak: "RP",
  swing_high: "H",
  swing_low: "L",
};

const PATTERN_LINE_LABELS: Record<string, string> = {
  neckline: "Neckline",
  upper_trendline: "Upper",
  lower_trendline: "Lower",
};

/** Bearish patterns render in the error color, bullish in the success color; triangles are directionally neutral until broken, so they use the primary color. */
function patternColor(
  kind: string,
  colors: { success: string; error: string; primary: string },
): string {
  if (kind === "double_top" || kind === "head_and_shoulders") {
    return colors.error;
  }
  if (kind === "double_bottom" || kind === "inverse_head_and_shoulders") {
    return colors.success;
  }
  return colors.primary;
}

function cssVar(name: string, fallback: string): string {
  if (typeof window === "undefined") {
    return fallback;
  }
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

/** Series refs the data-application effect needs — populated by the
 * structural effect whenever it (re)creates the chart, `null` for any
 * study that's currently off. */
type SeriesRefs = {
  price: ISeriesApi<"Candlestick"> | null;
  sma: ISeriesApi<"Line"> | null;
  vwap: ISeriesApi<"Line"> | null;
  volume: ISeriesApi<"Histogram"> | null;
  rsi: ISeriesApi<"Line"> | null;
  macdHistogram: ISeriesApi<"Histogram"> | null;
  macdLine: ISeriesApi<"Line"> | null;
  macdSignal: ISeriesApi<"Line"> | null;
  atr: ISeriesApi<"Line"> | null;
};

const EMPTY_SERIES_REFS: SeriesRefs = {
  price: null,
  sma: null,
  vwap: null,
  volume: null,
  rsi: null,
  macdHistogram: null,
  macdLine: null,
  macdSignal: null,
  atr: null,
};

/** Full `setData` on every active series — used for the initial paint and
 * whenever the dataset itself changed (symbol/period/interval switch),
 * as opposed to just its latest bar (a live-refresh tick). */
function applyFullData(series: SeriesRefs, data: OhlcvData[]) {
  series.price?.setData(data);
  series.sma?.setData(simpleMovingAverage(data, 20));
  series.vwap?.setData(vwap(data));
  series.volume?.setData(volumeHistogram(data));
  series.rsi?.setData(relativeStrengthIndex(data, 14));
  series.atr?.setData(averageTrueRange(data, 14));
  if (series.macdLine || series.macdSignal || series.macdHistogram) {
    const { macdLine, signalLine, histogram } = macd(data);
    series.macdLine?.setData(macdLine);
    series.macdSignal?.setData(signalLine);
    series.macdHistogram?.setData(histogram);
  }
}

/** Patches just the latest bar/point on every active series — no dataset
 * reset, no `fitContent()`, so zoom/pan/crosshair state survives a live
 * refresh tick untouched. Safe to call with more than one new/changed bar
 * (e.g. after a missed poll); each is applied in order. */
function applyIncrementalUpdate(series: SeriesRefs, data: OhlcvData[], newBars: OhlcvData[]) {
  for (const bar of newBars) {
    series.price?.update(bar);
    series.volume?.update(volumeHistogram([bar])[0]!);
  }
  // Windowed indicators (SMA/RSI/MACD/ATR/VWAP) aren't simple per-bar
  // transforms — recomputing them over the full (still-cheap) series and
  // taking just the last point is far simpler than maintaining incremental
  // running state for five different formulas, and just as flicker-free
  // since only `.update()` (not `.setData()`) is called.
  const lastOf = <T,>(values: T[]): T | undefined => values[values.length - 1];

  if (series.sma) {
    const last = lastOf(simpleMovingAverage(data, 20));
    if (last) series.sma.update(last);
  }
  if (series.vwap) {
    const last = lastOf(vwap(data));
    if (last) series.vwap.update(last);
  }
  if (series.rsi) {
    const last = lastOf(relativeStrengthIndex(data, 14));
    if (last) series.rsi.update(last);
  }
  if (series.atr) {
    const last = lastOf(averageTrueRange(data, 14));
    if (last) series.atr.update(last);
  }
  if (series.macdLine || series.macdSignal || series.macdHistogram) {
    const { macdLine, signalLine, histogram } = macd(data);
    const lastMacd = lastOf(macdLine);
    const lastSignal = lastOf(signalLine);
    const lastHist = lastOf(histogram);
    if (lastMacd) series.macdLine?.update(lastMacd);
    if (lastSignal) series.macdSignal?.update(lastSignal);
    if (lastHist) series.macdHistogram?.update(lastHist);
  }
}

/**
 * A candlestick chart mounted via TradingView's lightweight-charts, with
 * optional Volume/Moving Average/RSI/MACD/ATR/VWAP study panes and
 * AI-detected chart pattern overlays (markers + trendlines).
 *
 * Two separate effects, deliberately not merged:
 * - Structural (chart/series/pane creation, pattern markers) runs only when
 *   `studies`/`patterns` change — rare, user-initiated, a full rebuild here
 *   is cheap and simplest.
 * - Data application runs on every `data` change and never tears down the
 *   chart. It distinguishes a genuinely different dataset (symbol/period/
 *   interval switch — full `setData` + refit) from a live-refresh tick
 *   (same series, latest bar(s) changed — `.update()` only), so a 5-10s
 *   price poll never flickers, resets zoom, or loses the crosshair.
 */
export function CandlestickChart({ data, studies, patterns = [] }: CandlestickChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<SeriesRefs>(EMPTY_SERIES_REFS);
  // Seeded with the initial `data` (not []) so the structural effect below
  // paints the real dataset on first mount rather than an empty chart that
  // only self-corrects once the data-application effect runs after it.
  const previousDataRef = useRef<OhlcvData[]>(data);
  const { timezone } = useTimezone();

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    const borderColor = cssVar("--nest-color-border", "#b8bfc6");
    const textColor = cssVar("--nest-color-muted", "#4a5568");
    const successColor = cssVar("--nest-color-success", "#006a4d");
    const errorColor = cssVar("--nest-color-error", "#b91c1c");
    const primaryColor = cssVar("--nest-color-primary", "#003f2d");
    const warningColor = cssVar("--nest-color-warning", "#b45309");

    const chart = createChart(container, {
      width: container.clientWidth,
      height: container.clientHeight,
      layout: {
        // Transparent, not a solid color read from theme vars at mount time —
        // a baked-in solid color wouldn't update if the theme changes after
        // the chart is created (canvas draws, it isn't CSS). Transparent
        // always shows whatever's behind it, in whatever the current theme is.
        background: { type: ColorType.Solid, color: "transparent" },
        textColor,
      },
      grid: {
        vertLines: { color: borderColor },
        horzLines: { color: borderColor },
      },
      rightPriceScale: { borderColor },
      timeScale: {
        borderColor,
        timeVisible: true,
        secondsVisible: false,
        tickMarkFormatter: makeTickMarkFormatter(timezone),
      },
      localization: {
        timeFormatter: makeTimeFormatter(timezone),
      },
    });
    chartRef.current = chart;

    const currentData = previousDataRef.current;
    const priceSeries = chart.addSeries(CandlestickSeries, {
      upColor: successColor,
      downColor: errorColor,
      borderVisible: false,
      wickUpColor: successColor,
      wickDownColor: errorColor,
    });
    priceSeries.setData(currentData);

    if (patterns.length > 0) {
      const markers: SeriesMarker<Time>[] = patterns.flatMap((pattern) => {
        const color = patternColor(pattern.kind, {
          success: successColor,
          error: errorColor,
          primary: primaryColor,
        });
        return pattern.points.map((point) => ({
          time: point.date as Time,
          position: point.kind === "high" ? "aboveBar" : "belowBar",
          shape: point.kind === "high" ? "arrowDown" : "arrowUp",
          color,
          text: PATTERN_POINT_LABELS[point.role] ?? point.role,
        }));
      });
      createSeriesMarkers(priceSeries, markers);

      for (const pattern of patterns) {
        const color = patternColor(pattern.kind, {
          success: successColor,
          error: errorColor,
          primary: primaryColor,
        });
        for (const line of pattern.lines) {
          priceSeries.attachPrimitive(
            new TrendlinePrimitive(
              { time: line.from.date as Time, price: line.from.price },
              { time: line.to.date as Time, price: line.to.price },
              {
                color,
                dashed: pattern.status !== "confirmed",
                label: PATTERN_LINE_LABELS[line.role] ?? line.role,
              },
            ),
          );
        }
      }
    }

    const nextRefs: SeriesRefs = { ...EMPTY_SERIES_REFS, price: priceSeries };

    if (studies.movingAverage) {
      nextRefs.sma = chart.addSeries(LineSeries, {
        color: primaryColor,
        lineWidth: 2,
        priceLineVisible: false,
        lastValueVisible: false,
      });
      nextRefs.sma.setData(simpleMovingAverage(currentData, 20));
    }

    if (studies.vwap) {
      nextRefs.vwap = chart.addSeries(LineSeries, {
        color: warningColor,
        lineWidth: 2,
        priceLineVisible: false,
        lastValueVisible: false,
      });
      nextRefs.vwap.setData(vwap(currentData));
    }

    let nextPane = 1;
    if (studies.volume) {
      nextRefs.volume = chart.addSeries(
        HistogramSeries,
        { priceLineVisible: false, lastValueVisible: false },
        nextPane,
      );
      nextRefs.volume.setData(volumeHistogram(currentData));
      nextPane += 1;
    }

    if (studies.rsi) {
      nextRefs.rsi = chart.addSeries(
        LineSeries,
        { color: primaryColor, lineWidth: 2, priceLineVisible: false },
        nextPane,
      );
      nextRefs.rsi.setData(relativeStrengthIndex(currentData, 14));
      nextPane += 1;
    }

    if (studies.macd) {
      const { macdLine, signalLine, histogram } = macd(currentData);
      nextRefs.macdHistogram = chart.addSeries(
        HistogramSeries,
        { priceLineVisible: false, lastValueVisible: false },
        nextPane,
      );
      nextRefs.macdHistogram.setData(histogram);
      nextRefs.macdLine = chart.addSeries(
        LineSeries,
        { color: primaryColor, lineWidth: 2, priceLineVisible: false, lastValueVisible: false },
        nextPane,
      );
      nextRefs.macdLine.setData(macdLine);
      nextRefs.macdSignal = chart.addSeries(
        LineSeries,
        { color: errorColor, lineWidth: 1, priceLineVisible: false, lastValueVisible: false },
        nextPane,
      );
      nextRefs.macdSignal.setData(signalLine);
      nextPane += 1;
    }

    if (studies.atr) {
      nextRefs.atr = chart.addSeries(
        LineSeries,
        { color: warningColor, lineWidth: 2, priceLineVisible: false },
        nextPane,
      );
      nextRefs.atr.setData(averageTrueRange(currentData, 14));
      nextPane += 1;
    }

    seriesRef.current = nextRefs;
    chart.timeScale().fitContent();

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) {
        return;
      }
      chart.applyOptions({
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      });
    });
    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
      chart.remove();
      chartRef.current = null;
      seriesRef.current = EMPTY_SERIES_REFS;
    };
    // `data` deliberately excluded — the effect below applies it, both at
    // creation time (via `previousDataRef`, already up to date by the time
    // this runs) and on every subsequent change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [studies, patterns, timezone]);

  useEffect(() => {
    if (!chartRef.current) {
      // Structural effect hasn't created the chart yet on this render pass —
      // just record `data` so it's there when that effect runs.
      previousDataRef.current = data;
      return;
    }

    const previous = previousDataRef.current;
    const sameDataset =
      previous.length > 0 && data.length > 0 && previous[0]!.time === data[0]!.time;

    if (!sameDataset) {
      applyFullData(seriesRef.current, data);
      chartRef.current.timeScale().fitContent();
    } else {
      const previousLastTime = previous[previous.length - 1]!.time;
      const startIndex = data.findIndex((bar) => bar.time === previousLastTime);
      const newBars = startIndex === -1 ? data.slice(-1) : data.slice(startIndex);
      if (newBars.length > 0) {
        applyIncrementalUpdate(seriesRef.current, data, newBars);
      }
    }

    previousDataRef.current = data;
  }, [data]);

  return <div ref={containerRef} className="h-full w-full" />;
}
