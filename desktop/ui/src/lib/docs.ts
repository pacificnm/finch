import { DOCS_MANIFEST, type DocManifestEntry } from "../docs/manifest";

export type DocEntry = {
  id: string;
  title: string;
  content: string;
};

// Vite resolves this at build time into a plain Record<path, content> —
// eager + ?raw means no async loading state is needed anywhere downstream.
const rawDocs = import.meta.glob("/src/docs/**/*.md", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

function resolveContent(entry: DocManifestEntry): string | null {
  const key = `/src/docs/${entry.file}`;
  return Object.prototype.hasOwnProperty.call(rawDocs, key) ? rawDocs[key]! : null;
}

/**
 * The Help → Documents content, in table-of-contents order. A manifest
 * entry whose `file` doesn't match an actual `.md` file is dropped (with a
 * console error) rather than crashing the app — a typo in `manifest.ts`
 * shouldn't take down the whole UI.
 */
export const DOC_ENTRIES: DocEntry[] = DOCS_MANIFEST.flatMap((entry) => {
  const content = resolveContent(entry);
  if (content === null) {
    // eslint-disable-next-line no-console
    console.error(`[docs] manifest entry "${entry.id}" references missing file: ${entry.file}`);
    return [];
  }
  return [{ id: entry.id, title: entry.title, content }];
});
