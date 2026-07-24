import { invoke } from "@tauri-apps/api/core";

/** Valid setting value types stored in PostgreSQL. */
export type SettingValue =
  | { type: "string"; value: string }
  | { type: "integer"; value: number }
  | { type: "float"; value: number }
  | { type: "boolean"; value: boolean }
  | { type: "json"; value: unknown };

/** Known Finch setting keys. */
export const SettingKeys = {
  themeId: "theme.id",
  defaultAccountHash: "account.default_hash",
  chartPeriod: "chart.period",
  chartInterval: "chart.interval",
  chartStudies: "chart.studies",
  chartSymbol: "chart.symbol",
  displayTimezone: "display.timezone",
} as const;

/** Fetches a single setting by key. Returns `null` when absent. */
export async function getSetting(key: string): Promise<SettingValue | null> {
  return invoke("plugin:finch|settings_get", { key });
}

/** Persists a single setting. */
export async function setSetting(
  key: string,
  value: SettingValue,
): Promise<void> {
  return invoke("plugin:finch|settings_set", { key, value });
}

/** Convenience helpers for typed settings. */
export const Settings = {
  async getString(key: string, fallback = ""): Promise<string> {
    const v = await getSetting(key);
    return v?.type === "string" ? v.value : fallback;
  },
  async getInteger(key: string, fallback = 0): Promise<number> {
    const v = await getSetting(key);
    return v?.type === "integer" ? v.value : fallback;
  },
  async getFloat(key: string, fallback = 0): Promise<number> {
    const v = await getSetting(key);
    return v?.type === "float" ? v.value : fallback;
  },
  async getBoolean(key: string, fallback = false): Promise<boolean> {
    const v = await getSetting(key);
    return v?.type === "boolean" ? v.value : fallback;
  },
  async getJson<T = unknown>(key: string, fallback?: T): Promise<T | undefined> {
    const v = await getSetting(key);
    return v?.type === "json" ? (v.value as T) : fallback;
  },
  setString(key: string, value: string): Promise<void> {
    return setSetting(key, { type: "string", value });
  },
  setInteger(key: string, value: number): Promise<void> {
    return setSetting(key, { type: "integer", value });
  },
  setFloat(key: string, value: number): Promise<void> {
    return setSetting(key, { type: "float", value });
  },
  setBoolean(key: string, value: boolean): Promise<void> {
    return setSetting(key, { type: "boolean", value });
  },
  setJson(key: string, value: unknown): Promise<void> {
    return setSetting(key, { type: "json", value });
  },
} as const;
