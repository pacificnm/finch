import { useEffect, useId, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkBreaks from "remark-breaks";
import { faFileLines, faXmark } from "../lib/fontawesome";
import { Icon } from "./Icon";
import { DOC_ENTRIES } from "../lib/docs";

type HelpDocumentsDrawerProps = {
  open: boolean;
  onClose: () => void;
};

/**
 * Help → Documents: a slide-out reader for the in-app markdown docs (see
 * `src/docs/`). Adapts `ConfirmDialog`'s backdrop/Escape-to-close skeleton
 * for a left-anchored, 2/3-width panel instead of a centered box.
 */
export function HelpDocumentsDrawer({ open, onClose }: HelpDocumentsDrawerProps) {
  const titleId = useId();
  const [selectedId, setSelectedId] = useState<string>(DOC_ENTRIES[0]?.id ?? "");

  useEffect(() => {
    if (!open) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  const selectedDoc = DOC_ENTRIES.find((doc) => doc.id === selectedId) ?? DOC_ENTRIES[0] ?? null;

  return (
    <div className="fixed inset-0 z-[70] bg-black/40" onClick={onClose} role="presentation">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="help-drawer-enter fixed inset-y-0 left-0 flex h-full w-2/3 flex-col border-r border-nest-border bg-nest-surface shadow-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="flex shrink-0 items-center justify-between border-b border-nest-border px-5 py-3">
          <div className="flex items-center gap-2">
            <Icon icon={faFileLines} className="size-4 text-nest-primary" />
            <h2 id={titleId} className="text-sm font-semibold">
              Documentation
            </h2>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-nest-sm p-1 text-nest-muted hover:bg-nest-muted/10 hover:text-nest-foreground"
            aria-label="Close"
          >
            <Icon icon={faXmark} className="size-3.5" />
          </button>
        </header>

        <div className="flex min-h-0 flex-1">
          <nav className="w-56 shrink-0 overflow-y-auto border-r border-nest-border p-2">
            <ul className="flex flex-col gap-0.5">
              {DOC_ENTRIES.map((doc) => (
                <li key={doc.id}>
                  <button
                    type="button"
                    onClick={() => setSelectedId(doc.id)}
                    className={[
                      "w-full rounded-nest-sm px-3 py-1.5 text-left text-[12px]",
                      doc.id === selectedDoc?.id
                        ? "bg-nest-primary/10 font-medium text-nest-primary"
                        : "text-nest-foreground hover:bg-nest-muted/10",
                    ].join(" ")}
                  >
                    {doc.title}
                  </button>
                </li>
              ))}
            </ul>
          </nav>

          <div className="min-h-0 flex-1 overflow-y-auto p-6">
            {selectedDoc ? (
              <article className="nest-rich-text max-w-none">
                <ReactMarkdown remarkPlugins={[remarkGfm, remarkBreaks]}>
                  {selectedDoc.content}
                </ReactMarkdown>
              </article>
            ) : (
              <p className="text-sm text-nest-muted">No documents available.</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
