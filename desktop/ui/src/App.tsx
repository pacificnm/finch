import { useEffect, useState } from "react";
import { TitleBar } from "./components/TitleBar";
import { LoginScreen } from "./components/LoginScreen";
import { SettingsScreen } from "./components/SettingsScreen";
import { HelpDocumentsDrawer } from "./components/HelpDocumentsDrawer";
import { TradingWorkspace } from "./components/trading/TradingWorkspace";
import { AppShell, useStatusBar, useToast } from "./shell";
import {
  applyThemeRootBlock,
  fetchAppMetadata,
  fetchThemeCss,
  listThemes,
  runCli,
  schwabAuthLogout,
  schwabAuthStatus,
  setActiveTheme,
  type AppMetadata,
  type ThemeSummary,
} from "./lib/nest";
import { quitApp } from "./lib/tauri";
import { SettingKeys, Settings } from "./lib/settings";

const THEME_STORAGE_KEY = "finch.theme-id";
const DEFAULT_SYMBOL = "SCHG";

export function App() {
  const [metadata, setMetadata] = useState<AppMetadata | null>(null);
  const [themes, setThemes] = useState<ThemeSummary[]>([]);
  const [activeThemeId, setActiveThemeId] = useState<string | null>(null);
  const [selectedSymbol, setSelectedSymbol] = useState<string>(DEFAULT_SYMBOL);
  const [authStatus, setAuthStatus] = useState<"checking" | "logged-in" | "logged-out">("checking");
  const [showSettings, setShowSettings] = useState(false);
  const [docsOpen, setDocsOpen] = useState(false);
  const toast = useToast();
  const { setStatus } = useStatusBar();

  const checkAuth = async () => {
    try {
      const status = await schwabAuthStatus();
      setAuthStatus(status.startsWith("Logged in") ? "logged-in" : "logged-out");
    } catch (error: unknown) {
      setAuthStatus("logged-out");
      toast.error(`Failed to check Schwab auth status: ${String(error)}`);
    }
  };

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

    void checkAuth();

    void (async () => {
      try {
        const saved = await Settings.getString(SettingKeys.chartSymbol, "");
        if (saved) {
          setSelectedSymbol(saved);
        }
      } catch {
        // Settings may be unavailable on first run before migrations apply.
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

  const handleSymbolSelect = (symbol: string) => {
    setSelectedSymbol(symbol);
    setStatus(`Loaded ${symbol}`, { variant: "success", timeoutMs: 2000 });
    void Settings.setString(SettingKeys.chartSymbol, symbol).catch((error: unknown) =>
      toast.error(`Failed to save symbol: ${String(error)}`),
    );
  };

  return (
    <>
      <AppShell
        titleBar={
          <TitleBar
            themes={themes}
            activeThemeId={activeThemeId}
            onSelectTheme={handleSelectTheme}
            onSymbolSelect={handleSymbolSelect}
            onOpenSettings={() => setShowSettings(true)}
            onLogOut={() => {
              void schwabAuthLogout()
                .then(() => setAuthStatus("logged-out"))
                .catch((error: unknown) => toast.error(`Failed to log out: ${String(error)}`));
            }}
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
            onOpenDocuments={() => setDocsOpen(true)}
          />
        }
        statusLeft={<span>Ready</span>}
        statusRight={<span>{metadata?.name ?? "…"}</span>}
      >
        {showSettings ? (
          <SettingsScreen onClose={() => setShowSettings(false)} />
        ) : authStatus === "checking" ? (
          <div className="flex h-full items-center justify-center text-nest-muted">
            Checking login status…
          </div>
        ) : authStatus === "logged-out" ? (
          <LoginScreen onLoggedIn={() => setAuthStatus("logged-in")} />
        ) : (
          <TradingWorkspace symbol={selectedSymbol} />
        )}
      </AppShell>
      <HelpDocumentsDrawer open={docsOpen} onClose={() => setDocsOpen(false)} />
    </>
  );
}
