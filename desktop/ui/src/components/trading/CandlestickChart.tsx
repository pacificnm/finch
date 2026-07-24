import { useEffect, useRef } from "react";
import {
  createChart,
  createSeriesMarkers,
  CandlestickSeries,
  HistogramSeries,
  LineSeries,
  ColorType,
  type IChartApi,
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

/**
 * A candlestick chart mounted via TradingView's lightweight-charts, with
 * optional Volume/Moving Average/RSI/MACD/ATR/VWAP study panes and AI-detected
 * chart pattern overlays (markers + trendlines). Rebuilt from scratch on
 * every `data`/`studies`/`patterns` change — simplest way to keep panes in
 * sync given how few series this chart carries; not worth the incremental-
 * update bookkeeping at this scale.
 */
export function CandlestickChart({ data, studies, patterns = [] }: CandlestickChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);

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
      },
    });
    chartRef.current = chart;

    const priceSeries = chart.addSeries(CandlestickSeries, {
      upColor: successColor,
      downColor: errorColor,
      borderVisible: false,
      wickUpColor: successColor,
      wickDownColor: errorColor,
    });
    priceSeries.setData(data);

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

    if (studies.movingAverage) {
      const smaSeries = chart.addSeries(LineSeries, {
        color: primaryColor,
        lineWidth: 2,
        priceLineVisible: false,
        lastValueVisible: false,
      });
      smaSeries.setData(simpleMovingAverage(data, 20));
    }

    if (studies.vwap) {
      const vwapSeries = chart.addSeries(LineSeries, {
        color: warningColor,
        lineWidth: 2,
        priceLineVisible: false,
        lastValueVisible: false,
      });
      vwapSeries.setData(vwap(data));
    }

    let nextPane = 1;
    if (studies.volume) {
      const volumeSeries = chart.addSeries(
        HistogramSeries,
        { priceLineVisible: false, lastValueVisible: false },
        nextPane,
      );
      volumeSeries.setData(volumeHistogram(data));
      nextPane += 1;
    }

    if (studies.rsi) {
      const rsiSeries = chart.addSeries(
        LineSeries,
        { color: primaryColor, lineWidth: 2, priceLineVisible: false },
        nextPane,
      );
      rsiSeries.setData(relativeStrengthIndex(data, 14));
      nextPane += 1;
    }

    if (studies.macd) {
      const { macdLine, signalLine, histogram } = macd(data);
      const macdHistogramSeries = chart.addSeries(
        HistogramSeries,
        { priceLineVisible: false, lastValueVisible: false },
        nextPane,
      );
      macdHistogramSeries.setData(histogram);
      const macdLineSeries = chart.addSeries(
        LineSeries,
        { color: primaryColor, lineWidth: 2, priceLineVisible: false, lastValueVisible: false },
        nextPane,
      );
      macdLineSeries.setData(macdLine);
      const signalLineSeries = chart.addSeries(
        LineSeries,
        { color: errorColor, lineWidth: 1, priceLineVisible: false, lastValueVisible: false },
        nextPane,
      );
      signalLineSeries.setData(signalLine);
      nextPane += 1;
    }

    if (studies.atr) {
      const atrSeries = chart.addSeries(
        LineSeries,
        { color: warningColor, lineWidth: 2, priceLineVisible: false },
        nextPane,
      );
      atrSeries.setData(averageTrueRange(data, 14));
      nextPane += 1;
    }

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
    };
  }, [data, studies, patterns]);

  return <div ref={containerRef} className="h-full w-full" />;
}
