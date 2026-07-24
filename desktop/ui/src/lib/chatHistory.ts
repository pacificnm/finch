import { invoke } from "@tauri-apps/api/core";

/** One persisted AI chat message for a symbol. */
export type ChatMessageRow = {
  id: number;
  symbol: string;
  role: "user" | "assistant" | "error";
  content: string;
  created_at: string;
};

/** Fetches persisted chat history for `symbol`, oldest first. */
export async function fetchChatHistory(symbol: string): Promise<ChatMessageRow[]> {
  return invoke("plugin:finch|ai_chat_history", { symbol });
}

/** Deletes all persisted chat history for `symbol`. */
export async function clearChatHistory(symbol: string): Promise<void> {
  return invoke("plugin:finch|ai_chat_clear", { symbol });
}
