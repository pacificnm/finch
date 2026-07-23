import { useEffect, useRef, useState } from "react";
import { Alert, Button, CircularProgress, TextField, Typography } from "@nest/components";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { askStockQuestion } from "../../lib/nest";

export type AiChatPanelProps = {
  /** The symbol currently loaded in the Trade screen. */
  symbol: string;
  /** Called when the user accepts an AI-generated trade setup. */
  onTradeSetup?: (setup: TradeSetup) => void;
};

export type TradeSetup = {
  symbol: string;
  entry: number;
  stop: number;
  target: number;
  shares: number;
  risk: number;
  reward: number;
};

type ChatMessage = {
  id: number;
  role: "user" | "assistant" | "error";
  content: string;
};

/**
 * Chat-style AI assistant scoped to the symbol loaded in the Trade screen.
 * Answers stream in token-by-token via the `ai-chat-*` events emitted by the
 * `ask_stock_question` Tauri command (see `lib/nest.ts`).
 */
const TRADE_SETUP_REGEX = /<TRADE_SETUP>([\s\S]*?)<\/TRADE_SETUP>/;

export function AiChatPanel({ symbol, onTradeSetup }: AiChatPanelProps) {
  const [question, setQuestion] = useState("");
  const [loading, setLoading] = useState(false);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [pendingSetup, setPendingSetup] = useState<TradeSetup | null>(null);
  const nextId = useRef(0);
  const scrollRef = useRef<HTMLDivElement>(null);
  const stopRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    const el = scrollRef.current;
    // jsdom (used by this component's own tests) doesn't implement
    // scrollTo at all — guard rather than let it throw there.
    if (el && typeof el.scrollTo === "function") {
      el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
    }
  }, [messages, loading]);

  // Clear chat when symbol changes (new stock = fresh conversation)
  useEffect(() => {
    setMessages([]);
    setQuestion("");
    setLoading(false);
    stopRef.current?.();
  }, [symbol]);

  // Stop listening for a still-in-flight answer if the panel unmounts (e.g.
  // the user navigates away mid-stream).
  useEffect(() => {
    return () => {
      stopRef.current?.();
    };
  }, []);

  function appendMessage(role: ChatMessage["role"], content: string): number {
    const id = nextId.current;
    nextId.current += 1;
    setMessages((current) => [...current, { id, role, content }]);
    return id;
  }

  function appendToMessage(id: number, delta: string) {
    setMessages((current) =>
      current.map((message) =>
        message.id === id ? { ...message, content: message.content + delta } : message,
      ),
    );
  }

  function parseTradeSetup(content: string): TradeSetup | null {
    const match = TRADE_SETUP_REGEX.exec(content);
    if (!match || !match[1]) return null;
    try {
      const parsed = JSON.parse(match[1].trim()) as unknown;
      if (
        parsed &&
        typeof parsed === "object" &&
        "symbol" in parsed &&
        "entry" in parsed &&
        "stop" in parsed &&
        "target" in parsed &&
        "shares" in parsed
      ) {
        return parsed as TradeSetup;
      }
    } catch {
      // Ignore malformed JSON.
    }
    return null;
  }

  function handleAsk() {
    const trimmed = question.trim();
    if (trimmed === "" || loading) {
      return;
    }
    setQuestion("");
    setLoading(true);
    appendMessage("user", trimmed);
    const assistantId = appendMessage("assistant", "");

    void askStockQuestion(symbol, trimmed, {
      onChunk: (delta) => appendToMessage(assistantId, delta),
      onDone: () => {
        setLoading(false);
        setMessages((current) => {
          const assistantMessage = current.find((m) => m.id === assistantId);
          if (assistantMessage) {
            const setup = parseTradeSetup(assistantMessage.content);
            if (setup) {
              setPendingSetup(setup);
            }
          }
          return current;
        });
      },
      onError: (message) => {
        setMessages((current) => current.filter((m) => m.id !== assistantId));
        appendMessage("error", message);
        setLoading(false);
        setPendingSetup(null);
      },
    }).then((stop) => {
      stopRef.current = stop;
    });
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 p-3">
      <Typography variant="subtitle2">Ask about {symbol}</Typography>

      <div ref={scrollRef} className="flex min-h-0 flex-1 flex-col gap-3 overflow-auto">
        {messages.length === 0 && (
          <Typography variant="body2" className="text-nest-muted">
            Ask a question about {symbol}, e.g. &quot;how has it performed this month?&quot;
          </Typography>
        )}
        {messages.map((message) => (
          <ChatBubble key={message.id} message={message} />
        ))}
        {loading && (
          <div data-testid="ai-chat-loading">
            <CircularProgress size="small" />
          </div>
        )}
      </div>

      {pendingSetup && (
        <div className="shrink-0 rounded-nest-md border border-nest-primary bg-nest-primary/10 p-3 text-[11px]">
          <p className="mb-2 font-medium text-nest-primary">
            AI trade setup ready for {pendingSetup.symbol}
          </p>
          <p className="text-nest-foreground">
            Entry ${pendingSetup.entry.toFixed(2)} · Stop ${pendingSetup.stop.toFixed(2)} · Target ${pendingSetup.target.toFixed(2)} · {pendingSetup.shares} shares
          </p>
          <div className="mt-2 flex gap-2">
            <Button
              variant="contained"
              size="small"
              onClick={() => {
                onTradeSetup?.(pendingSetup);
                setPendingSetup(null);
              }}
            >
              Populate order ticket
            </Button>
            <Button
              variant="outlined"
              size="small"
              onClick={() => setPendingSetup(null)}
            >
              Dismiss
            </Button>
          </div>
        </div>
      )}

      <div className="flex shrink-0 flex-col gap-2 border-t border-nest-border pt-3">
        <TextField
          label="Ask a question"
          multiline
          rows={2}
          value={question}
          onChange={(event) => setQuestion(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              handleAsk();
            }
          }}
          placeholder={`e.g. why did ${symbol} move today?`}
        />
        <div>
          <Button
            variant="contained"
            disabled={loading || question.trim() === ""}
            onClick={handleAsk}
          >
            Ask
          </Button>
        </div>
      </div>
    </div>
  );
}

function ChatBubble({ message }: { message: ChatMessage }) {
  if (message.role === "user") {
    return (
      <div className="flex justify-end">
        <div className="max-w-[85%] rounded-nest-md bg-nest-primary px-3 py-2 text-sm text-white">
          {message.content}
        </div>
      </div>
    );
  }

  if (message.role === "error") {
    return <Alert severity="error">{message.content}</Alert>;
  }

  return (
    <article
      className="nest-rich-text max-w-[95%] rounded-nest-md border border-nest-border bg-nest-surface p-3"
      data-testid="ai-chat-response"
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{message.content || "…"}</ReactMarkdown>
    </article>
  );
}
