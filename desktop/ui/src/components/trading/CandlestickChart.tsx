import { useEffect, useRef } from "react";
import {
  createChart,
  CandlestickSeries,
  HistogramSeries,
  LineSeries,
  ColorType,
  type IChartApi,
} from "lightweight-charts";
import {
  averageTrueRange,
  macd,
  relativeStrengthIndex,
  simpleMovingAverage,
  volumeHistogram,
  vwap,
} from "../../lib/indicators";
import type { OhlcvData } from "../../lib/nest";

export type ActiveStudies = {
  volume: boolean;
  movingAverage: boolean;
  rsi: boolean;
  macd: boolean;
  atr: boolean;
  vwap: boolean;
};

type CandlestickChartProps = {
  data: OhlcvData[];
  studies: ActiveStudies;
};

function cssVar(name: string, fallback: string): string {
  if (typeof window === "undefined") {
    return fallback;
  }
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

/**
 * A candlestick chart mounted via TradingView's lightweight-charts, with
 * optional Volume/Moving Average/RSI/MACD/ATR/VWAP study panes. Rebuilt from scratch on
 * every `data`/`studies` change — simplest way to keep panes in sync given
 * how few series this chart carries; not worth the incremental-update
 * bookkeeping at this scale.
 */
export function CandlestickChart({ data, studies }: CandlestickChartProps) {
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
  }, [data, studies]);

  return <div ref={containerRef} className="h-full w-full" />;
}
