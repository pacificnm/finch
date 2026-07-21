import { Settings } from "lucide-react";

export type QuoteMetric = {
  label: string;
  value: string;
};

const MOCK_QUOTE_METRICS: QuoteMetric[] = [
  { label: "Volume", value: "7,210,276" },
  { label: "Beta", value: "1.1955" },
  { label: "Market Cap", value: "59.739M" },
  { label: "Yield", value: "0.39%" },
  { label: "VWAP", value: "34.276" },
  { label: "IV", value: "17.39%" },
  { label: "PE", value: "—" },
  { label: "Div Amount", value: "0.0337" },
  { label: "50-Day Avg Volume", value: "10,520,301" },
  { label: "HV", value: "14.86%" },
  { label: "EPS", value: "—" },
  { label: "Ex Date", value: "06/24/26" },
  { label: "Gain Per Tick", value: "—" },
];

export function QuoteDetails() {
  return (
    <div className="rounded-nest-md border border-nest-border bg-nest-surface p-3">
      <div className="grid grid-cols-5 gap-x-4 gap-y-2 text-[11px]">
        {MOCK_QUOTE_METRICS.map((metric) => (
          <div key={metric.label} className="flex items-center justify-between">
            <span className="text-nest-muted">{metric.label}</span>
            <span className="font-medium">{metric.value}</span>
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
