import { invoke } from "@tauri-apps/api/core";
import type { CandlestickData, Time } from "lightweight-charts";
import { formatIpcError } from "./errors";

export type AppMetadata = {
  name: string;
  title: string;
};

export type ThemeCss = {
  id: string;
  mode: string;
  variables: Record<string, string>;
  root_block: string;
};

export type ThemeSummary = {
  id: string;
  mode: string;
};

export type ImageFetchResponse = {
  bytes_base64: string;
  mime: string;
  cache_key: string;
};

export type ImageFetchRequest = {
  url: string;
  tags?: string[] | null;
};

// Mirrors finch_core::CliCommand (default serde externally-tagged
// representation: unit variants serialize as a bare string, struct variants
// as `{ VariantName: { ...fields } }`).
export type CliCommand =
  | "AboutVersion"
  | "ListRecipes"
  | { Greet: { name: string } }
  | { SchwabSearchInstruments: { query: string } }
  | { SchwabQuoteJson: { symbol: string } }
  | {
      SchwabPriceHistory: {
        symbol: string;
        period_type: string;
        period: string;
        frequency_type: string;
        frequency: string;
        start_date?: string;
        end_date?: string;
      };
    };

export async function fetchAppMetadata(): Promise<AppMetadata> {
  return invoke<AppMetadata>("nest_app_metadata");
}

export async function fetchThemeCss(): Promise<ThemeCss> {
  return invoke<ThemeCss>("nest_theme_css");
}

export async function listThemes(): Promise<ThemeSummary[]> {
  return invoke<ThemeSummary[]>("nest_theme_list");
}

export async function setActiveTheme(id: string): Promise<ThemeCss> {
  return invoke<ThemeCss>("nest_theme_set_active", { request: { id } });
}

export async function fetchImage(
  url: string,
  tags?: string[],
): Promise<ImageFetchResponse> {
  return invoke<ImageFetchResponse>("nest_image_fetch", {
    request: { url, tags: tags ?? null } satisfies ImageFetchRequest,
  });
}

export async function invalidateImageTag(tag: string): Promise<{ removed_count: number }> {
  return invoke<{ removed_count: number }>("nest_image_invalidate_tag", {
    request: { tag },
  });
}

// App-specific commands are registered as a Tauri plugin (not the main
// invoke_handler, which nest-tauri reserves for its built-in commands — see
// src-tauri/src/main.rs), so the UI invokes them under the `plugin:` prefix.
export async function runCli(command: CliCommand): Promise<string> {
  return invoke<string>("plugin:finch|run_cli", { command });
}

export async function schwabAuthBegin(): Promise<string> {
  return invoke<string>("plugin:finch|schwab_auth_begin");
}

export async function schwabAuthComplete(code: string, state: string): Promise<string> {
  return invoke<string>("plugin:finch|schwab_auth_complete", { code, state });
}

/**
 * Opens an in-app login webview for Schwab, captures the OAuth callback
 * automatically, and resolves once the token has been stored — no manual
 * code copy/paste. Resolves/rejects only after the login webview closes.
 */
export async function schwabAuthLogin(): Promise<void> {
  const { listen } = await import("@tauri-apps/api/event");
  return new Promise<void>((resolve, reject) => {
    let stop: (() => void) | undefined;
    listen<string | null>("schwab-auth-result", (event) => {
      stop?.();
      if (event.payload) {
        reject(new Error(event.payload));
      } else {
        resolve();
      }
    })
      .then((unlisten) => {
        stop = unlisten;
      })
      .catch(reject);

    invoke("plugin:finch|schwab_auth_login").catch((err: unknown) => {
      stop?.();
      reject(err instanceof Error ? err : new Error(String(err)));
    });
  });
}

export async function schwabAuthStatus(): Promise<string> {
  return invoke<string>("plugin:finch|schwab_auth_status");
}

export async function schwabAuthLogout(): Promise<string> {
  return invoke<string>("plugin:finch|schwab_auth_logout");
}

export type SchwabAccount = {
  account_number: string;
  hash: string;
  nickname?: string;
  display_account_id?: string;
  account_type?: string;
  primary_account: boolean;
  account_color?: string;
};

export type SchwabAccountSummary = {
  account_value: string;
  buying_power: string;
  cash_for_withdrawal: string;
  pl_day_percent: string;
};

export type SchwabOrderRow = {
  order_id: string;
  time: string;
  side: string;
  pos_effect: string;
  qty: string;
  amount: string;
  symbol: string;
  desc: string;
  price: string;
  tif: string;
  mark: string;
  net_prc: string;
  status: string;
};

export type SchwabPositionRow = {
  position: string;
  qty: string;
  pl_day: string;
  pl_open: string;
  pl_ytd: string;
  cost: string;
  net_liq: string;
  trade_price: string;
  bp_effect: string;
  delta: string;
  gamma: string;
  theta: string;
  vega: string;
};

export type InstrumentSearchResult = {
  symbol: string;
  description: string;
  asset_type: string;
  exchange: string;
};

export async function searchInstruments(
  query: string,
): Promise<InstrumentSearchResult[]> {
  console.log("[searchInstruments] Calling CLI with query:", query);
  try {
    const jsonStr = await runCli({ "SchwabSearchInstruments": { query } } as CliCommand);
    console.log("[searchInstruments] CLI response:", jsonStr.substring(0, 200));
    const parsed = JSON.parse(jsonStr) as InstrumentSearchResult[];
    console.log("[searchInstruments] Parsed results count:", parsed.length);
    return parsed;
  } catch (error) {
    console.error("[searchInstruments] Failed:", error);
    throw error;
  }
}

export type QuoteData = {
  symbol: string;
  description?: string;
  lastPrice?: number;
  openPrice?: number;
  highPrice?: number;
  lowPrice?: number;
  closePrice?: number;
  netChange?: number;
  percentChange?: number;
  bidSize?: string;
  askSize?: string;
  volume?: number;
  marketCap?: number;
  peRatio?: number;
  dividendYield?: number;
  beta?: number;
  eps?: number;
  divAmount?: number;
  avg10DayVolume?: number;
  avg1YearVolume?: number;
  avg50DayVolume?: number;
  sharesOutstanding?: number;
  vwap?: number;
  // Optional fields that may be present in Schwab quote/fundamental data or calculated.
  iv?: number;
  hv?: number;
  mmm?: number;
  exDate?: string;
  earningsDate?: string;
};

export type SchwabCandle = {
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  datetime: number;
};

export type SchwabPriceHistoryResponse = {
  candles: SchwabCandle[];
  symbol: string;
  empty: boolean;
};

export async function fetchQuote(symbol: string): Promise<QuoteData> {
  console.log("[fetchQuote] Calling CLI for symbol:", symbol);
  try {
    const jsonStr = await runCli({ "SchwabQuoteJson": { symbol } } as CliCommand);
    console.log("[fetchQuote] CLI raw response:\n", jsonStr);
    const parsed = JSON.parse(jsonStr) as QuoteData;
    console.log("[fetchQuote] Parsed QuoteData:", parsed);
    return parsed;
  } catch (error) {
    console.error("[fetchQuote] Failed:", error);
    throw error;
  }
}

// Schwab's relative `periodType=day` window doesn't reliably include the
// current session — confirmed live: `periodType=day&period=1` (and `2`)
// returned only bars through the prior day's after-hours close, even hours
// into today's open session, while an equivalent explicit `startDate`/
// `endDate` request returned bars up to the current minute. So the
// day-based periods below carry a calendar-day lookback for use as explicit
// dates instead of being sent as `periodType`/`period`.
const DAY_PERIOD_LOOKBACK_DAYS: Record<string, number> = {
  today: 1,
  "1d": 1,
  "3d": 3,
  "1w": 7,
  "2w": 14,
};

function schwabHistoryParams(
  period: string,
  interval: string,
): {
  period_type: string;
  period: string;
  frequency_type: string;
  frequency: string;
  aggregate_to?: string;
  start_date?: string;
  end_date?: string;
} {
  // Map period to Schwab periodType/period.
  const periodMap: Record<string, { period_type: string; period: string }> = {
    today: { period_type: "day", period: "1" },
    "1d": { period_type: "day", period: "1" },
    "3d": { period_type: "day", period: "3" },
    "1w": { period_type: "day", period: "5" },
    "2w": { period_type: "day", period: "10" },
    "1m": { period_type: "month", period: "1" },
    "3m": { period_type: "month", period: "3" },
    "6m": { period_type: "month", period: "6" },
    ytd: { period_type: "ytd", period: "1" },
    "1y": { period_type: "year", period: "1" },
    "3y": { period_type: "year", period: "3" },
    "5y": { period_type: "year", period: "5" },
    "15y": { period_type: "year", period: "15" },
    max: { period_type: "year", period: "20" },
  };

  const mapped = periodMap[period] ?? { period_type: "year", period: "1" };

  // Map interval. Schwab only supports minute frequencies up to 30m, so for
  // hourly intervals we fetch 30m candles and aggregate them in the UI.
  const intervalMap: Record<string, { frequency_type: string; frequency: string; aggregate_to?: string }> = {
    "1m": { frequency_type: "minute", frequency: "1" },
    "3m": { frequency_type: "minute", frequency: "3" },
    "10m": { frequency_type: "minute", frequency: "10" },
    "15m": { frequency_type: "minute", frequency: "15" },
    "30m": { frequency_type: "minute", frequency: "30" },
    "1h": { frequency_type: "minute", frequency: "30", aggregate_to: "1h" },
    "2h": { frequency_type: "minute", frequency: "30", aggregate_to: "2h" },
    "4h": { frequency_type: "minute", frequency: "30", aggregate_to: "4h" },
    "1d": { frequency_type: "daily", frequency: "1" },
    "1w": { frequency_type: "weekly", frequency: "1" },
    "1mo": { frequency_type: "monthly", frequency: "1" },
  };

  const intervalMapped = intervalMap[interval] ?? { frequency_type: "daily", frequency: "1" };

  // Validate: minute data is only supported with periodType=day.
  if (intervalMapped.frequency_type === "minute" && mapped.period_type !== "day") {
    return { period_type: mapped.period_type, period: mapped.period, frequency_type: "daily", frequency: "1" };
  }

  if (intervalMapped.frequency_type === "minute") {
    const lookbackDays = DAY_PERIOD_LOOKBACK_DAYS[period] ?? 1;
    const end = Date.now();
    const start = end - lookbackDays * 24 * 60 * 60 * 1000;
    return { ...mapped, ...intervalMapped, start_date: String(start), end_date: String(end) };
  }

  return { ...mapped, ...intervalMapped };
}

function aggregateCandles(
  candles: SchwabCandle[],
  targetInterval: "1h" | "2h" | "4h",
): SchwabCandle[] {
  const minutes = targetInterval === "1h" ? 60 : targetInterval === "2h" ? 120 : 240;
  if (candles.length === 0) return [];

  // Determine the start of the first bucket using the first candle's timestamp.
  const firstTs = candles[0]!.datetime;
  const bucketStart = Math.floor(firstTs / (minutes * 60 * 1000)) * (minutes * 60 * 1000);

  const buckets = new Map<number, SchwabCandle[]>();
  for (const candle of candles) {
    const offset = Math.floor((candle.datetime - bucketStart) / (minutes * 60 * 1000));
    const key = bucketStart + offset * minutes * 60 * 1000;
    const bucket = buckets.get(key);
    if (bucket) {
      bucket.push(candle);
    } else {
      buckets.set(key, [candle]);
    }
  }

  const sortedKeys = Array.from(buckets.keys()).sort((a, b) => a - b);
  return sortedKeys.map((key) => {
    const bucket = buckets.get(key)!;
    return {
      open: bucket[0]!.open,
      high: Math.max(...bucket.map((c) => c.high)),
      low: Math.min(...bucket.map((c) => c.low)),
      close: bucket[bucket.length - 1]!.close,
      volume: bucket.reduce((sum, c) => sum + c.volume, 0),
      datetime: key,
    };
  });
}

/** A candle plus its volume — `lightweight-charts`' own `CandlestickData` doesn't carry volume. */
export type OhlcvData = CandlestickData & { volume: number };

export async function fetchPriceHistory(
  symbol: string,
  period: string,
  interval: string,
): Promise<OhlcvData[]> {
  console.log("[fetchPriceHistory] Calling CLI for symbol:", symbol, "period:", period, "interval:", interval);
  const params = schwabHistoryParams(period, interval);

  try {
    const jsonStr = await runCli({
      "SchwabPriceHistory": {
        symbol,
        period_type: params.period_type,
        period: params.period,
        frequency_type: params.frequency_type,
        frequency: params.frequency,
        start_date: params.start_date,
        end_date: params.end_date,
      },
    } as CliCommand);
    console.log("[fetchPriceHistory] CLI raw response:\n", jsonStr);
    const parsed = JSON.parse(jsonStr) as SchwabPriceHistoryResponse;

    let candles = parsed.candles ?? [];
    if (params.aggregate_to) {
      candles = aggregateCandles(candles, params.aggregate_to as "1h" | "2h" | "4h");
    }

    const isIntraday = params.frequency_type === "minute" || params.aggregate_to != null;

    return candles.map((c) =>
      ({
        time: (isIntraday
          ? Math.floor(c.datetime / 1000)
          : new Date(c.datetime).toISOString().slice(0, 10)) as Time,
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close,
        volume: c.volume,
      }) as OhlcvData,
    );
  } catch (error) {
    console.error("[fetchPriceHistory] Failed:", error);
    throw error;
  }
}

export async function fetchSchwabAccounts(): Promise<SchwabAccount[]> {
  return invoke<SchwabAccount[]>("plugin:finch|schwab_accounts");
}

export async function fetchSchwabAccountSummary(
  accountHash: string,
): Promise<SchwabAccountSummary> {
  return invoke<SchwabAccountSummary>("plugin:finch|schwab_account_summary", {
    accountHash,
  });
}

export async function fetchSchwabOrders(
  accountHash: string,
): Promise<SchwabOrderRow[]> {
  return invoke<SchwabOrderRow[]>("plugin:finch|schwab_orders", {
    accountHash,
  });
}

export async function fetchSchwabPositions(
  accountHash: string,
): Promise<SchwabPositionRow[]> {
  return invoke<SchwabPositionRow[]>("plugin:finch|schwab_positions", {
    accountHash,
  });
}

export type AskStockQuestionHandlers = {
  onChunk: (delta: string) => void;
  onDone: () => void;
  onError: (message: string) => void;
};

type AiChatChunkEvent = { request_id: string; delta: string };
type AiChatDoneEvent = { request_id: string };
type AiChatErrorEvent = { request_id: string; message: string };

/**
 * Streams an answer to a free-form question about `symbol` via the
 * `ai-chat-chunk`/`ai-chat-done`/`ai-chat-error` events emitted by the
 * `ask_stock_question` command. Returns a stop function that unsubscribes
 * from those events — call it if the caller unmounts or starts a new
 * question before this one resolves, to avoid stale chunks landing on a
 * later question's answer.
 */
export async function askStockQuestion(
  symbol: string,
  question: string,
  handlers: AskStockQuestionHandlers,
): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  const requestId =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random()}`;

  const unlistenChunk = await listen<AiChatChunkEvent>("ai-chat-chunk", (event) => {
    if (event.payload.request_id === requestId) {
      handlers.onChunk(event.payload.delta);
    }
  });
  const unlistenDone = await listen<AiChatDoneEvent>("ai-chat-done", (event) => {
    if (event.payload.request_id === requestId) {
      stop();
      handlers.onDone();
    }
  });
  const unlistenError = await listen<AiChatErrorEvent>("ai-chat-error", (event) => {
    if (event.payload.request_id === requestId) {
      stop();
      handlers.onError(event.payload.message);
    }
  });

  function stop() {
    unlistenChunk();
    unlistenDone();
    unlistenError();
  }

  try {
    await invoke("plugin:finch|ask_stock_question", { symbol, question, requestId });
  } catch (err: unknown) {
    stop();
    handlers.onError(formatIpcError(err));
  }

  return stop;
}

export function applyThemeRootBlock(rootBlock: string): void {
  let style = document.getElementById("nest-theme-vars");
  if (!style) {
    style = document.createElement("style");
    style.id = "nest-theme-vars";
    document.head.appendChild(style);
  }
  style.textContent = rootBlock;
}
