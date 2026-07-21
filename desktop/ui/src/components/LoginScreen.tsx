import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@nest/components";

export type LoginScreenProps = {
  onLoggedIn: () => void;
};

export function LoginScreen({ onLoggedIn }: LoginScreenProps) {
  const [url, setUrl] = useState<string | null>(null);
  const [code, setCode] = useState("");
  const [state, setState] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleGenerateLink = async () => {
    setLoading(true);
    setError(null);
    try {
      const authUrl = await invoke<string>("plugin:finch|schwab_auth_begin");
      // Extract the state parameter so we can verify it on completion.
      const stateParam = new URL(authUrl).searchParams.get("state");
      setUrl(authUrl);
      setState(stateParam);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = async () => {
    if (!state) return;
    setLoading(true);
    setError(null);
    try {
      await invoke<string>("plugin:finch|schwab_auth_complete", {
        code: code.trim(),
        state,
      });
      onLoggedIn();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 p-6">
      <div className="w-full max-w-md rounded-nest-md border border-nest-border bg-nest-surface p-6 shadow-sm">
        <h1 className="mb-2 text-xl font-semibold">Connect Schwab Account</h1>
        <p className="mb-4 text-[12px] text-nest-muted">
          Link your Schwab account to access live market data, positions, and trading.
        </p>

        {!url ? (
          <Button
            onClick={handleGenerateLink}
            disabled={loading}
            variant="contained"
            className="w-full"
          >
            {loading ? "Generating link..." : "Generate Schwab login link"}
          </Button>
        ) : (
          <div className="space-y-4">
            <div>
              <label className="mb-1 block text-[12px] font-medium text-nest-foreground">
                Authorization URL
              </label>
              <div className="break-all rounded-nest-md border border-nest-border bg-nest-background p-2 text-[11px] text-nest-muted">
                {url}
              </div>
              <p className="mt-1 text-[11px] text-nest-muted">
                Open this URL in your browser, log in to Schwab, then paste the authorization code
                from the callback URL below.
              </p>
            </div>

            <div>
              <label className="mb-1 block text-[12px] font-medium text-nest-foreground">
                Authorization code
              </label>
              <input
                type="text"
                value={code}
                onChange={(e) => setCode(e.target.value)}
                placeholder="Paste code from callback URL"
                className="w-full rounded-nest-md border border-nest-border bg-nest-background px-3 py-2 text-[12px] outline-none focus:ring-2 focus:ring-nest-primary/50"
              />
            </div>

            <Button
              onClick={handleSubmit}
              disabled={loading || !code.trim()}
              variant="contained"
              className="w-full"
            >
              {loading ? "Connecting..." : "Connect account"}
            </Button>
          </div>
        )}

        {error ? <p className="text-[12px] text-nest-error">{error}</p> : null}
      </div>
    </div>
  );
}
