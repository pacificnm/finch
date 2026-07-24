import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { StatusBarProvider } from "./context/StatusBarContext";
import { ToastProvider } from "./context/ToastContext";
import { TimezoneProvider } from "./context/TimezoneContext";
import "./lib/fontawesome";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary label="root">
      <ToastProvider>
        <StatusBarProvider>
          <TimezoneProvider>
            <App />
          </TimezoneProvider>
        </StatusBarProvider>
      </ToastProvider>
    </ErrorBoundary>
  </StrictMode>,
);
