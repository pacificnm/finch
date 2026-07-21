import { useMemo, useState } from "react";
import { Select, type SelectOption } from "@nest/components";
import { CandlestickChart, type ActiveStudies } from "./CandlestickChart";
import { StudiesDialog } from "./StudiesDialog";
import { ChartSettingsDialog } from "./ChartSettingsDialog";
import { aggregateWeekly, generateMockCandles } from "../../lib/mockOhlc";

const MOCK_SYMBOL = "SCHG";
const MOCK_NAME = "Schwab US Large-Cap Growth ETF";

const PERIOD_OPTIONS: (SelectOption & { days: number })[] = [
  { value: "1m", label: "1M", days: 30 },
  { value: "3m", label: "3M", days: 90 },
  { value: "6m", label: "6M", days: 180 },
  { value: "1y", label: "1Y", days: 365 },
  { value: "2y", label: "2Y", days: 730 },
];

const INTERVAL_OPTIONS: SelectOption[] = [
  { value: "1d", label: "1D" },
  { value: "1w", label: "1W" },
];

const DEFAULT_STUDIES: ActiveStudies = { volume: false, movingAverage: false, rsi: false };

/** Charts section: symbol header, chart toolbar, and the candlestick chart. */
export function ChartsScreen() {
  const [period, setPeriod] = useState("1y");
  const [aggregation, setAggregation] = useState("1d");
  const [studiesOpen, setStudiesOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [studies, setStudies] = useState<ActiveStudies>(DEFAULT_STUDIES);

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
    <div className="flex h-full flex-col p-4">
      <div className="mb-3 flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold">{MOCK_SYMBOL}</h1>
          <p className="text-[12px] text-nest-muted">{MOCK_NAME}</p>
        </div>
        {last ? (
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
        {/*
          Select's trigger button has `w-full` baked in. Used directly as a
          flex-row child (no wrapper), a `width: 100%` on a flex item becomes
          its flex-basis (CSS spec: flex-basis:auto resolves to the width
          property when set) — so the item's *preferred* size is "the whole
          row," and flexbox's proportional shrink-to-fit still leaves it far
          wider than its siblings. `shrink-0` on a wrapper with an explicit
          width fixes the flex-basis at that width instead, so the `w-full`
          inside resolves against a normal (non-flex) 4rem containing block.
        */}
        <div className="w-16 shrink-0">
          <Select value={period} onChange={setPeriod} options={PERIOD_OPTIONS} size="small" />
        </div>
        <div className="w-16 shrink-0">
          <Select
            value={aggregation}
            onChange={setAggregation}
            options={INTERVAL_OPTIONS}
            size="small"
          />
        </div>
        <button
          type="button"
          onClick={() => setSettingsOpen(true)}
          className="hover:text-nest-foreground"
        >
          Settings
        </button>
      </div>

      <div className="min-h-0 flex-1 rounded-nest-md border border-nest-border">
        <CandlestickChart data={candles} studies={studies} />
      </div>

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
