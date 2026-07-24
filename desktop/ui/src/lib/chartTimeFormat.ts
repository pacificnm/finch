import { TickMarkType, type BusinessDay, type Time, type TickMarkFormatter, type TimeFormatterFn } from "lightweight-charts";

function isBusinessDay(time: Time): time is BusinessDay {
  return typeof time === "object" && time !== null && "year" in time;
}

// Daily/weekly/monthly bars carry a calendar date with no time-of-day
// component (see `nest.ts`'s `fetchPriceHistory`, which emits a plain
// "YYYY-MM-DD" string for non-intraday bars) — there's nothing for a
// timezone to adjust, so this reads the y/m/d fields directly rather than
// building a `Date` and reformatting it, which would risk shifting the day
// depending on the viewer's local offset.
function businessDayToUtcDate(day: BusinessDay): Date {
  return new Date(Date.UTC(day.year, day.month - 1, day.day));
}

/**
 * Builds the chart's time-axis tick formatter for a given IANA timezone.
 * Only intraday ticks (`UTCTimestamp`, a real instant) are timezone-
 * sensitive; returning `null` for business-day ticks defers to
 * `lightweight-charts`' own (already timezone-agnostic) default.
 */
export function makeTickMarkFormatter(timezone: string): TickMarkFormatter {
  return (time, tickMarkType, locale) => {
    if (typeof time !== "number") {
      return null;
    }
    const date = new Date(time * 1000);
    switch (tickMarkType) {
      case TickMarkType.Year:
        return new Intl.DateTimeFormat(locale, { year: "numeric", timeZone: timezone }).format(date);
      case TickMarkType.Month:
        return new Intl.DateTimeFormat(locale, { month: "short", timeZone: timezone }).format(date);
      case TickMarkType.DayOfMonth:
        return new Intl.DateTimeFormat(locale, {
          day: "numeric",
          month: "short",
          timeZone: timezone,
        }).format(date);
      case TickMarkType.TimeWithSeconds:
        return new Intl.DateTimeFormat(locale, {
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
          hour12: false,
          timeZone: timezone,
        }).format(date);
      default:
        return new Intl.DateTimeFormat(locale, {
          hour: "2-digit",
          minute: "2-digit",
          hour12: false,
          timeZone: timezone,
        }).format(date);
    }
  };
}

/** Builds the crosshair/legend time label formatter for a given IANA timezone. */
export function makeTimeFormatter(timezone: string): TimeFormatterFn<Time> {
  return (time) => {
    if (isBusinessDay(time)) {
      return new Intl.DateTimeFormat(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
        timeZone: "UTC",
      }).format(businessDayToUtcDate(time));
    }
    if (typeof time === "string") {
      return time;
    }
    return new Intl.DateTimeFormat(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
      timeZone: timezone,
    }).format(new Date(time * 1000));
  };
}
