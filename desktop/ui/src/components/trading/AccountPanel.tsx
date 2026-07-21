import { useState } from "react";
import { ChevronDown, Plus, Settings } from "lucide-react";
import { Select, type SelectOption } from "@nest/components";
import type { SchwabAccount, SchwabAccountSummary } from "../../lib/nest";

export type WatchlistRow = {
  symbol: string;
  bid: string;
  ask: string;
  changePercent: string;
  negative: boolean;
};

const MOCK_WATCHLIST: WatchlistRow[] = [
  { symbol: "SCHG", bid: "34.12", ask: "34.23", changePercent: "-0.09%", negative: true },
];

type AccountPanelProps = {
  accounts: SchwabAccount[];
  selectedHash: string;
  summary: SchwabAccountSummary;
  onSelectAccount: (hash: string) => void;
};

function accountOption(account: SchwabAccount): SelectOption {
  const display = account.display_account_id ?? account.account_number;
  const label = account.nickname
    ? `${account.nickname} ${display}`
    : display;
  return {
    value: account.hash,
    label,
  };
}

/** Persistent left panel: Account Summary + Watchlist. Present on every section. */
export function AccountPanel({
  accounts,
  selectedHash,
  summary,
  onSelectAccount,
}: AccountPanelProps) {
  const [selectedSymbol, setSelectedSymbol] = useState<string>(MOCK_WATCHLIST[0]?.symbol ?? "");
  const accountOptions = accounts.map(accountOption);

  return (
    <div className="flex h-full flex-col gap-3 overflow-y-auto p-3">
      <div>
        <h3 className="pb-2 text-[12px] font-semibold">Account Summary</h3>
        <div className="mb-3 flex overflow-hidden rounded-nest-md border border-nest-border text-[11px] font-medium">
          <button type="button" className="flex-1 bg-nest-primary py-1 text-white">
            Live Trading
          </button>
          <button
            type="button"
            className="flex-1 py-1 text-nest-muted hover:text-nest-foreground"
          >
            paperMoney
          </button>
        </div>
        <div className="mb-3">
          <Select
            value={selectedHash}
            onChange={onSelectAccount}
            options={accountOptions}
            placeholder="Select account"
            size="small"
            className="w-full"
          />
        </div>
        <dl className="space-y-1.5 text-[12px]">
          <SummaryRow label="Account Value" value={summary.account_value} />
          <SummaryRow label="Buying Power" value={summary.buying_power} />
          <SummaryRow label="Cash for Withdrawal" value={summary.cash_for_withdrawal} />
          <SummaryRow label="P/L Day %" value={summary.pl_day_percent} />
        </dl>
      </div>

      <div className="flex min-h-0 flex-1 flex-col">
        <h3 className="pb-2 text-[12px] font-semibold">Watchlist</h3>
        <div className="mb-2 flex items-center justify-between">
          <button
            type="button"
            className="flex items-center gap-1 text-[11px] font-medium text-nest-foreground hover:text-nest-primary"
          >
            All Account Positions
            <ChevronDown className="size-3" />
          </button>
          <button
            type="button"
            className="flex items-center gap-1 text-[11px] font-medium text-nest-foreground hover:text-nest-primary"
          >
            <Plus className="size-3" />
            New Watchlist
          </button>
        </div>
        <div className="flex-1 overflow-y-auto">
          <table className="w-full text-[11px]">
            <thead>
              <tr className="text-left text-nest-muted">
                <th className="pb-1 font-medium">Symbol</th>
                <th className="pb-1 font-medium">Bid</th>
                <th className="pb-1 font-medium">Ask</th>
                <th className="pb-1 text-right font-medium">Chg %</th>
                <th className="pb-1 text-right font-medium">
                  <Settings className="size-3" />
                </th>
              </tr>
            </thead>
            <tbody>
              {MOCK_WATCHLIST.map((row) => {
                const selected = row.symbol === selectedSymbol;
                return (
                  <tr
                    key={row.symbol}
                    onClick={() => setSelectedSymbol(row.symbol)}
                    className={`cursor-pointer border-t border-nest-border ${
                      selected ? "bg-nest-primary/10" : "hover:bg-nest-muted/10"
                    }`}
                  >
                    <td className="py-1 font-medium">{row.symbol}</td>
                    <td className="py-1">{row.bid}</td>
                    <td className="py-1">{row.ask}</td>
                    <td
                      className={`py-1 text-right ${
                        row.negative ? "text-nest-error" : "text-nest-success"
                      }`}
                    >
                      {row.changePercent}
                    </td>
                    <td className="py-1 text-right">
                      <Settings className="size-3 text-nest-muted" />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between">
      <dt className="text-nest-muted">{label}</dt>
      <dd className="font-medium">{value}</dd>
    </div>
  );
}
