import { ChevronDown, Layers } from "lucide-react";

const STRATEGIES = [
  { label: "Option Trade", active: true },
  { label: "Covered Stock", active: false },
  { label: "Vertical", active: false },
  { label: "Iron Condor", active: false },
];

export function OptionChain() {
  return (
    <div className="rounded-nest-md border border-nest-border bg-nest-surface p-3">
      <h3 className="mb-2 flex items-center gap-1.5 text-[12px] font-semibold">
        <Layers className="size-3.5" />
        Option Chain
      </h3>
      <div className="flex flex-wrap items-center gap-2 text-[11px]">
        {STRATEGIES.map((strategy) => (
          <button
            key={strategy.label}
            type="button"
            className={`flex items-center gap-1 rounded-nest-md px-2 py-1 font-medium ${
              strategy.active
                ? "bg-nest-primary text-white"
                : "border border-nest-border text-nest-foreground hover:bg-nest-muted/10"
            }`}
          >
            {strategy.active ? <span className="text-[10px]">+</span> : null}
            {strategy.label}
          </button>
        ))}
        <button
          type="button"
          className="flex items-center gap-1 rounded-nest-md border border-nest-border px-2 py-1 font-medium text-nest-foreground hover:bg-nest-muted/10"
        >
          Strategy
          <ChevronDown className="size-3" />
        </button>
      </div>
    </div>
  );
}
