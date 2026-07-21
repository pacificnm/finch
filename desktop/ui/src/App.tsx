import { useEffect, useState } from "react";
import { TitleBar } from "./components/TitleBar";
import { TradingWorkspace } from "./components/trading/TradingWorkspace";
import { AppShell, useStatusBar, useToast } from "./shell";
import {
  applyThemeRootBlock,
  fetchAppMetadata,
  fetchThemeCss,
  listThemes,
  runCli,
  setActiveTheme,
  type AppMetadata,
  type ThemeSummary,
} from "./lib/nest";
import { quitApp } from "./lib/tauri";

const THEME_STORAGE_KEY = "finch.theme-id";

export function App() {
  const [metadata, setMetadata] = useState<AppMetadata | null>(null);
  const [themes, setThemes] = useState<ThemeSummary[]>([]);
  const [activeThemeId, setActiveThemeId] = useState<string | null>(null);
  const toast = useToast();
  const { setStatus } = useStatusBar();

  useEffect(() => {
    void (async () => {
      try {
        const meta = await fetchAppMetadata();
        setMetadata(meta);
        setStatus(`Loaded ${meta.title}`, { variant: "success", timeoutMs: 3000 });
      } catch (error: unknown) {
        setMetadata({ name: "finch", title: "Finch" });
        toast.error(`Failed to load app metadata: ${String(error)}`);
      }
    })();

    void (async () => {
      try {
        const theme = await fetchThemeCss();
        applyThemeRootBlock(theme.root_block);
        setActiveThemeId(theme.id);

        const themeList = await listThemes();
        setThemes(themeList);

        const remembered = window.localStorage.getItem(THEME_STORAGE_KEY);
        const rememberedIsValid = themeList.some((entry) => entry.id === remembered);
        if (remembered && rememberedIsValid && remembered !== theme.id) {
          const applied = await setActiveTheme(remembered);
          applyThemeRootBlock(applied.root_block);
          setActiveThemeId(applied.id);
        }
      } catch (error: unknown) {
        toast.error(`Failed to load themes: ${String(error)}`);
      }
    })();
  }, [setStatus]);

  const handleSelectTheme = (id: string) => {
    void setActiveTheme(id)
      .then((theme) => {
        applyThemeRootBlock(theme.root_block);
        setActiveThemeId(theme.id);
        window.localStorage.setItem(THEME_STORAGE_KEY, theme.id);
      })
      .catch((error: unknown) => toast.error(`Failed to switch theme: ${String(error)}`));
  };

  const appTitle = metadata?.title ?? "Finch";

  return (
    <AppShell
      titleBar={
        <TitleBar
          title={appTitle}
          themes={themes}
          activeThemeId={activeThemeId}
          onSelectTheme={handleSelectTheme}
          onQuit={() => void quitApp()}
          onShowRecipes={() => {
            void runCli("ListRecipes")
              .then((recipes) => toast.info(recipes || "No recipes applied."))
              .catch((error: unknown) =>
                toast.error(`Failed to list recipes: ${String(error)}`),
              );
          }}
          onAbout={() => {
            void runCli("AboutVersion")
              .then((version) => toast.info(`${appTitle} v${version}`))
              .catch((error: unknown) =>
                toast.error(`Failed to read version: ${String(error)}`),
              );
          }}
        />
      }
      statusLeft={<span>Ready</span>}
      statusRight={<span>{metadata?.name ?? "…"}</span>}
    >
      <TradingWorkspace />
    </AppShell>
  );
}
