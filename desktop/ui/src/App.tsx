import { useEffect, useState } from "react";
import { RemoteImage } from "./components/RemoteImage";
import { TitleBar } from "./components/TitleBar";
import { AppShell, DatePicker, todayIsoDate, useStatusBar, useToast } from "./shell";
import {
  applyThemeRootBlock,
  fetchAppMetadata,
  fetchThemeCss,
  runCli,
  type AppMetadata,
} from "./lib/nest";
import { quitApp } from "./lib/tauri";

const DEMO_IMAGE =
  "https://upload.wikimedia.org/wikipedia/commons/thumb/4/47/PNG_transparency_demonstration_1.png/280px-PNG_transparency_demonstration_1.png";

export function App() {
  const [metadata, setMetadata] = useState<AppMetadata | null>(null);
  const [date, setDate] = useState(todayIsoDate());
  const toast = useToast();
  const { setStatus } = useStatusBar();

  useEffect(() => {
    void (async () => {
      try {
        const [meta, theme] = await Promise.all([
          fetchAppMetadata(),
          fetchThemeCss(),
        ]);
        applyThemeRootBlock(theme.root_block);
        setMetadata(meta);
        setStatus(`Loaded ${meta.title}`, { variant: "success", timeoutMs: 3000 });
      } catch {
        setMetadata({ name: "finch", title: "Finch" });
      }
    })();
  }, [setStatus]);

  const appTitle = metadata?.title ?? "Finch";

  return (
    <AppShell
      titleBar={
        <TitleBar
          title={appTitle}
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
      <div className="mx-auto flex h-full max-w-3xl flex-col gap-6 overflow-auto p-8">
        <header>
          <h1 className="text-2xl font-semibold">{appTitle}</h1>
          <p className="text-sm text-nest-muted">
            Tauri + React + Tailwind · shared Nest shell (cbre-light theme)
          </p>
        </header>

        <section className="rounded-nest-lg border border-nest-border bg-nest-surface p-6">
          <h2 className="mb-3 text-lg font-medium">Date picker</h2>
          <DatePicker value={date} onChange={setDate} variant="default" placement="below" />
        </section>

        <section className="rounded-nest-lg border border-nest-border bg-nest-surface p-6">
          <h2 className="mb-4 text-lg font-medium">Remote image (nest_image_fetch)</h2>
          <RemoteImage
            url={DEMO_IMAGE}
            alt="PNG transparency demo"
            tags={["demo"]}
            className="h-48 w-full rounded-nest-md object-contain"
          />
        </section>
      </div>
    </AppShell>
  );
}
