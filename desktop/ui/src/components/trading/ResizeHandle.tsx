import { useCallback, useEffect, useState } from "react";
import { cn } from "@nest/components";

interface ResizeHandleProps {
  /** Called on every mouse move with the delta in pixels. */
  onResize: (delta: number) => void;
  /** Which direction the handle resizes. */
  direction?: "horizontal" | "vertical";
  /** Accessible label for the resize handle. */
  label?: string;
}

/**
 * A simple resize handle for adjusting adjacent panel widths.
 * Dragging resizes in the direction specified; positive delta typically
 * expands the panel after the handle.
 */
export function ResizeHandle({
  onResize,
  direction = "horizontal",
  label = "Resize panel",
}: ResizeHandleProps) {
  const [resizing, setResizing] = useState(false);

  const handleMouseDown = useCallback(() => {
    setResizing(true);
  }, []);

  useEffect(() => {
    if (!resizing) return;

    const handleMouseMove = (event: MouseEvent) => {
      const delta = direction === "horizontal" ? event.movementX : event.movementY;
      onResize(delta);
    };

    const handleMouseUp = () => {
      setResizing(false);
    };

    // Capture on window so dragging continues even if the cursor leaves the handle.
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp, { once: true });

    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
    };
  }, [direction, onResize, resizing]);

  return (
    <div
      role="separator"
      aria-label={label}
      aria-orientation={direction === "horizontal" ? "vertical" : "horizontal"}
      onMouseDown={handleMouseDown}
      className={cn(
        "group relative shrink-0 bg-transparent hover:bg-nest-primary/20 active:bg-nest-primary/40",
        "flex items-center justify-center",
        direction === "horizontal" ? "w-1 cursor-col-resize" : "h-1 cursor-row-resize",
        resizing && "bg-nest-primary/40"
      )}
    >
      <div
        className={cn(
          "rounded-full bg-nest-border group-hover:bg-nest-primary/50",
          direction === "horizontal" ? "h-8 w-px" : "h-px w-8"
        )}
      />
    </div>
  );
}

