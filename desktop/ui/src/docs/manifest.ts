export type DocManifestEntry = {
  id: string;
  title: string;
  /** Filename relative to `desktop/ui/src/docs/`. */
  file: string;
};

/**
 * The Help → Documents table of contents. Order here is display order —
 * this list *is* the index, not something derived from filenames or
 * in-file headings, so reordering/renaming entries is the whole edit.
 */
export const DOCS_MANIFEST: DocManifestEntry[] = [
  { id: "overview", title: "Overview", file: "overview.md" },
  { id: "market-data-tools", title: "Market Data & News Tools", file: "market-data-tools.md" },
  {
    id: "technical-analysis-tools",
    title: "Technical Analysis Tools",
    file: "technical-analysis-tools.md",
  },
  {
    id: "chart-overlays",
    title: "Chart Overlays: Studies & Patterns",
    file: "chart-overlays.md",
  },
  { id: "trade-setup", title: "Trade Setup Workflow", file: "trade-setup.md" },
  { id: "tips", title: "Tips & Example Prompts", file: "tips.md" },
];
