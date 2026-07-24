/** Setting value meaning "follow the system timezone" rather than a fixed one. */
export const TIMEZONE_AUTO = "auto";

// Used only if the runtime doesn't support `Intl.supportedValuesOf` — covers
// the market timezone plus the other continental US zones, which is the
// common case for this app.
const FALLBACK_TIMEZONES = [
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Phoenix",
  "America/Los_Angeles",
  "America/Anchorage",
  "Pacific/Honolulu",
  "UTC",
  "Europe/London",
  "Europe/Paris",
  "Europe/Berlin",
  "Asia/Tokyo",
  "Asia/Shanghai",
  "Asia/Kolkata",
  "Australia/Sydney",
];

/** All timezone identifiers offered in the picker. */
export function listTimezones(): string[] {
  const supportedValuesOf = (Intl as unknown as { supportedValuesOf?: (key: string) => string[] })
    .supportedValuesOf;
  if (typeof supportedValuesOf === "function") {
    try {
      return supportedValuesOf("timeZone");
    } catch {
      return FALLBACK_TIMEZONES;
    }
  }
  return FALLBACK_TIMEZONES;
}

/** The runtime's own timezone, e.g. what a fresh install should default to. */
export function systemTimezone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone;
  } catch {
    return "UTC";
  }
}

/** Resolves a saved `display.timezone` setting to a concrete IANA identifier. */
export function resolveTimezone(saved: string | null | undefined): string {
  if (!saved || saved === TIMEZONE_AUTO) {
    return systemTimezone();
  }
  return saved;
}
