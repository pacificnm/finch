import { useState } from "react";
import { IconRail, type WorkspaceSection } from "./IconRail";
import { AccountPanel } from "./AccountPanel";
import { PositionsPanel } from "./PositionsPanel";
import { ChartsScreen } from "./ChartsScreen";

const SECTION_LABEL: Record<WorkspaceSection, string> = {
  positions: "Positions",
  trade: "Trade",
  charts: "Charts",
  scans: "Scans",
};

const ACCOUNT_LABEL = "Day Trading *000";

/**
 * The trading workspace: icon rail | account panel | section content | right
 * positions rail (hidden on the Positions section itself, since the main
 * content already shows the full positions view there — matches the
 * reference: only Trade/Charts/Scans keep the right rail docked).
 */
export function TradingWorkspace() {
  const [section, setSection] = useState<WorkspaceSection>("charts");

  return (
    <div className="flex h-full min-h-0">
      <IconRail active={section} onSelect={setSection} />
      <AccountPanel />

      <div className="min-h-0 min-w-0 flex-1 overflow-y-auto">
        {section === "positions" ? (
          <PositionsPanel accountLabel={ACCOUNT_LABEL} variant="full" />
        ) : section === "charts" ? (
          <ChartsScreen />
        ) : (
          <div className="flex h-full flex-col items-center justify-center gap-2 text-nest-muted">
            <p className="text-sm font-medium">{SECTION_LABEL[section]}</p>
            <p className="text-xs">Structure coming next.</p>
          </div>
        )}
      </div>

      {section !== "positions" ? (
        <div className="w-72 shrink-0 border-l border-nest-border bg-nest-surface">
          <PositionsPanel accountLabel={ACCOUNT_LABEL} variant="compact" />
        </div>
      ) : null}
    </div>
  );
}
