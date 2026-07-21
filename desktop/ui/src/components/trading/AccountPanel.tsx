import { Card, CardContent, CardHeader } from "@nest/components";

export type AccountSummaryData = {
  accountLabel: string;
  accountValue: string;
  buyingPower: string;
  cashForWithdrawal: string;
  plDayPercent: string;
};

export type WatchlistRow = {
  symbol: string;
  mark: string;
  changePercent: string;
  negative: boolean;
};

const MOCK_ACCOUNT: AccountSummaryData = {
  accountLabel: "Day Trading *000",
  accountValue: "$0.00",
  buyingPower: "$0.00",
  cashForWithdrawal: "$0.00",
  plDayPercent: "0%",
};

const MOCK_WATCHLIST: WatchlistRow[] = [
  { symbol: "SCHG", mark: "34.15", changePercent: "-0.09%", negative: true },
];

/** Persistent left panel: Account Summary + Watchlist. Present on every section. */
export function AccountPanel() {
  return (
    <div className="flex w-60 shrink-0 flex-col gap-3 overflow-y-auto border-r border-nest-border bg-nest-surface p-3">
      <Card variant="outlined" elevation={0}>
        <CardHeader title="Account Summary" className="pb-2" />
        <CardContent className="pt-0">
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
          <div className="mb-3 rounded-nest-md border border-nest-border px-2 py-1.5 text-[12px] font-medium">
            {MOCK_ACCOUNT.accountLabel}
          </div>
          <dl className="space-y-1.5 text-[12px]">
            <SummaryRow label="Account Value" value={MOCK_ACCOUNT.accountValue} />
            <SummaryRow label="Buying Power" value={MOCK_ACCOUNT.buyingPower} />
            <SummaryRow label="Cash for Withdrawal" value={MOCK_ACCOUNT.cashForWithdrawal} />
            <SummaryRow label="P/L Day %" value={MOCK_ACCOUNT.plDayPercent} />
          </dl>
        </CardContent>
      </Card>

      <Card variant="outlined" elevation={0} className="flex min-h-0 flex-1 flex-col">
        <CardHeader title="Watchlist" className="pb-2" />
        <CardContent className="flex-1 overflow-y-auto pt-0">
          <table className="w-full text-[11px]">
            <thead>
              <tr className="text-left text-nest-muted">
                <th className="pb-1 font-medium">Symbol</th>
                <th className="pb-1 font-medium">Mark</th>
                <th className="pb-1 text-right font-medium">Chg %</th>
              </tr>
            </thead>
            <tbody>
              {MOCK_WATCHLIST.map((row) => (
                <tr key={row.symbol} className="border-t border-nest-border">
                  <td className="py-1 font-medium">{row.symbol}</td>
                  <td className="py-1">{row.mark}</td>
                  <td
                    className={`py-1 text-right ${row.negative ? "text-nest-error" : "text-nest-success"}`}
                  >
                    {row.changePercent}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </CardContent>
      </Card>
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
