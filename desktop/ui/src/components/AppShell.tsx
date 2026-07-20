import type { ReactNode } from "react";
import { StatusBar } from "./StatusBar";
import { ToastViewport } from "./ToastViewport";

type AppShellProps = {
  /** Title bar / menu bar (or other top chrome) rendered above the main region. */
  titleBar?: ReactNode;
  /** Optional left-hand nav (e.g. a section/page menu). */
  nav?: ReactNode;
  /** Optional right-hand rail (e.g. an assistant/inspector panel). */
  rail?: ReactNode;
  /** Left slot of the status bar. */
  statusLeft?: ReactNode;
  /** Right slot of the status bar. */
  statusRight?: ReactNode;
  /** Main content region. */
  children: ReactNode;
};

/**
 * Product-agnostic window layout: top chrome, a main region with optional
 * left nav and right rail, a footer status bar, and the toast viewport.
 *
 * Wrap the tree in {@link ToastProvider} and {@link StatusBarProvider} above
 * this component so `useToast` / `useStatusBar` are available.
 */
export function AppShell({
  titleBar,
  nav,
  rail,
  statusLeft,
  statusRight,
  children,
}: AppShellProps) {
  return (
    <div className="flex h-screen min-h-0 flex-col bg-nest-background text-nest-foreground">
      {titleBar}

      <div className="flex min-h-0 flex-1">
        {nav ? (
          <nav className="flex w-56 shrink-0 flex-col border-r border-nest-border bg-nest-surface">
            {nav}
          </nav>
        ) : null}

        <div className="flex min-w-0 flex-1 flex-col">
          <main className="min-h-0 flex-1 overflow-hidden">{children}</main>
        </div>

        {rail ? (
          <aside className="flex w-80 shrink-0 flex-col border-l border-nest-border bg-nest-surface">
            {rail}
          </aside>
        ) : null}
      </div>

      <StatusBar left={statusLeft} right={statusRight} />
      <ToastViewport />
    </div>
  );
}
