/** Shape of a serialized `NestError` rejected from a Tauri command. */
type NestErrorLike = {
  message?: string;
  code?: string | null;
  module?: string | null;
  kind?: string;
};

/** Renders an IPC rejection (NestError object, Error, or string) as text. */
export function formatIpcError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object") {
    const nest = error as NestErrorLike;
    if (typeof nest.message === "string" && nest.message.length > 0) {
      const code = nest.code ? ` [${nest.code}]` : "";
      return `${nest.message}${code}`;
    }
    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }
  return String(error);
}
