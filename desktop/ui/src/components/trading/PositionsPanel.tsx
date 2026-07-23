import { useMemo, useState } from "react";
import { Briefcase, Clock, Settings, Wallet } from "lucide-react";
import type { SchwabOrderRow, SchwabPositionRow } from "../../lib/nest";

type ActivityTab = "working" | "filled" | "canceled";

type PositionsPanelProps = {
  accountLabel: string;
  orders: SchwabOrderRow[];
  positions: SchwabPositionRow[];
  /** "full": main content when Positions is the active section. "compact": persistent right rail on other sections. */
  variant?: "compact" | "full";
};

const ACTIVITY_COLUMNS = [
  "Time",
  "Side",
  "Pos Effect",
  "Qty",
  "Amount",
  "Symbol",
  "Desc",
  "Price",
  "TIF",
  "Mark",
  "Net Prc",
  "Status",
];

const POSITION_COLUMNS = [
  "Position",
  "Qty",
  "P/L Day $",
  "P/L Open $",
  "P/L YTD $",
  "Cost",
  "Net Liq",
  "Trade Price",
  "BP Effect",
  "Delta (A)",
  "Gamma (T)",
  "Theta (O)",
  "Vega (v)",
];

/**
 * Activity (Working/Filled/Canceled orders) + Positions table. Used both as
 * the main content on the Positions section and as the persistent right
 * rail everywhere else — same content, matching the reference layout.
 */
function statusCategory(status: string): ActivityTab {
  const upper = status.toUpperCase();
  if (["FILLED", "PARTIAL"].includes(upper)) return "filled";
  if (["CANCELED", "REJECTED", "EXPIRED"].includes(upper)) return "canceled";
  return "working";
}

export function PositionsPanel({
  accountLabel,
  orders,
  positions,
  variant = "compact",
}: PositionsPanelProps) {
  const [activityTab, setActivityTab] = useState<ActivityTab>("working");

  const positionRows = positions.length > 0 ? positions : [];

  const workingOrders = useMemo(
    () => orders.filter((o) => statusCategory(o.status) === "working"),
    [orders],
  );
  const filledOrders = useMemo(
    () => orders.filter((o) => statusCategory(o.status) === "filled"),
    [orders],
  );
  const canceledOrders = useMemo(
    () => orders.filter((o) => statusCategory(o.status) === "canceled"),
    [orders],
  );

  return (
    <div
      className={`flex h-full flex-col gap-3 overflow-y-auto ${variant === "full" ? "p-4" : "p-3"}`}
    >
      <h2 className="flex items-center gap-2 text-[13px] font-semibold">
        <Briefcase className="size-4" />
        <span>{accountLabel} Positions</span>
      </h2>

      <div>
        <div className="mb-2 flex items-center justify-between">
          <h3 className="flex items-center gap-1.5 text-[12px] font-semibold">
            <Clock className="size-3.5" />
            Activity
          </h3>
          {variant === "full" ? (
            <button
              type="button"
              className="text-[11px] text-nest-primary hover:underline"
            >
              View Transaction History »
            </button>
          ) : null}
        </div>
        <div className="border-b border-nest-border">
          <div className="flex">
            <ActivityTabButton
              label="Working"
              count={workingOrders.length}
              selected={activityTab === "working"}
              onClick={() => setActivityTab("working")}
            />
            <ActivityTabButton
              label="Filled"
              count={filledOrders.length}
              selected={activityTab === "filled"}
              onClick={() => setActivityTab("filled")}
            />
            <ActivityTabButton
              label="Canceled"
              count={canceledOrders.length}
              selected={activityTab === "canceled"}
              onClick={() => setActivityTab("canceled")}
            />
          </div>
        </div>
        {activityTab === "working" && (
          <ActivityTable
            orders={workingOrders}
            emptyMessage="You do not currently have any working orders."
          />
        )}
        {activityTab === "filled" && (
          <ActivityTable orders={filledOrders} emptyMessage="No filled orders." />
        )}
        {activityTab === "canceled" && (
          <ActivityTable orders={canceledOrders} emptyMessage="No canceled orders." />
        )}
      </div>

      <div>
        <div className="flex items-center justify-between pb-2">
          <h3 className="text-[12px] font-semibold">Positions</h3>
          <button
            type="button"
            className="text-nest-muted hover:text-nest-foreground"
            title="Positions settings"
          >
            <Settings className="size-3.5" />
          </button>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full min-w-max text-[11px]">
            <thead>
              <tr className="text-left text-nest-muted">
                {POSITION_COLUMNS.map((column) => (
                  <th key={column} className="pb-1 pr-3 font-medium last:pr-0">
                    {column}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {positionRows.length === 0 ? (
                <tr className="border-t border-nest-border">
                  <td className="py-4 pr-3 text-center text-nest-muted" colSpan={POSITION_COLUMNS.length}>
                    No positions.
                  </td>
                </tr>
              ) : (
                positionRows.map((row) => {
                  const isTotals = row.position === "Totals:";
                  const weight = isTotals ? "font-semibold" : "font-medium";
                  return (
                    <tr key={row.position} className="border-t border-nest-border">
                      <td className={`py-1 pr-3 ${weight}`}>{row.position}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.qty}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.pl_day}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.pl_open}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.pl_ytd}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.cost}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.net_liq}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.trade_price}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.bp_effect}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.delta}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.gamma}</td>
                      <td className={`py-1 pr-3 ${isTotals ? "font-semibold" : ""}`}>{row.theta}</td>
                      <td className={isTotals ? "py-1 font-semibold" : "py-1"}>{row.vega}</td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      <div>
        <h3 className="flex items-center gap-1.5 pb-2 text-[12px] font-semibold">
          <Wallet className="size-3.5" />
          Portfolio Digest
        </h3>
        <p className="text-center text-[12px] text-nest-muted">No portfolio digest available.</p>
      </div>
    </div>
  );
}

function ActivityTable({
  orders,
  emptyMessage,
}: {
  orders: SchwabOrderRow[];
  emptyMessage: string;
}) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-max text-[11px]">
        <thead>
          <tr className="text-left text-nest-muted">
            <th className="pb-1 pr-3 font-medium">
              <input type="checkbox" className="size-3 accent-nest-primary" aria-label="Select all" />
            </th>
            {ACTIVITY_COLUMNS.map((column) => (
              <th key={column} className="pb-1 pr-3 font-medium last:pr-0">
                {column}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {orders.length === 0 ? (
            <tr className="border-t border-nest-border">
              <td className="py-4 pr-3" colSpan={ACTIVITY_COLUMNS.length + 1}>
                <p className="text-center text-nest-muted">{emptyMessage}</p>
              </td>
            </tr>
          ) : (
            orders.map((order) => {
              const side = order.side.toUpperCase();
              const sideClass = side.includes("SELL")
                ? "text-nest-error"
                : side.includes("BUY")
                  ? "text-nest-success"
                  : "";
              return (
              <tr key={order.order_id} className="border-t border-nest-border">
                <td className="py-1 pr-3">
                  <input type="checkbox" className="size-3 accent-nest-primary" aria-label={`Select order ${order.order_id}`} />
                </td>
                <td className="py-1 pr-3">{order.time}</td>
                <td className={`py-1 pr-3 font-medium ${sideClass}`}>{order.side}</td>
                <td className={`py-1 pr-3 font-medium ${sideClass}`}>{order.pos_effect}</td>
                <td className="py-1 pr-3">{order.qty}</td>
                <td className="py-1 pr-3">{order.amount}</td>
                <td className="py-1 pr-3 font-medium">{order.symbol}</td>
                <td className="py-1 pr-3">{order.desc}</td>
                <td className="py-1 pr-3">{order.price}</td>
                <td className="py-1 pr-3">{order.tif}</td>
                <td className="py-1 pr-3">{order.mark}</td>
                <td className="py-1 pr-3">{order.net_prc}</td>
                <td className="py-1">{order.status}</td>
              </tr>
              );
            })
          )}
        </tbody>
      </table>
    </div>
  );
}

function ActivityTabButton({
  label,
  count,
  selected,
  onClick,
}: {
  label: string;
  count: number;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex items-center gap-1.5 px-3 py-2 text-[11px] font-medium transition-colors ${
        selected
          ? "border-b-2 border-nest-primary text-nest-primary"
          : "border-b-2 border-transparent text-nest-muted hover:text-nest-foreground"
      }`}
    >
      {label}
      <span className="rounded-full bg-nest-muted/20 px-1.5 py-0.5 text-[10px] leading-none text-nest-muted">
        {count}
      </span>
    </button>
  );
}
