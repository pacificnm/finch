import { useState } from "react";
import { Card, CardContent, CardHeader, Tab, TabPanel, Tabs } from "@nest/components";

export type PositionRow = {
  position: string;
  qty: string;
  plDay: string;
  plOpen: string;
  netLiq: string;
};

const MOCK_POSITIONS: PositionRow[] = [
  { position: "Cash", qty: "—", plDay: "—", plOpen: "—", netLiq: "$0.00" },
];

type ActivityTab = "working" | "filled" | "canceled";

type PositionsPanelProps = {
  accountLabel: string;
  /** "full": main content when Positions is the active section. "compact": persistent right rail on other sections. */
  variant?: "compact" | "full";
};

/**
 * Activity (Working/Filled/Canceled orders) + Positions table. Used both as
 * the main content on the Positions section and as the persistent right
 * rail everywhere else — same content, matching the reference layout.
 */
export function PositionsPanel({ accountLabel, variant = "compact" }: PositionsPanelProps) {
  const [activityTab, setActivityTab] = useState<ActivityTab>("working");

  return (
    <div
      className={`flex h-full flex-col gap-3 overflow-y-auto ${variant === "full" ? "p-4" : "p-3"}`}
    >
      <h2 className="text-[13px] font-semibold">{accountLabel} Positions</h2>

      <Card variant="outlined" elevation={0}>
        <CardHeader title="Activity" className="pb-2" />
        <CardContent className="pt-0">
          <Tabs value={activityTab} onChange={(value) => setActivityTab(value as ActivityTab)}>
            <Tab value="working" label={<TabLabel label="Working" count={0} />} />
            <Tab value="filled" label={<TabLabel label="Filled" count={0} />} />
            <Tab value="canceled" label="Canceled" />
          </Tabs>
          <TabPanel value="working">
            <p className="py-4 text-center text-[12px] text-nest-muted">
              You do not currently have any working orders.
            </p>
          </TabPanel>
          <TabPanel value="filled">
            <p className="py-4 text-center text-[12px] text-nest-muted">No filled orders.</p>
          </TabPanel>
          <TabPanel value="canceled">
            <p className="py-4 text-center text-[12px] text-nest-muted">No canceled orders.</p>
          </TabPanel>
        </CardContent>
      </Card>

      <Card variant="outlined" elevation={0}>
        <CardHeader title="Positions" className="pb-2" />
        <CardContent className="overflow-x-auto pt-0">
          <table className="w-full min-w-max text-[11px]">
            <thead>
              <tr className="text-left text-nest-muted">
                <th className="pb-1 pr-4 font-medium">Position</th>
                <th className="pb-1 pr-4 font-medium">Qty</th>
                <th className="pb-1 pr-4 font-medium">P/L Day $</th>
                <th className="pb-1 pr-4 font-medium">P/L Open $</th>
                <th className="pb-1 font-medium">Net Liq</th>
              </tr>
            </thead>
            <tbody>
              {MOCK_POSITIONS.map((row) => (
                <tr key={row.position} className="border-t border-nest-border">
                  <td className="py-1 pr-4 font-medium">{row.position}</td>
                  <td className="py-1 pr-4">{row.qty}</td>
                  <td className="py-1 pr-4">{row.plDay}</td>
                  <td className="py-1 pr-4">{row.plOpen}</td>
                  <td className="py-1">{row.netLiq}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </CardContent>
      </Card>
    </div>
  );
}

function TabLabel({ label, count }: { label: string; count: number }) {
  return (
    <span className="flex items-center gap-1.5">
      {label}
      <span className="rounded-full bg-nest-muted/20 px-1.5 py-0.5 text-[10px] leading-none text-nest-muted">
        {count}
      </span>
    </span>
  );
}
