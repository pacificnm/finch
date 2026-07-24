import { useEffect, useMemo, useState } from "react";
import { IconRail, type WorkspaceSection } from "./IconRail";
import { AccountPanel } from "./AccountPanel";
import { PositionsPanel } from "./PositionsPanel";
import { AiChatPanel } from "./AiChatPanel";
import { type ActiveStudies, type ChartPattern } from "./CandlestickChart";
import { ChartsScreen } from "./ChartsScreen";
import { TradeScreen, MOCK_SYMBOL } from "./TradeScreen";
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
import { getDefaultIntervalForPeriod, getIntervalOptionsForPeriod, type IntervalValue } from "../../lib/chartIntervals";
import { useToast } from "../../shell";
import type { TradeSetup } from "./OrderTicket";

const SECTION_LABEL: Record<WorkspaceSection, string> = {
  positions: "Positions",
  trade: "Trade",
  charts: "Charts",
  scans: "Scans",
};

const DEFAULT_STUDIES: ActiveStudies = {
  volume: false,
  movingAverage: false,
  rsi: false,
  macd: false,
  atr: false,
  vwap: false,
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

type TradingWorkspaceProps = {
  /** Initial symbol to display in the Trade screen. */
  symbol?: string;
};

/**
 * The trading workspace: icon rail | account panel | section content | right
 * positions rail (hidden on the Positions section itself, since the main
 * content already shows the full positions view there — matches the
 * reference: only Trade/Charts/Scans keep the right rail docked).
 */
export function TradingWorkspace({ symbol }: TradingWorkspaceProps) {
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
  const [tradeSetup, setTradeSetup] = useState<TradeSetup | null>(null);
  // Shared across Trade/Charts so the AI chat panel can control whichever
  // chart is currently on screen, and so a study toggled on one screen
  // stays on when you switch to the other.
  const [studies, setStudies] = useState<ActiveStudies>(DEFAULT_STUDIES);
  // AI-detected chart pattern overlays, shared the same way `studies` is.
  // Unlike studies, these are cleared on symbol change below — a leftover
  // overlay computed from a different symbol's prices would be actively
  // misleading rather than just a harmless toggle left on.
  const [patterns, setPatterns] = useState<ChartPattern[]>([]);
  // Chart period/interval, shared across Trade/Charts the same way `studies`
  // is, and persisted below so the app reopens where it was left.
  const [period, setPeriod] = useState("1y");
  const [aggregation, setAggregation] = useState("1d");
  const toast = useToast();

  const toggleStudy = (key: keyof ActiveStudies) => {
    setStudies((current) => {
      const next = { ...current, [key]: !current[key] };
      void Settings.setJson(SettingKeys.chartStudies, next).catch(() => {});
      return next;
    });
  };

  const applyChartStudies = (partial: Partial<ActiveStudies>) => {
    setStudies((current) => {
      const next = { ...current, ...partial };
      void Settings.setJson(SettingKeys.chartStudies, next).catch(() => {});
      return next;
    });
  };

  const applyChartPatterns = (next: ChartPattern[]) => {
    setPatterns(next);
  };

  const handlePeriodChange = (value: string) => {
    setPeriod(value);
    void Settings.setString(SettingKeys.chartPeriod, value).catch(() => {});
  };

  const handleAggregationChange = (value: string) => {
    setAggregation(value);
    void Settings.setString(SettingKeys.chartInterval, value).catch(() => {});
  };

  useEffect(() => {
    setPatterns([]);
  }, [symbol]);

  // Restore the last chart period/interval/studies from the previous
  // session. Runs once on mount — the account/theme/symbol settings each
  // load the same way, elsewhere. Period and interval are applied together
  // (not as they each resolve) so the interval-clamp effect below never
  // sees a saved period paired with the still-default interval, which
  // would otherwise clamp and persist the wrong value before the real
  // saved interval arrives.
  useEffect(() => {
    void (async () => {
      let savedStudies: Partial<ActiveStudies> | null = null;
      let savedPeriod = "";
      let savedInterval = "";
      try {
        savedStudies = (await Settings.getJson<ActiveStudies>(SettingKeys.chartStudies)) ?? null;
      } catch {
        // Settings may be unavailable on first run before migrations apply.
      }
      try {
        savedPeriod = await Settings.getString(SettingKeys.chartPeriod, "");
      } catch {
        // Settings may be unavailable on first run before migrations apply.
      }
      try {
        savedInterval = await Settings.getString(SettingKeys.chartInterval, "");
      } catch {
        // Settings may be unavailable on first run before migrations apply.
      }

      if (savedStudies) {
        setStudies((current) => ({ ...current, ...savedStudies }));
      }
      if (savedPeriod) {
        setPeriod(savedPeriod);
      }
      if (savedInterval) {
        setAggregation(savedInterval);
      }
    })();
  }, []);

  // Keep the interval valid for whichever period is selected (e.g. minute
  // intervals aren't offered for a 1-year period) — mirrors the per-screen
  // effect this replaced, now centralized since period/interval are shared.
  useEffect(() => {
    const validValues = new Set(getIntervalOptionsForPeriod(period).map((option) => option.value));
    setAggregation((current) => {
      if (!validValues.has(current as IntervalValue)) {
        const next = getDefaultIntervalForPeriod(period);
        void Settings.setString(SettingKeys.chartInterval, next).catch(() => {});
        return next;
      }
      return current;
    });
  }, [period]);

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
          <ChartsScreen
            symbol={symbol}
            studies={studies}
            onToggleStudy={toggleStudy}
            patterns={patterns}
            period={period}
            onPeriodChange={handlePeriodChange}
            aggregation={aggregation}
            onAggregationChange={handleAggregationChange}
          />
        ) : section === "trade" ? (
          <TradeScreen
            symbol={symbol}
            tradeSetup={tradeSetup}
            onClearTradeSetup={() => setTradeSetup(null)}
            period={period}
            onPeriodChange={handlePeriodChange}
            aggregation={aggregation}
            onAggregationChange={handleAggregationChange}
            studies={studies}
            onToggleStudy={toggleStudy}
            patterns={patterns}
          />
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
            {section === "trade" || section === "charts" ? (
              <AiChatPanel
                symbol={symbol ?? MOCK_SYMBOL}
                onTradeSetup={setTradeSetup}
                onChartStudies={applyChartStudies}
                onChartPatterns={applyChartPatterns}
              />
            ) : (
              <PositionsPanel
                accountLabel={accountLabel}
                orders={orders}
                positions={positions}
                variant="compact"
              />
            )}
          </div>
        </>
      ) : null}
    </div>
  );
}
