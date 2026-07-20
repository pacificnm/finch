/**
 * Shared Nest desktop shell.
 *
 * Product-agnostic UI: app shell, status bar, toasts, dialogs, and the date
 * picker. Themed via `nest-react-theme` CSS
 * variables (`--nest-color-*`) and the Tailwind preset.
 */

export { AppShell } from "../components/AppShell";
export { ErrorBoundary } from "../components/ErrorBoundary";
export { Icon } from "../components/Icon";
export { ConfirmDialog } from "../components/ConfirmDialog";
export { DatePicker } from "../components/DatePicker";
export { StatusBar } from "../components/StatusBar";
export { ToastViewport } from "../components/ToastViewport";

export {
  ToastProvider,
  useToast,
  type ToastItem,
  type ToastOptions,
  type ToastVariant,
} from "../context/ToastContext";
export {
  StatusBarProvider,
  useStatusBar,
  type StatusVariant,
} from "../context/StatusBarContext";

export { formatDisplayDate, todayIsoDate } from "../lib/date";
export { isTauri, quitApp } from "../lib/tauri";
