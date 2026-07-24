import { useMemo, useState } from "react";
import { Dialog, TextField } from "@nest/components";
import { Icon } from "../Icon";
import { faMagnifyingGlass, faPlus } from "../../lib/fontawesome";
import type { ActiveStudies } from "./CandlestickChart";

type StudyKey = keyof ActiveStudies;

type PopularStudy = {
  key: StudyKey | null;
  label: string;
};

// Matches the reference "Popular Studies" grid. Only Volume, Moving Average,
// RSI, MACD, ATR, and VWAP are wired to the chart so far (key !== null); the
// rest are shown (for layout parity with thinkorswim) but disabled rather
// than silently doing nothing on click — a disabled button is honest, a
// no-op button pretending to work is exactly the kind of thing that reads as
// broken.
const POPULAR_STUDIES: PopularStudy[] = [
  { key: null, label: "ADX" },
  { key: "atr", label: "ATR" },
  { key: null, label: "Awesome Oscillator" },
  { key: null, label: "CCI" },
  { key: null, label: "Comparison" },
  { key: null, label: "Comparison - Perc..." },
  { key: null, label: "DMI" },
  { key: null, label: "Keltner Channels" },
  { key: null, label: "Linear Regression" },
  { key: "macd", label: "MACD" },
  { key: "movingAverage", label: "Moving Average" },
  { key: null, label: "Open Interest" },
  { key: null, label: "Pivot Points" },
  { key: null, label: "Price Channel" },
  { key: null, label: "Probability Of Expi..." },
  { key: "rsi", label: "RSI" },
  { key: null, label: "StdDev Channel" },
  { key: null, label: "Stochastic" },
  { key: null, label: "TRIX" },
  { key: "volume", label: "Volume Profile" },
  { key: "vwap", label: "VWAP" },
];

const UPPER_STUDY_LABEL: Record<StudyKey, string> = {
  volume: "Volume",
  movingAverage: "Moving Average",
  rsi: "RSI",
  macd: "MACD",
  atr: "ATR",
  vwap: "VWAP",
};

type StudiesDialogProps = {
  open: boolean;
  onClose: () => void;
  active: ActiveStudies;
  onToggle: (key: StudyKey) => void;
};

export function StudiesDialog({ open, onClose, active, onToggle }: StudiesDialogProps) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) {
      return POPULAR_STUDIES;
    }
    return POPULAR_STUDIES.filter((study) => study.label.toLowerCase().includes(needle));
  }, [query]);

  const activeKeys = (Object.keys(active) as StudyKey[]).filter((key) => active[key]);

  return (
    <Dialog open={open} onClose={onClose} title="Studies">
      <div className="w-[600px] max-w-full">
        <TextField
          placeholder="Find a study"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          startAdornment={<Icon icon={faMagnifyingGlass} className="text-nest-muted" />}
          className="mb-4 w-full"
        />

        <div className="grid grid-cols-[minmax(0,160px)_1fr] gap-6">
          <div>
            <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-nest-muted">
              Upper Studies
            </h3>
            {activeKeys.length === 0 ? (
              <p className="text-[12px] text-nest-muted">None added.</p>
            ) : (
              <ul className="space-y-1.5">
                {activeKeys.map((key) => (
                  <li
                    key={key}
                    className="flex items-center justify-between rounded-nest-md border border-nest-border px-2 py-1.5 text-[12px]"
                  >
                    <span>{UPPER_STUDY_LABEL[key]}</span>
                    <button
                      type="button"
                      className="text-nest-muted hover:text-nest-error"
                      aria-label={`Remove ${UPPER_STUDY_LABEL[key]}`}
                      onClick={() => onToggle(key)}
                    >
                      ×
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div>
            <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-nest-muted">
              Popular Studies
            </h3>
            <div className="grid grid-cols-2 gap-2">
              {filtered.map((study) => {
                const isActive = study.key !== null && active[study.key];
                const disabled = study.key === null;
                return (
                  <button
                    key={study.label}
                    type="button"
                    disabled={disabled}
                    title={disabled ? "Not implemented yet" : undefined}
                    onClick={() => study.key && onToggle(study.key)}
                    className={`flex items-center gap-2 rounded-nest-md border px-2.5 py-1.5 text-left text-[12px] ${
                      disabled
                        ? "cursor-not-allowed border-nest-border/50 text-nest-muted/50"
                        : isActive
                          ? "border-nest-success text-nest-success"
                          : "border-nest-border hover:border-nest-primary hover:text-nest-primary"
                    }`}
                  >
                    <Icon
                      icon={faPlus}
                      className={isActive ? "text-nest-success" : "text-current"}
                    />
                    <span className="truncate">{study.label}</span>
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </Dialog>
  );
}
