import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { SettingKeys, Settings } from "../lib/settings";
import { resolveTimezone, TIMEZONE_AUTO } from "../lib/timezone";

type TimezoneContextValue = {
  /** The setting as saved — an IANA identifier, or "auto". */
  raw: string;
  /** `raw` resolved to a concrete IANA identifier (system timezone when "auto"). */
  timezone: string;
  /** Saves a new display timezone; pass `TIMEZONE_AUTO` to follow the system. */
  setTimezone: (value: string) => void;
};

const TimezoneContext = createContext<TimezoneContextValue | null>(null);

/** Loads/persists `display.timezone` and makes the resolved zone available app-wide. */
export function TimezoneProvider({ children }: { children: ReactNode }) {
  const [raw, setRaw] = useState(TIMEZONE_AUTO);

  useEffect(() => {
    void (async () => {
      try {
        const saved = await Settings.getString(SettingKeys.displayTimezone, "");
        if (saved) {
          setRaw(saved);
        }
      } catch {
        // Settings may be unavailable on first run before migrations apply.
      }
    })();
  }, []);

  const setTimezone = (value: string) => {
    setRaw(value);
    void Settings.setString(SettingKeys.displayTimezone, value).catch(() => {});
  };

  return (
    <TimezoneContext.Provider value={{ raw, timezone: resolveTimezone(raw), setTimezone }}>
      {children}
    </TimezoneContext.Provider>
  );
}

export function useTimezone(): TimezoneContextValue {
  const context = useContext(TimezoneContext);
  if (!context) {
    throw new Error("useTimezone must be used within TimezoneProvider");
  }
  return context;
}
