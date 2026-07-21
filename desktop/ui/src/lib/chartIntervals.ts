import type { SelectOption } from "@nest/components";

/** Supported chart interval values. */
export type IntervalValue =
  | "1m"
  | "3m"
  | "10m"
  | "15m"
  | "30m"
  | "1h"
  | "2h"
  | "4h"
  | "1d"
  | "1w"
  | "1mo";

/** Interval option with a strongly-typed value. */
export interface IntervalOption extends SelectOption {
  value: IntervalValue;
}

/** Intervals for very short periods (Today, Day, 3 Days, Week). */
export const INTERVAL_OPTIONS_SHORT_TERM: IntervalOption[] = [
  { value: "1m", label: "1 Minute" },
  { value: "3m", label: "3 Minutes" },
  { value: "10m", label: "10 Minutes" },
  { value: "15m", label: "15 Minutes" },
  { value: "30m", label: "30 Minutes" },
  { value: "1h", label: "1 Hour" },
];

/** Intervals for short periods (2 Weeks, 1 Month). */
export const INTERVAL_OPTIONS_HOURLY: IntervalOption[] = [
  { value: "1h", label: "1 Hour" },
  { value: "2h", label: "2 Hours" },
  { value: "4h", label: "4 Hours" },
];

/** Intervals for medium periods (3 Months, 6 Months). */
export const INTERVAL_OPTIONS_DAILY_WEEKLY: IntervalOption[] = [
  { value: "1d", label: "1 Day" },
  { value: "1w", label: "1 Week" },
];

/** Intervals for long periods (YTD, 1 Year, 3 Years, 5 Years). */
export const INTERVAL_OPTIONS_DAILY_WEEKLY_MONTHLY: IntervalOption[] = [
  { value: "1d", label: "1 Day" },
  { value: "1w", label: "1 Week" },
  { value: "1mo", label: "1 Month" },
];

/** Monthly interval only, for very long periods (15 Years, Max). */
export const INTERVAL_OPTIONS_MONTHLY: IntervalOption[] = [
  { value: "1mo", label: "1 Month" },
];

/**
 * Returns the available interval options for a given chart period.
 *
 * Mapping rules:
 * - Today, Day, 3 Days, Week -> 1m, 3m, 10m, 15m, 30m, 1h
 * - 2 Weeks, 1 Month -> 1h, 2h, 4h
 * - 3 Months, 6 Months -> 1d, 1w
 * - YTD, 1 Year, 3 Years, 5 Years -> 1d, 1w, 1mo
 * - 15 Years, Max -> 1mo
 */
export function getIntervalOptionsForPeriod(period: string): IntervalOption[] {
  switch (period) {
    case "today":
    case "1d":
    case "3d":
    case "1w":
      return INTERVAL_OPTIONS_SHORT_TERM;
    case "2w":
    case "1m":
      return INTERVAL_OPTIONS_HOURLY;
    case "3m":
    case "6m":
      return INTERVAL_OPTIONS_DAILY_WEEKLY;
    case "ytd":
    case "1y":
    case "3y":
    case "5y":
      return INTERVAL_OPTIONS_DAILY_WEEKLY_MONTHLY;
    case "15y":
    case "max":
      return INTERVAL_OPTIONS_MONTHLY;
    default:
      return INTERVAL_OPTIONS_DAILY_WEEKLY_MONTHLY;
  }
}

/**
 * Returns the default interval value for a given chart period.
 * Use this to reset the interval when the period changes and the current
 * interval is no longer available.
 */
export function getDefaultIntervalForPeriod(period: string): IntervalValue {
  switch (period) {
    case "today":
    case "1d":
    case "3d":
    case "1w":
    case "2w":
    case "1m":
      return "1h";
    case "3m":
    case "6m":
    case "ytd":
    case "1y":
    case "3y":
    case "5y":
      return "1d";
    case "15y":
    case "max":
      return "1mo";
    default:
      return "1d";
  }
}
