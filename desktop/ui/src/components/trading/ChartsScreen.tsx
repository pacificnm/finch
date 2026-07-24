import { useEffect, useMemo, useState } from "react";
import { Select, type SelectOption } from "@nest/components";
import { Bell, ChevronDown, LayoutGrid, Plus } from "lucide-react";
import { CandlestickChart, type ActiveStudies, type ChartPattern } from "./CandlestickChart";
import { StudiesDialog } from "./StudiesDialog";
import { ChartSettingsDialog } from "./ChartSettingsDialog";
import { getIntervalOptionsForPeriod } from "../../lib/chartIntervals";
import { aggregateWeekly, generateMockCandles } from "../../lib/mockOhlc";
import { fetchPriceHistory, fetchQuote, type OhlcvData } from "../../lib/nest";
import { useLiveCandles } from "../../lib/useLiveCandles";
import { MOCK_SYMBOL } from "./TradeScreen";

type QuoteData = {
  description?: string;
  lastPrice?: number;
  netChange?: number;
  percentChange?: number;
  bidSize?: string;
  askSize?: string;
};

type ChartsScreenProps = {
  /** Symbol to display. Defaults to MOCK_SYMBOL when not provided. */
  symbol?: string;
  /** Which chart studies are on — shared with the Trade screen and the AI chat panel. */
  studies: ActiveStudies;
  /** Toggles one chart study on/off. */
  onToggleStudy: (key: keyof ActiveStudies) => void;
  /** AI-detected chart pattern overlays — shared with the Trade screen and the AI chat panel. */
  patterns?: ChartPattern[];
  /** Selected chart period, e.g. "1y" — shared with the Trade screen and persisted. */
  period: string;
  /** Called when the user changes the period. */
  onPeriodChange: (value: string) => void;
  /** Selected chart interval, e.g. "1d" — shared with the Trade screen and persisted. */
  aggregation: string;
  /** Called when the user changes the interval. */
  onAggregationChange: (value: string) => void;
};

const PERIOD_OPTIONS: (SelectOption & { days: number })[] = [
  { value: "today", label: "Today", days: 1 },
  { value: "1d", label: "Day", days: 1 },
  { value: "3d", label: "3 Days", days: 3 },
  { value: "1w", label: "Week", days: 7 },
  { value: "2w", label: "2 Weeks", days: 14 },
  { value: "1m", label: "1 Month", days: 30 },
  { value: "3m", label: "3 Months", days: 90 },
  { value: "6m", label: "6 Months", days: 180 },
  { value: "ytd", label: "Year-To-Date", days: 250 },
  { value: "1y", label: "1 Year", days: 365 },
  { value: "3y", label: "3 Years", days: 1095 },
  { value: "5y", label: "5 Years", days: 1825 },
  { value: "15y", label: "15 Years", days: 5475 },
  { value: "max", label: "Max", days: 7300 },
];

/** Charts section: symbol header, chart toolbar, and the candlestick chart. */
export function ChartsScreen({
  symbol = MOCK_SYMBOL,
  studies,
  onToggleStudy,
  patterns = [],
  period,
  onPeriodChange,
  aggregation,
  onAggregationChange,
}: ChartsScreenProps) {
  const [studiesOpen, setStudiesOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [quoteData, setQuoteData] = useState<QuoteData | null>(null);
  const [candles, setCandles] = useState<OhlcvData[]>([]);
  const [candlesLoading, setCandlesLoading] = useState(false);
  const [candlesError, setCandlesError] = useState(false);

  const intervalOptions = useMemo(() => getIntervalOptionsForPeriod(period), [period]);

  // Fetch real price history when symbol, period, or interval changes.
  useEffect(() => {
    if (!symbol) return;

    let cancelled = false;

    const loadCandles = async () => {
      setCandlesLoading(true);
      setCandlesError(false);
      try {
        const data = await fetchPriceHistory(symbol, period, aggregation);
        if (!cancelled) {
          if (data.length > 0) {
            setCandles(data);
          } else {
            const days = PERIOD_OPTIONS.find((option) => option.value === period)?.days ?? 365;
            const daily = generateMockCandles(days, 30, 7);
            setCandles(aggregation === "1w" ? aggregateWeekly(daily) : daily);
          }
        }
      } catch (error) {
        console.error(`[ChartsScreen] Failed to fetch price history for ${symbol}:`, error);
        if (!cancelled) {
          setCandlesError(true);
          const days = PERIOD_OPTIONS.find((option) => option.value === period)?.days ?? 365;
          const daily = generateMockCandles(days, 30, 7);
          setCandles(aggregation === "1w" ? aggregateWeekly(daily) : daily);
        }
      } finally {
        if (!cancelled) {
          setCandlesLoading(false);
        }
      }
    };

    loadCandles();

    return () => {
      cancelled = true;
    };
  }, [symbol, period, aggregation]);

  // Keep the chart current without redrawing it: polls a small recent
  // window and merges just the changed tail (see useLiveCandles for why
  // this never resets zoom/pan or flickers).
  useLiveCandles(symbol, aggregation, !candlesLoading, setCandles);

  // Fetch quote data when symbol changes.
  useEffect(() => {
    if (!symbol) return;

    let cancelled = false;

    const loadQuote = async () => {
      try {
        const data = await fetchQuote(symbol);
        if (!cancelled) {
          setQuoteData(data);
        }
      } catch (error) {
        console.error(`[ChartsScreen] Failed to fetch quote for ${symbol}:`, error);
        if (!cancelled) {
          setQuoteData({ description: "Failed to load" });
        }
      }
    };

    loadQuote();

    return () => {
      cancelled = true;
    };
  }, [symbol]);

  const last = candles.at(-1);
  const prev = candles.at(-2);
  const chartChange = last && prev ? last.close - prev.close : 0;
  const chartChangePercent = last && prev && prev.close !== 0 ? (chartChange / prev.close) * 100 : 0;

  const displayName = quoteData?.description || symbol;
  // Price and change always come from `candles` — the exact same array the
  // chart renders — so this can never drift from what's on screen. Only
  // fall back to the one-shot quote fetch before any candle has loaded.
  const price = last?.close ?? quoteData?.lastPrice ?? 0;
  const change = last && prev ? chartChange : (quoteData?.netChange ?? 0);
  const changePercent = last && prev ? chartChangePercent : (quoteData?.percentChange ?? 0);
  const negative = change < 0;

  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-3 flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold">{symbol}</h1>
          <p className="text-[12px] text-nest-muted">{displayName}</p>
          <div className="mt-1 flex items-center gap-3 text-[12px] text-nest-muted">
            <button
              type="button"
              className="flex items-center gap-1 hover:text-nest-foreground"
              title="Add to Watchlist"
            >
              <Plus className="size-3" />
              <span>Add to Watchlist</span>
              <ChevronDown className="size-3" />
            </button>
            <button
              type="button"
              className="flex items-center gap-1 hover:text-nest-foreground"
              title="Create Alert"
            >
              <Bell className="size-3" />
              <span>Create Alert</span>
            </button>
            <button
              type="button"
              className="flex items-center gap-1 hover:text-nest-foreground"
              title="Grid"
            >
              <LayoutGrid className="size-3" />
              <span>Grid</span>
              <ChevronDown className="size-3" />
            </button>
          </div>
        </div>
        {last ? (
          <div className="flex items-start gap-4">
            <div className="text-right">
              <p
                className={`text-xl font-semibold ${negative ? "text-nest-error" : "text-nest-success"}`}
              >
                {price?.toFixed(2) || (last ? last.close.toFixed(2) : "—")}
              </p>
              <p className={`text-[12px] ${negative ? "text-nest-error" : "text-nest-success"}`}>
                {change >= 0 ? "+" : ""}
                {change.toFixed(2)} ({changePercent >= 0 ? "+" : ""}
                {changePercent.toFixed(2)}%)
              </p>
            </div>
            <div className="flex items-center gap-3 text-[12px]">
              <div className="text-right leading-tight">
                <p className="text-nest-muted">Bid size: {quoteData?.bidSize || "—"}</p>
                <p className="text-nest-muted">Ask size: {quoteData?.askSize || "—"}</p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  className="rounded-nest-md bg-nest-error px-4 py-1.5 text-[13px] font-medium text-white hover:opacity-90"
                >
                  Sell
                </button>
                <button
                  type="button"
                  className="rounded-nest-md bg-nest-success px-4 py-1.5 text-[13px] font-medium text-white hover:opacity-90"
                >
                  Buy
                </button>
              </div>
            </div>
          </div>
        ) : null}
      </div>

      <div className="mb-3 flex items-center gap-4 border-b border-nest-border pb-2 text-[12px] text-nest-muted">
        <button
          type="button"
          onClick={() => setStudiesOpen(true)}
          className="hover:text-nest-foreground"
        >
          Studies
        </button>
        <button type="button" className="cursor-not-allowed opacity-50" title="Not implemented yet">
          Drawings
        </button>
        <span className="flex-1" />
        <Select
          value={period}
          onChange={onPeriodChange}
          options={PERIOD_OPTIONS}
          size="small"
          className="!w-fit shrink-0"
        />
        <Select
          value={aggregation}
          onChange={onAggregationChange}
          options={intervalOptions}
          size="small"
          className="!w-fit shrink-0"
        />
        <button
          type="button"
          onClick={() => setSettingsOpen(true)}
          className="hover:text-nest-foreground"
        >
          Settings
        </button>
      </div>

      <div className="relative min-h-0 flex-1 rounded-nest-md border border-nest-border">
        {candlesLoading && (
          <div className="absolute inset-0 z-10 flex items-center justify-center bg-nest-surface/50 text-[12px] text-nest-muted">
            Loading chart…
          </div>
        )}
        {candlesError && (
          <div className="absolute left-2 top-2 z-10 rounded-nest-md bg-nest-error/10 px-2 py-1 text-[11px] text-nest-error">
            Chart data unavailable — showing placeholder
          </div>
        )}
        <CandlestickChart data={candles} studies={studies} patterns={patterns} />
      </div>

      <StudiesDialog
        open={studiesOpen}
        onClose={() => setStudiesOpen(false)}
        active={studies}
        onToggle={onToggleStudy}
      />
      <ChartSettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}
