import { useState } from "react";
import { Button } from "@nest/components";
import { schwabAuthLogin } from "../lib/nest";

export type LoginScreenProps = {
  onLoggedIn: () => void;
};

export function LoginScreen({ onLoggedIn }: LoginScreenProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLogin = async () => {
    setLoading(true);
    setError(null);
    try {
      await schwabAuthLogin();
      onLoggedIn();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
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

        <Button onClick={handleLogin} disabled={loading} variant="contained" className="w-full">
          {loading ? "Waiting for Schwab login..." : "Log in with Schwab"}
        </Button>

        {loading ? (
          <p className="mt-3 text-[11px] text-nest-muted">
            Your browser will open to Schwab's login page. After logging in it will show a
            certificate warning on the redirect back to Finch — that's expected, click through
            it. This window will update automatically once you're logged in.
          </p>
        ) : null}

        {error ? <p className="mt-3 text-[12px] text-nest-error">{error}</p> : null}
      </div>
    </div>
  );
}
