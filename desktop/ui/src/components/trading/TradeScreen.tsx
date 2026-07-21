import { useEffect, useMemo, useState } from "react";
import { Select, type SelectOption } from "@nest/components";
import { Bell, Plus } from "lucide-react";
import { CandlestickChart, type ActiveStudies } from "./CandlestickChart";
import { StudiesDialog } from "./StudiesDialog";
import { ChartSettingsDialog } from "./ChartSettingsDialog";
import { QuoteDetails } from "./QuoteDetails";
import { OptionChain } from "./OptionChain";
import { TradesTable } from "./TradesTable";
import { OrderTicket } from "./OrderTicket";
import {
  getDefaultIntervalForPeriod,
  getIntervalOptionsForPeriod,
  type IntervalValue,
} from "../../lib/chartIntervals";
import { aggregateWeekly, generateMockCandles } from "../../lib/mockOhlc";

const MOCK_SYMBOL = "SCHG";
const MOCK_NAME = "Schwab US Large-Cap Growth ETF";

const MOCK_BID_ASK = {
  bid: 34.12,
  ask: 34.17,
  bidSize: "2.3K",
  askSize: "3.9K",
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

const DEFAULT_STUDIES: ActiveStudies = { volume: false, movingAverage: false, rsi: false };

export function TradeScreen() {
  const [activeHeaderTab, setActiveHeaderTab] = useState("quote");
  const [period, setPeriod] = useState("1y");
  const [aggregation, setAggregation] = useState("1d");
  const [studiesOpen, setStudiesOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [studies, setStudies] = useState<ActiveStudies>(DEFAULT_STUDIES);

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

  const days = PERIOD_OPTIONS.find((option) => option.value === period)?.days ?? 365;

  const candles = useMemo(() => {
    const daily = generateMockCandles(days, 30, 7);
    return aggregation === "1w" ? aggregateWeekly(daily) : daily;
  }, [days, aggregation]);

  const last = candles.at(-1);
  const prev = candles.at(-2);
  const change = last && prev ? last.close - prev.close : 0;
  const changePercent = last && prev && prev.close !== 0 ? (change / prev.close) * 100 : 0;
  const negative = change < 0;

  const toggleStudy = (key: keyof ActiveStudies) => {
    setStudies((current) => ({ ...current, [key]: !current[key] }));
  };

  return (
    <div className="flex h-full flex-col gap-3 overflow-y-auto p-4">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold">{MOCK_SYMBOL}</h1>
          <p className="text-[12px] text-nest-muted">{MOCK_NAME}</p>
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
                {last.close.toFixed(2)}
              </p>
              <p className={`text-[12px] ${negative ? "text-nest-error" : "text-nest-success"}`}>
                {change >= 0 ? "+" : ""}
                {change.toFixed(2)} ({changePercent >= 0 ? "+" : ""}
                {changePercent.toFixed(2)}%)
              </p>
            </div>
            <div className="flex items-center gap-3 text-[12px]">
              <div className="text-right leading-tight">
                <p className="text-nest-muted">Bid size: {MOCK_BID_ASK.bidSize}</p>
                <p className="text-nest-muted">Ask size: {MOCK_BID_ASK.askSize}</p>
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
          <QuoteDetails />
          <OptionChain />
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
        <div className="min-h-0 flex-1">
          <CandlestickChart data={candles} studies={studies} />
        </div>
      </div>

      <OrderTicket symbol={MOCK_SYMBOL} />

      <StudiesDialog
        open={studiesOpen}
        onClose={() => setStudiesOpen(false)}
        active={studies}
        onToggle={toggleStudy}
      />
      <ChartSettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}
