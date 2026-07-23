import { useEffect, useState } from "react";
import { Settings } from "lucide-react";
import { fetchQuote, type QuoteData } from "../../lib/nest";

export type QuoteMetric = {
  label: string;
  value: string;
};

type QuoteDetailsProps = {
  symbol: string;
};

function formatNumber(num?: number | null, decimals = 2): string {
  if (num == null || Number.isNaN(num)) return "—";
  return num.toLocaleString(undefined, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

function formatVolume(num?: number | null): string {
  if (num == null || Number.isNaN(num)) return "—";
  const abs = Math.abs(num);
  if (abs >= 1e12) return `${(num / 1e12).toFixed(2)}T`;
  if (abs >= 1e9) return `${(num / 1e9).toFixed(2)}B`;
  if (abs >= 1e6) return `${(num / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${(num / 1e3).toFixed(2)}K`;
  return num.toLocaleString();
}

// Market cap is displayed in millions to match thinkorswim (e.g. 878,282M).
function formatMarketCap(num?: number | null): string {
  if (num == null || Number.isNaN(num)) return "—";
  const millions = num / 1e6;
  return `${millions.toLocaleString(undefined, { maximumFractionDigits: 0 })}M`;
}

// MMM is displayed as a +/- expected move (e.g. ±22.04).
function formatMmm(num?: number | null): string {
  if (num == null || Number.isNaN(num)) return "—";
  return `±${Math.abs(num).toFixed(2)}`;
}

function formatPercent(num?: number | null, decimals = 2): string {
  if (num == null || Number.isNaN(num)) return "—";
  return `${num.toFixed(decimals)}%`;
}

// Schwab may return percentages as decimals (0.0055 = 0.55%) or as percent values (0.55 = 0.55%).
// Treat values <= 1 as decimals and convert them to percent values.
function normalizePercent(num: number): number {
  return num > 1 ? num : num * 100;
}

function buildMetrics(data: QuoteData | null): QuoteMetric[] {
  if (!data) {
    return [
      { label: "Volume", value: "—" },
      { label: "Beta", value: "—" },
      { label: "Market Cap", value: "—" },
      { label: "Yield", value: "—" },
      { label: "VWAP", value: "—" },
      { label: "IV", value: "—" },
      { label: "PE", value: "—" },
      { label: "Div Amount", value: "—" },
      { label: "MMM", value: "—" },
      { label: "50-Day Avg Volume", value: "—" },
      { label: "HV", value: "—" },
      { label: "EPS", value: "—" },
      { label: "Ex Date", value: "—" },
      { label: "Earnings", value: "—" },
    ];
  }

  const dividendYield =
    data.dividendYield != null && data.dividendYield > 0
      ? formatPercent(normalizePercent(data.dividendYield))
      : "—";

  const divAmount =
    data.divAmount != null && data.divAmount > 0
      ? formatNumber(data.divAmount, 2)
      : "—";

  return [
    // Row 1
    { label: "Volume", value: formatVolume(data.volume) },
    { label: "Beta", value: data.beta != null ? data.beta.toFixed(4) : "—" },
    { label: "Market Cap", value: formatMarketCap(data.marketCap) },
    { label: "Yield", value: dividendYield },
    { label: "VWAP", value: formatNumber(data.vwap, 3) },

    // Row 2
    { label: "IV", value: data.iv != null ? formatPercent(normalizePercent(data.iv)) : "—" },
    { label: "PE", value: formatNumber(data.peRatio, 2) },
    { label: "Div Amount", value: divAmount },
    { label: "MMM", value: data.mmm != null ? formatMmm(data.mmm) : "—" },
    { label: "50-Day Avg Volume", value: formatVolume(data.avg50DayVolume) },

    // Row 3
    { label: "HV", value: data.hv != null ? formatPercent(normalizePercent(data.hv)) : "—" },
    { label: "EPS", value: formatNumber(data.eps, 2) },
    { label: "Ex Date", value: data.exDate || "—" },
    { label: "Earnings", value: data.earningsDate || "—" },
  ];
}

export function QuoteDetails({ symbol }: QuoteDetailsProps) {
  const [metrics, setMetrics] = useState<QuoteMetric[]>(buildMetrics(null));
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!symbol) {
      setMetrics(buildMetrics(null));
      setError(false);
      return;
    }

    let cancelled = false;

    const loadQuote = async () => {
      try {
        setError(false);
        const data = await fetchQuote(symbol);
        if (!cancelled) {
          setMetrics(buildMetrics(data));
        }
      } catch (err) {
        console.error("[QuoteDetails] Failed to fetch quote:", err);
        if (!cancelled) {
          setError(true);
          setMetrics(buildMetrics(null));
        }
      }
    };

    loadQuote();

    return () => {
      cancelled = true;
    };
  }, [symbol]);

  return (
    <div className="rounded-nest-md border border-nest-border bg-nest-surface p-3">
      <div className="grid grid-cols-5 gap-x-4 gap-y-2 text-[11px]">
        {metrics.map((metric) => (
          <div key={metric.label} className="flex items-center justify-between">
            <span className="text-nest-muted">{metric.label}</span>
            <span className={`font-medium ${error ? "text-nest-error" : ""}`}>
              {metric.value}
            </span>
          </div>
        ))}
        <button
          type="button"
          className="flex items-center justify-center gap-1 rounded-nest-md border border-nest-border py-1 text-nest-muted hover:text-nest-foreground"
        >
          <Settings className="size-3" />
          Customize
        </button>
      </div>
    </div>
  );
}
