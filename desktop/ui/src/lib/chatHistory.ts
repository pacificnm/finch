import { invoke } from "@tauri-apps/api/core";

/** One persisted AI chat message within a session. */
export type ChatMessageRow = {
  id: number;
  session_id: number;
  symbol: string;
  role: "user" | "assistant" | "error";
  content: string;
  created_at: string;
};

/** One past conversation, as shown in the history picker. */
export type SessionSummary = {
  id: number;
  symbol: string;
  started_at: string;
  message_count: number;
  preview: string | null;
};

/** Fetches `symbol`'s current (most recent) session's messages, oldest first. */
export async function fetchCurrentSession(symbol: string): Promise<ChatMessageRow[]> {
  return invoke("plugin:finch|ai_chat_current_session", { symbol });
}

/** Starts a new session for `symbol` — prior sessions stay intact and browsable. */
export async function startNewSession(symbol: string): Promise<void> {
  return invoke("plugin:finch|ai_chat_start_new_session", { symbol });
}

/** Lists `symbol`'s past sessions, most recent first. */
export async function fetchSessions(symbol: string): Promise<SessionSummary[]> {
  return invoke("plugin:finch|ai_chat_sessions", { symbol });
}

/** Fetches one past session's messages, oldest first. */
export async function fetchSessionMessages(sessionId: number): Promise<ChatMessageRow[]> {
  return invoke("plugin:finch|ai_chat_session_messages", { sessionId });
}
