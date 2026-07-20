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

export function applyThemeRootBlock(rootBlock: string): void {
  let style = document.getElementById("nest-theme-vars");
  if (!style) {
    style = document.createElement("style");
    style.id = "nest-theme-vars";
    document.head.appendChild(style);
  }
  style.textContent = rootBlock;
}
