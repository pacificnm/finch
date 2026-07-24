import { useEffect, useMemo, useState } from "react";
import { Select, type SelectOption } from "@nest/components";
import { Bell, Plus } from "lucide-react";
import { CandlestickChart, type ActiveStudies, type ChartPattern } from "./CandlestickChart";
import { StudiesDialog } from "./StudiesDialog";
import { ChartSettingsDialog } from "./ChartSettingsDialog";
import { QuoteDetails } from "./QuoteDetails";
import { TradesTable } from "./TradesTable";
import { OrderTicket } from "./OrderTicket";
import {
  getDefaultIntervalForPeriod,
  getIntervalOptionsForPeriod,
  type IntervalValue,
} from "../../lib/chartIntervals";
import { generateMockCandles } from "../../lib/mockOhlc";
import { fetchPriceHistory, fetchQuote, type OhlcvData } from "../../lib/nest";
import type { TradeSetup } from "./OrderTicket";

export const MOCK_SYMBOL = "SCHG";

type QuoteData = {
  description?: string;
  lastPrice?: number;
  netChange?: number;
  percentChange?: number;
  bidSize?: string;
  askSize?: string;
};

type TradeScreenProps = {
  /** Symbol to display. Defaults to MOCK_SYMBOL when not provided. */
  symbol?: string;
  /** Optional AI-generated trade setup to populate the order ticket. */
  tradeSetup?: TradeSetup | null;
  /** Called when the user clears the AI trade setup from the order ticket. */
  onClearTradeSetup?: () => void;
  /** Which chart studies are on — shared with the Charts screen and the AI chat panel. */
  studies: ActiveStudies;
  /** Toggles one chart study on/off. */
  onToggleStudy: (key: keyof ActiveStudies) => void;
  /** AI-detected chart pattern overlays — shared with the Charts screen and the AI chat panel. */
  patterns?: ChartPattern[];
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

const HEADER_TABS = [
  { value: "quote", label: "Quote Details" },
  { value: "analyst", label: "Analyst Reports" },
  { value: "fundamentals", label: "Fundamentals" },
  { value: "optionStats", label: "Option Stats" },
];

export function TradeScreen({
  symbol = MOCK_SYMBOL,
  tradeSetup,
  onClearTradeSetup,
  studies,
  onToggleStudy,
  patterns = [],
}: TradeScreenProps) {
  const [activeHeaderTab, setActiveHeaderTab] = useState("quote");
  const [period, setPeriod] = useState("1y");
  const [aggregation, setAggregation] = useState("1d");
  const [studiesOpen, setStudiesOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [currentSymbol, setCurrentSymbol] = useState(symbol);
  const [quoteData, setQuoteData] = useState<QuoteData | null>(null);
  const [candles, setCandles] = useState<OhlcvData[]>([]);
  const [candlesLoading, setCandlesLoading] = useState(false);
  const [candlesError, setCandlesError] = useState(false);

  const intervalOptions = useMemo(() => getIntervalOptionsForPeriod(period), [period]);

  useEffect(() => {
    const validValues = new Set(intervalOptions.map((option) => option.value));
    setAggregation((current) => {
      if (!validValues.has(current as IntervalValue)) {
        return getDefaultIntervalForPeriod(period);
      }
      return current;
    });
  }, [period, intervalOptions]);

  // Fetch real price history when symbol, period, or interval changes.
  useEffect(() => {
    if (!currentSymbol) return;

    let cancelled = false;

    const loadCandles = async () => {
      setCandlesLoading(true);
      setCandlesError(false);
      try {
        const data = await fetchPriceHistory(currentSymbol, period, aggregation);
        if (!cancelled) {
          if (data.length > 0) {
            setCandles(data);
          } else {
            // Fall back to mock data if the API returns nothing usable.
            const days = PERIOD_OPTIONS.find((option) => option.value === period)?.days ?? 365;
            setCandles(generateMockCandles(days, 30, 7));
          }
        }
      } catch (error) {
        console.error(`[TradeScreen] Failed to fetch price history for ${currentSymbol}:`, error);
        if (!cancelled) {
          setCandlesError(true);
          const days = PERIOD_OPTIONS.find((option) => option.value === period)?.days ?? 365;
          setCandles(generateMockCandles(days, 30, 7));
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
  }, [currentSymbol, period, aggregation]);

  const last = candles.at(-1);
  const prev = candles.at(-2);
  const chartChange = last && prev ? last.close - prev.close : 0;
  const chartChangePercent = last && prev && prev.close !== 0 ? (chartChange / prev.close) * 100 : 0;

  // Fetch quote data when symbol changes
  useEffect(() => {
    if (!currentSymbol) return;
    
    let cancelled = false;
    
    const loadQuote = async () => {
      try {
        const data = await fetchQuote(currentSymbol);
        if (!cancelled) {
          setQuoteData(data);
        }
      } catch (error) {
        console.error(`[TradeScreen] Failed to fetch quote for ${currentSymbol}:`, error);
        if (!cancelled) {
          setQuoteData({
            description: "Failed to load",
          });
        }
      }
    };
    
    loadQuote();
    
    return () => {
      cancelled = true;
    };
  }, [currentSymbol]);

  // Update currentSymbol when prop changes
  if (symbol !== currentSymbol) {
    setCurrentSymbol(symbol);
  }

  const displayName = quoteData?.description || currentSymbol;
  const price = quoteData?.lastPrice ?? (last?.close || 0);
  const quoteChange = quoteData?.netChange ?? chartChange;
  const quoteChangePercent = quoteData?.percentChange ?? chartChangePercent;
  const negative = quoteChange < 0;

  return (
    <div className="flex h-full flex-col gap-3 overflow-y-auto p-4">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold">{currentSymbol}</h1>
          <p className="text-[12px] text-nest-muted">{displayName}</p>
          <div className="mt-1 flex items-center gap-3 text-[12px] text-nest-muted">
            <button
              type="button"
              className="flex items-center gap-1 hover:text-nest-foreground"
              title="Add to Watchlist"
            >
              <Plus className="size-3" />
              <span>Add to Watchlist</span>
            </button>
            <button
              type="button"
              className="flex items-center gap-1 hover:text-nest-foreground"
              title="Create Alert"
            >
              <Bell className="size-3" />
              <span>Create Alert</span>
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
                {quoteChange >= 0 ? "+" : ""}
                {quoteChange.toFixed(2)} ({quoteChangePercent >= 0 ? "+" : ""}
                {quoteChangePercent.toFixed(2)}%)
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

      <div className="flex items-center gap-1 border-b border-nest-border pb-2 text-[12px]">
        {HEADER_TABS.map((tab) => (
          <button
            key={tab.value}
            type="button"
            onClick={() => setActiveHeaderTab(tab.value)}
            className={`rounded-nest-md px-3 py-1.5 font-medium ${
              activeHeaderTab === tab.value
                ? "bg-nest-muted/20 text-nest-foreground"
                : "text-nest-muted hover:text-nest-foreground"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {activeHeaderTab === "quote" ? (
        <>
          <QuoteDetails symbol={currentSymbol} />
          <TradesTable />
        </>
      ) : (
        <div className="flex h-32 items-center justify-center rounded-nest-md border border-nest-border text-[12px] text-nest-muted">
          {HEADER_TABS.find((tab) => tab.value === activeHeaderTab)?.label} coming next.
        </div>
      )}

      <div className="flex min-h-0 flex-1 flex-col rounded-nest-md border border-nest-border">
        <div className="flex items-center gap-4 border-b border-nest-border px-3 py-2 text-[12px] text-nest-muted">
          <button
            type="button"
            onClick={() => setStudiesOpen(true)}
            className="hover:text-nest-foreground"
          >
            Studies
          </button>
          <button
            type="button"
            className="cursor-not-allowed opacity-50"
            title="Not implemented yet"
          >
            Drawings
          </button>
          <span className="flex-1" />
          <Select
            value={period}
            onChange={setPeriod}
            options={PERIOD_OPTIONS}
            size="small"
            className="!w-fit shrink-0"
          />
          <Select
            value={aggregation}
            onChange={setAggregation}
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
        <div className="relative min-h-0 flex-1">
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
      </div>

        <OrderTicket symbol={currentSymbol} tradeSetup={tradeSetup} onClearTradeSetup={onClearTradeSetup} />

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
