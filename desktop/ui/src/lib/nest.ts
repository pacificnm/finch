import { invoke } from "@tauri-apps/api/core";

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
export type CliCommand = "AboutVersion" | "ListRecipes" | { Greet: { name: string } };

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

export async function schwabAuthStatus(): Promise<string> {
  return invoke<string>("plugin:finch|schwab_auth_status");
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

export function applyThemeRootBlock(rootBlock: string): void {
  let style = document.getElementById("nest-theme-vars");
  if (!style) {
    style = document.createElement("style");
    style.id = "nest-theme-vars";
    document.head.appendChild(style);
  }
  style.textContent = rootBlock;
}
