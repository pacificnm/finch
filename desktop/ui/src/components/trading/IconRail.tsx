import { Icon } from "../Icon";
import { faBullseye, faChartLine, faFileLines, faSackDollar } from "../../lib/fontawesome";
import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";

export type WorkspaceSection = "positions" | "trade" | "charts" | "scans";

const SECTIONS: { id: WorkspaceSection; label: string; icon: IconDefinition }[] = [
  { id: "positions", label: "Positions", icon: faSackDollar },
  { id: "trade", label: "Trade", icon: faFileLines },
  { id: "charts", label: "Charts", icon: faChartLine },
  { id: "scans", label: "Scans", icon: faBullseye },
];

type IconRailProps = {
  active: WorkspaceSection;
  onSelect: (section: WorkspaceSection) => void;
};

/** Thin left-hand section rail: Positions / Trade / Charts / Scans. */
export function IconRail({ active, onSelect }: IconRailProps) {
  return (
    <nav className="flex w-14 shrink-0 flex-col items-stretch border-r border-nest-border bg-nest-surface py-2">
      {SECTIONS.map((section) => {
        const selected = section.id === active;
        return (
          <button
            key={section.id}
            type="button"
            onClick={() => onSelect(section.id)}
            aria-current={selected ? "page" : undefined}
            className={`flex flex-col items-center gap-1 px-1 py-2.5 text-[10px] font-medium transition-colors ${
              selected ? "text-nest-primary" : "text-nest-muted hover:text-nest-foreground"
            }`}
          >
            <Icon icon={section.icon} className="text-base" />
            <span>{section.label}</span>
          </button>
        );
      })}
    </nav>
  );
}
