import { useState } from "react";
import { ChevronDown, Minus, Plus, X } from "lucide-react";

export type OrderTicketProps = {
  symbol: string;
  defaultQuantity?: number;
  defaultPrice?: string;
};

export function OrderTicket({ symbol, defaultQuantity = 1, defaultPrice = "34.17" }: OrderTicketProps) {
  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [quantity, setQuantity] = useState(defaultQuantity);
  const [price, setPrice] = useState(defaultPrice);
  const [orderType, setOrderType] = useState("LIMIT");
  const [tif, setTif] = useState("Day");

  return (
    <div className="rounded-nest-md border border-nest-border bg-nest-surface">
      <div className="flex items-center justify-between border-b border-nest-border px-3 py-2">
        <div className="flex items-center gap-2">
          <span
            className={`size-2 rounded-full ${side === "buy" ? "bg-nest-success" : "bg-nest-error"}`}
          />
          <span className="text-[12px] font-semibold">
            {side === "buy" ? "Buy" : "Sell"} {quantity} {symbol} @{price} LIMIT {tif}
          </span>
        </div>
        <button
          type="button"
          className="text-nest-muted hover:text-nest-foreground"
          title="Close order ticket"
        >
          <X className="size-4" />
        </button>
      </div>

      <div className="space-y-3 p-3">
        <div className="flex items-center gap-3">
          <div className="flex overflow-hidden rounded-nest-md border border-nest-border text-[12px] font-medium">
            <button
              type="button"
              onClick={() => setSide("sell")}
              className={`px-4 py-1 ${
                side === "sell" ? "bg-nest-error text-white" : "text-nest-muted hover:text-nest-foreground"
              }`}
            >
              Sell
            </button>
            <button
              type="button"
              onClick={() => setSide("buy")}
              className={`px-4 py-1 ${
                side === "buy" ? "bg-nest-success text-white" : "text-nest-muted hover:text-nest-foreground"
              }`}
            >
              Buy
            </button>
          </div>

          <div className="flex items-center rounded-nest-md border border-nest-border px-2 py-1 text-[12px]">
            <span className="pr-1 text-nest-muted">#</span>
            <input
              type="number"
              min={1}
              value={quantity}
              onChange={(e) => setQuantity(Math.max(1, parseInt(e.target.value, 10) || 0))}
              className="w-12 bg-transparent text-right outline-none"
            />
          </div>
        </div>

        <div className="flex items-center gap-4 text-[11px]">
          <div className="flex flex-1 items-center gap-2">
            <span className="text-nest-muted">Bid</span>
            <span className="font-medium">34.12</span>
            <div className="relative flex-1">
              <input
                type="range"
                min={0}
                max={100}
                className="w-full accent-nest-primary"
              />
              <div className="absolute left-1/2 top-full mt-0.5 -translate-x-1/2 text-[10px] text-nest-muted">
                34.15
              </div>
            </div>
            <span className="font-medium">34.17</span>
            <span className="text-nest-muted">Ask</span>
          </div>

          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => setPrice((current) => (parseFloat(current) - 0.01).toFixed(2))}
              className="rounded-nest-md border border-nest-border p-1 hover:bg-nest-muted/10"
            >
              <Minus className="size-3" />
            </button>
            <input
              type="text"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              className="w-16 rounded-nest-md border border-nest-border bg-nest-background px-2 py-1 text-center text-[12px] outline-none"
            />
            <button
              type="button"
              onClick={() => setPrice((current) => (parseFloat(current) + 0.01).toFixed(2))}
              className="rounded-nest-md border border-nest-border p-1 hover:bg-nest-muted/10"
            >
              <Plus className="size-3" />
            </button>
          </div>

          <div className="relative">
            <select
              value={orderType}
              onChange={(e) => setOrderType(e.target.value)}
              className="appearance-none rounded-nest-md border border-nest-border bg-nest-background px-3 py-1 pr-7 text-[12px] outline-none"
            >
              <option>LIMIT</option>
              <option>MARKET</option>
              <option>STOP</option>
              <option>STOP LIMIT</option>
            </select>
            <ChevronDown className="pointer-events-none absolute right-2 top-1/2 size-3 -translate-y-1/2 text-nest-muted" />
          </div>

          <div className="relative">
            <select
              value={tif}
              onChange={(e) => setTif(e.target.value)}
              className="appearance-none rounded-nest-md border border-nest-border bg-nest-background px-3 py-1 pr-7 text-[12px] outline-none"
            >
              <option>Day</option>
              <option>GTC</option>
              <option>IOC</option>
              <option>FOK</option>
            </select>
            <ChevronDown className="pointer-events-none absolute right-2 top-1/2 size-3 -translate-y-1/2 text-nest-muted" />
          </div>
        </div>

        <div className="flex items-center gap-3 text-[11px]">
          <OrderTicketButton>Option Leg</OrderTicketButton>
          <OrderTicketButton>Order Rule</OrderTicketButton>
        </div>

        <div className="flex flex-wrap items-center gap-2 border-t border-nest-border pt-2 text-[11px]">
          <OrderTicketButton>Contingent Order</OrderTicketButton>
          <OrderTicketButton>Blast All</OrderTicketButton>
          <OrderTicketButton>OCO</OrderTicketButton>
          <OrderTicketButton>1st trgs Seq</OrderTicketButton>
          <OrderTicketButton>
            Advanced Orders
            <ChevronDown className="size-3" />
          </OrderTicketButton>
        </div>
      </div>

      <div className="flex items-center justify-between border-t border-nest-border px-3 py-2">
        <div className="flex items-center gap-2 text-[12px]">
          <span className="text-nest-muted">1 Selected</span>
          <span className="font-medium">
            {side === "buy" ? "BUY" : "SELL"} +{quantity} {symbol} @{price} {orderType === "LIMIT" ? "LMT" : orderType}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            className="rounded-nest-md border border-nest-border px-4 py-1 text-[12px] font-medium hover:bg-nest-muted/10"
          >
            Delete
          </button>
          <button
            type="button"
            className="rounded-nest-md bg-nest-primary px-4 py-1 text-[12px] font-medium text-white hover:opacity-90"
          >
            Review
          </button>
        </div>
      </div>
    </div>
  );
}

function OrderTicketButton({ children }: { children: React.ReactNode }) {
  return (
    <button
      type="button"
      className="flex items-center gap-1 rounded-nest-md border border-nest-border px-2 py-1 text-nest-foreground hover:bg-nest-muted/10"
    >
      <span className="text-nest-primary">+</span>
      {children}
    </button>
  );
}
