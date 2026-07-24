import { useMemo, type ReactNode } from "react";
import { Select, type SelectOption } from "@nest/components";
import { X } from "lucide-react";
import { useTimezone } from "../context/TimezoneContext";
import { listTimezones, systemTimezone, TIMEZONE_AUTO } from "../lib/timezone";

type SettingsScreenProps = {
  onClose: () => void;
};

/**
 * Full-page settings view (File → Settings). Grouped sections so more
 * settings can be added later without restructuring — currently just
 * Display → Timezone.
 */
export function SettingsScreen({ onClose }: SettingsScreenProps) {
  const { raw, setTimezone } = useTimezone();

  const timezoneOptions = useMemo<SelectOption[]>(() => {
    const auto: SelectOption = { value: TIMEZONE_AUTO, label: `Automatic (${systemTimezone()})` };
    const rest = listTimezones().map((tz): SelectOption => ({ value: tz, label: tz }));
    return [auto, ...rest];
  }, []);

  return (
    <div className="flex h-full flex-col overflow-y-auto p-6">
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-lg font-semibold">Settings</h1>
        <button
          type="button"
          onClick={onClose}
          className="rounded-nest-md p-1 text-nest-muted hover:bg-nest-muted/10 hover:text-nest-foreground"
          title="Close settings"
        >
          <X className="size-4" />
        </button>
      </div>

      <div className="flex max-w-xl flex-col gap-6">
        <SettingsGroup title="Display">
          <SettingsRow
            label="Timezone"
            description="Chart times (e.g. Schwab's Eastern/market time) are shown converted to this timezone."
          >
            <Select
              value={raw}
              onChange={setTimezone}
              options={timezoneOptions}
              size="small"
              className="!w-56"
            />
          </SettingsRow>
        </SettingsGroup>
      </div>
    </div>
  );
}

function SettingsGroup({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h2 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-nest-muted">
        {title}
      </h2>
      <div className="divide-y divide-nest-border rounded-nest-md border border-nest-border bg-nest-surface">
        {children}
      </div>
    </section>
  );
}

function SettingsRow({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="text-[13px] font-medium text-nest-foreground">{label}</p>
        {description ? <p className="mt-0.5 text-[11px] text-nest-muted">{description}</p> : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
