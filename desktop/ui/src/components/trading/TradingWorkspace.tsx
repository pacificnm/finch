import { useEffect, useMemo, useState } from "react";
import { IconRail, type WorkspaceSection } from "./IconRail";
import { AccountPanel } from "./AccountPanel";
import { PositionsPanel } from "./PositionsPanel";
import { ChartsScreen } from "./ChartsScreen";
import { TradeScreen } from "./TradeScreen";
import { ResizeHandle } from "./ResizeHandle";
import {
  fetchSchwabAccountSummary,
  fetchSchwabAccounts,
  fetchSchwabOrders,
  fetchSchwabPositions,
  type SchwabAccount,
  type SchwabAccountSummary,
  type SchwabOrderRow,
  type SchwabPositionRow,
} from "../../lib/nest";
import { SettingKeys, Settings } from "../../lib/settings";
import { useToast } from "../../shell";

const SECTION_LABEL: Record<WorkspaceSection, string> = {
  positions: "Positions",
  trade: "Trade",
  charts: "Charts",
  scans: "Scans",
};

const MIN_ACCOUNT_WIDTH = 180;
const DEFAULT_ACCOUNT_WIDTH = 240;
const MIN_POSITIONS_WIDTH = 280;
const DEFAULT_POSITIONS_WIDTH = 360;
const MAX_POSITIONS_WIDTH = 600;

function formatAccountLabel(account: SchwabAccount): string {
  const display = account.display_account_id ?? account.account_number;
  if (account.nickname) {
    return `${account.nickname} ${display}`;
  }
  return display;
}

/**
 * The trading workspace: icon rail | account panel | section content | right
 * positions rail (hidden on the Positions section itself, since the main
 * content already shows the full positions view there — matches the
 * reference: only Trade/Charts/Scans keep the right rail docked).
 */
export function TradingWorkspace() {
  const [section, setSection] = useState<WorkspaceSection>("positions");
  const [accountWidth, setAccountWidth] = useState(DEFAULT_ACCOUNT_WIDTH);
  const [positionsWidth, setPositionsWidth] = useState(DEFAULT_POSITIONS_WIDTH);
  const [accounts, setAccounts] = useState<SchwabAccount[]>([]);
  const [selectedHash, setSelectedHash] = useState<string>("");
  const [accountsLoading, setAccountsLoading] = useState(true);
  const [summary, setSummary] = useState<SchwabAccountSummary>({
    account_value: "$0.00",
    buying_power: "$0.00",
    cash_for_withdrawal: "$0.00",
    pl_day_percent: "0.00%",
  });
  const [orders, setOrders] = useState<SchwabOrderRow[]>([]);
  const [positions, setPositions] = useState<SchwabPositionRow[]>([]);
  const toast = useToast();

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const loaded = await fetchSchwabAccounts();
        if (cancelled) return;
        setAccounts(loaded);
        let saved = "";
        try {
          saved = await Settings.getString(SettingKeys.defaultAccountHash, "");
        } catch {
          // Settings may be unavailable on first run before migrations apply.
          saved = "";
        }
        const match = loaded.find((a) => a.hash === saved);
        setSelectedHash(match?.hash ?? loaded[0]?.hash ?? "");
      } catch (error: unknown) {
        // eslint-disable-next-line no-console
        console.error("[accounts] failed to load accounts:", error);
        toast.error(`Accounts failed: ${String(error)}`);
      } finally {
        if (!cancelled) setAccountsLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!selectedHash) return;
    let cancelled = false;
    void (async () => {
      try {
        // eslint-disable-next-line no-console
        console.log("[summary] fetching for hash", selectedHash);
        const data = await fetchSchwabAccountSummary(selectedHash);
        // eslint-disable-next-line no-console
        console.log("[summary] received", data);
        if (!cancelled) setSummary(data);
      } catch (error: unknown) {
        // eslint-disable-next-line no-console
        console.error("[summary] failed", error);
        toast.error(`Account summary failed: ${String(error)}`);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedHash]);

  useEffect(() => {
    if (!selectedHash) return;
    let cancelled = false;
    void (async () => {
      try {
        const data = await fetchSchwabOrders(selectedHash);
        if (!cancelled) setOrders(data);
      } catch (error: unknown) {
        toast.error(`Orders failed: ${String(error)}`);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedHash]);

  useEffect(() => {
    if (!selectedHash) return;
    let cancelled = false;
    void (async () => {
      try {
        const data = await fetchSchwabPositions(selectedHash);
        if (!cancelled) setPositions(data);
      } catch (error: unknown) {
        toast.error(`Positions failed: ${String(error)}`);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedHash]);

  const selectedAccount = useMemo(
    () => accounts.find((a) => a.hash === selectedHash),
    [accounts, selectedHash],
  );

  const handleSelectAccount = async (hash: string) => {
    setSelectedHash(hash);
    await Settings.setString(SettingKeys.defaultAccountHash, hash);
  };

  const accountLabel = selectedAccount
    ? formatAccountLabel(selectedAccount)
    : accountsLoading
      ? "Loading…"
      : "No account";

  const handleAccountResize = (delta: number) => {
    setAccountWidth((current) => Math.max(MIN_ACCOUNT_WIDTH, current + delta));
  };

  const handlePositionsResize = (delta: number) => {
    // The handle is to the left of the right panel, so dragging left
    // (negative delta) should widen the panel and vice versa.
    setPositionsWidth((current) =>
      Math.min(MAX_POSITIONS_WIDTH, Math.max(MIN_POSITIONS_WIDTH, current - delta))
    );
  };

  return (
    <div className="flex h-full min-h-0">
      <IconRail active={section} onSelect={setSection} />
      <div
        className="shrink-0 border-r border-nest-border bg-nest-surface"
        style={{ width: accountWidth }}
      >
        <AccountPanel
          accounts={accounts}
          selectedHash={selectedHash}
          summary={summary}
          onSelectAccount={handleSelectAccount}
        />
      </div>
      <ResizeHandle onResize={handleAccountResize} label="Resize account panel" />

      <div className="min-h-0 min-w-0 flex-1 overflow-y-auto">
        {section === "positions" ? (
          <PositionsPanel
            accountLabel={accountLabel}
            orders={orders}
            positions={positions}
            variant="full"
          />
        ) : section === "charts" ? (
          <ChartsScreen />
        ) : section === "trade" ? (
          <TradeScreen />
        ) : (
          <div className="flex h-full flex-col items-center justify-center gap-2 text-nest-muted">
            <p className="text-sm font-medium">{SECTION_LABEL[section]}</p>
            <p className="text-xs">Structure coming next.</p>
          </div>
        )}
      </div>

      {section !== "positions" ? (
        <>
          <ResizeHandle onResize={handlePositionsResize} label="Resize positions panel" />
          <div
            className="shrink-0 border-l border-nest-border bg-nest-surface"
            style={{ width: positionsWidth }}
          >
            <PositionsPanel
              accountLabel={accountLabel}
              orders={orders}
              positions={positions}
              variant="compact"
            />
          </div>
        </>
      ) : null}
    </div>
  );
}
