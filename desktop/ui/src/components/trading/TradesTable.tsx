import { useState } from "react";
import { Settings, Triangle } from "lucide-react";

export type TradeRow = {
  id: string;
  description: string;
  side: "buy" | "sell";
  mark: string;
  markChange: string;
  plDay: string;
  plOpen: string;
  netLiq: string;
  bpEffect: string;
  tradePrice: string;
  cost: string;
  delta: string;
  gamma: string;
  theta: string;
};

type TradeTab = "all" | "positions" | "working" | "lotDetails";

const TRADE_TABS: { value: TradeTab; label: string; icon?: "positions" | "working" }[] = [
  { value: "all", label: "All" },
  { value: "positions", label: "Positions", icon: "positions" },
  { value: "working", label: "Working", icon: "working" },
  { value: "lotDetails", label: "Lot Details" },
];

const MOCK_TRADES: TradeRow[] = [
  {
    id: "1",
    description: "Buy 1 SCHG @34.17 LIMIT Day",
    side: "buy",
    mark: "—",
    markChange: "—",
    plDay: "$0.00",
    plOpen: "$0.00",
    netLiq: "$0.00",
    bpEffect: "$0.00",
    tradePrice: "—",
    cost: "$0.00",
    delta: "0.00",
    gamma: "0.00",
    theta: "0.00",
  },
];

const COLUMNS = [
  "Trade",
  "Mark",
  "Mark Chng $",
  "P/L Day $",
  "P/L Open $",
  "Net Liq",
  "BP Effect",
  "Trade Price",
  "Cost",
  "Delta (A)",
  "Gamma (T)",
  "Theta (O)",
];

export function TradesTable() {
  const [activeTab, setActiveTab] = useState<TradeTab>("all");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set(["1"]));

  const toggleRow = (id: string) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  return (
    <div className="rounded-nest-md border border-nest-border bg-nest-surface">
      <div className="flex items-center gap-1 border-b border-nest-border p-2 text-[11px]">
        {TRADE_TABS.map((tab) => (
          <button
            key={tab.value}
            type="button"
            onClick={() => setActiveTab(tab.value)}
            className={`flex items-center gap-1.5 rounded-nest-md px-3 py-1.5 font-medium ${
              activeTab === tab.value
                ? "bg-nest-muted/20 text-nest-foreground"
                : "text-nest-muted hover:text-nest-foreground"
            }`}
          >
            {tab.icon ? <TabIcon type={tab.icon} /> : null}
            {tab.label}
          </button>
        ))}
      </div>

      <div className="overflow-x-auto">
        <table className="w-full min-w-max text-[11px]">
          <thead>
            <tr className="text-left text-nest-muted">
              {COLUMNS.map((column) => (
                <th key={column} className="px-2 py-1 font-medium last:pr-2">
                  {column === "Trade" ? (
                    <span className="flex items-center gap-1">
                      {column}
                      <Settings className="size-3" />
                    </span>
                  ) : (
                    column
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {MOCK_TRADES.map((row) => {
              const selected = selectedIds.has(row.id);
              return (
                <tr
                  key={row.id}
                  onClick={() => toggleRow(row.id)}
                  className={`cursor-pointer border-t border-nest-border ${
                    selected ? "bg-nest-primary/10" : "hover:bg-nest-muted/10"
                  }`}
                >
                  <td className="px-2 py-1">
                    <span className="flex items-center gap-1.5">
                      <input
                        type="checkbox"
                        checked={selected}
                        onChange={() => toggleRow(row.id)}
                        className="size-3 accent-nest-primary"
                      />
                      <Triangle
                        className={`size-3 ${
                          row.side === "buy" ? "text-nest-success" : "text-nest-error rotate-180"
                        }`}
                      />
                      <span className="font-medium">{row.description}</span>
                    </span>
                  </td>
                  <td className="px-2 py-1">{row.mark}</td>
                  <td className="px-2 py-1">{row.markChange}</td>
                  <td className="px-2 py-1">{row.plDay}</td>
                  <td className="px-2 py-1">{row.plOpen}</td>
                  <td className="px-2 py-1">{row.netLiq}</td>
                  <td className="px-2 py-1">{row.bpEffect}</td>
                  <td className="px-2 py-1">{row.tradePrice}</td>
                  <td className="px-2 py-1">{row.cost}</td>
                  <td className="px-2 py-1">{row.delta}</td>
                  <td className="px-2 py-1">{row.gamma}</td>
                  <td className="px-2 py-1 pr-2">{row.theta}</td>
                </tr>
              );
            })}
            <tr className="border-t border-nest-border font-medium">
              <td className="px-2 py-1">Selected Totals:</td>
              <td className="px-2 py-1">—</td>
              <td className="px-2 py-1">0.00</td>
              <td className="px-2 py-1">$0.00</td>
              <td className="px-2 py-1">$0.00</td>
              <td className="px-2 py-1">$0.00</td>
              <td className="px-2 py-1">$0.00</td>
              <td className="px-2 py-1">—</td>
              <td className="px-2 py-1">$0.00</td>
              <td className="px-2 py-1">0.00</td>
              <td className="px-2 py-1">0.00</td>
              <td className="px-2 py-1 pr-2">0.00</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div className="border-t border-nest-border px-2 py-1 text-[11px] font-medium">
        {selectedIds.size} Selected
      </div>
    </div>
  );
}

function TabIcon({ type }: { type: "positions" | "working" }) {
  return (
    <span className="flex flex-col gap-px">
      <span className={`h-px w-2 ${type === "positions" ? "bg-nest-success" : "bg-nest-primary"}`} />
      <span className={`h-px w-2 ${type === "positions" ? "bg-nest-success" : "bg-nest-primary"}`} />
    </span>
  );
}
