# Rendering Pipeline Specification

## Purpose

The Rendering Pipeline defines how Finch turns application state, workspace layouts, panels, themes, and terminal events into visible frames using Ratatui.

This subsystem is responsible for keeping the terminal UI responsive, predictable, and efficient while allowing panels and plugins to render safely inside their assigned areas.

## Goals

- Use Ratatui as the primary rendering abstraction.
- Compose all visible UI elements into a single terminal frame.
- Keep rendering deterministic and testable.
- Support redraw invalidation instead of unnecessary continuous rendering.
- Prevent panels from rendering outside their assigned areas.
- Support terminal resize handling.
- Support theming and layout-aware rendering.
- Keep the UI responsive under background load.

## Non-Goals for MVP

- GPU acceleration.
- Pixel-perfect graphical rendering.
- Complex animation systems.
- Browser-like layout engines.
- Rendering outside terminal capabilities.

## Core Concepts

### Frame

A frame is one complete render pass to the terminal. Finch should compose the full interface into a frame and then flush it through the terminal backend.

### Render Pass

A render pass is the process of gathering current state, resolving layout, rendering chrome, rendering panels, and presenting the result.

### Invalidation

Invalidation is the process of marking part or all of the UI as needing redraw.

### Dirty Region

A dirty region is an area of the UI that has changed and may need to be redrawn. MVP rendering may redraw the whole frame, but the architecture should leave room for region-based optimizations later.

### Chrome

Chrome refers to the surrounding Finch UI outside panel content, such as top bars, tab bars, borders, docks, status bars, command palette overlays, and notifications.

## High-Level Architecture

```text
+--------------------------------------------------+
|                Rendering Pipeline                |
+--------------------------------------------------+
| Render Scheduler | Layout Resolver | Theme Engine |
| Chrome Renderer  | Panel Renderer  | Overlay Layer |
+--------------------------------------------------+
        |                 |                  |
        v                 v                  v
 Workspace Manager    Panel Manager       Event Bus
        |
        v
 Ratatui Backend / Crossterm Terminal
```

## Rendering Flow

A normal frame should follow this order:

1. Read the latest application state.
2. Resolve active workspace layout.
3. Calculate terminal regions.
4. Render application chrome.
5. Render active workspace tabs and docks.
6. Render visible panels into assigned rectangles.
7. Render overlays such as command palette and notifications.
8. Flush the frame to the terminal backend.

## Render Loop

The render loop should be event-driven rather than constantly redrawing.

Recommended inputs:

- Input events
- Workspace layout changes
- Panel redraw requests
- Theme changes
- Terminal resize events
- Notifications
- Command palette state changes
- Timed ticks for panels that request them

The render loop should sleep when no redraw is required.

## Invalidation Model

Panels and services should request redraws by publishing events rather than directly forcing a render.

Initial invalidation scopes:

- Full application
- Workspace
- Tab
- Panel
- Overlay
- Status bar

MVP may implement full-frame redraws for simplicity while preserving invalidation metadata for future optimization.

## Double Buffering

Ratatui already maintains an internal buffer model. Finch should rely on Ratatui's buffer diffing behavior where possible.

Finch should still track logical invalidation so future optimizations can avoid unnecessary panel updates.

## Layout Resolution

The Rendering Pipeline should not own workspace layout state. It should ask the Workspace Manager for the active layout, then resolve it into concrete terminal rectangles.

Layout resolution should account for:

- Terminal size
- Top bar height
- Status bar height
- Tab bar height
- Dock regions
- Split ratios
- Minimum panel sizes
- Borders and padding
- Active overlays

## Panel Rendering

Panels receive only their assigned rectangle.

Rules:

- A panel must render only inside its rectangle.
- A panel should receive the current theme context.
- A panel should handle small sizes gracefully.
- A panel should avoid blocking during rendering.
- A panel should not directly flush to the terminal.

## Chrome Rendering

The core renderer is responsible for Finch chrome.

Chrome includes:

- Top bar
- Tab bar
- Workspace indicator
- Panel borders
- Dock separators
- Status bar
- Notification area
- Modal and overlay containers
- Command palette container

Chrome should be consistent across panels and plugins.

## Overlay Rendering

Overlays are rendered above the normal workspace layout.

Examples:

- Command palette
- Help screen
- Notifications
- Confirmation prompts
- Quick switcher
- Plugin picker

Overlay rendering should be ordered by z-index or a simple stack model.

## Terminal Resize Handling

Resize events should be coalesced before rendering.

On resize:

1. Update terminal dimensions.
2. Invalidate layout geometry.
3. Notify Workspace Manager and visible panels.
4. Recalculate layout regions.
5. Render a new frame.

Panels should receive resize lifecycle events when their assigned area changes.

## Scheduling

The Render Scheduler decides when to render.

Render triggers:

- Immediate render after high-priority input.
- Debounced render after resize events.
- Scheduled render for ticking panels.
- Render after state changes.
- Render after overlay changes.

The scheduler should avoid excessive redraws when many events arrive together.

## Events

The Rendering Pipeline should subscribe to:

- Input events
- Workspace events
- Panel redraw events
- Theme events
- Notification events
- Command palette events
- Terminal resize events

The Rendering Pipeline may publish:

- RenderStarted
- RenderCompleted
- RenderSkipped
- RenderFailed
- FrameRateUpdated
- LayoutResolved

## Error Handling

Rendering errors should be contained.

Rules:

- A panel render failure should not crash Finch.
- Finch should render a fallback error view for failed panels.
- Repeated panel render failures may disable that panel instance.
- Terminal backend failures should trigger safe shutdown and terminal restoration.
- Rendering diagnostics should be available in logs.

## Performance Considerations

- Avoid continuous redraw when idle.
- Coalesce repeated render requests.
- Avoid blocking IO in render methods.
- Cache resolved layout geometry when possible.
- Use incremental state updates where practical.
- Keep overlay rendering lightweight.
- Measure frame time and expose diagnostics.

## Testing Strategy

Rendering should be testable without a real terminal where possible.

Recommended tests:

- Layout resolution with different terminal sizes.
- Panel rectangle assignment.
- Overlay stacking order.
- Resize handling.
- Render invalidation behavior.
- Error fallback rendering.
- Theme application.

## MVP Requirements

The first implementation should support:

- Ratatui rendering with Crossterm backend.
- Full-frame redraws.
- Top bar, tab bar, panel area, and status bar.
- Rendering visible panels into assigned rectangles.
- Terminal resize handling.
- Basic redraw events.
- Command palette overlay.
- Notification overlay or status messages.

## Future Enhancements

- Dirty region optimization.
- Render performance dashboard.
- Panel render profiling.
- Optional animation support.
- Advanced overlay stack.
- Theme hot reload.
- Snapshot tests for UI layouts.
- Remote rendering experiments.

## Open Questions

- Should Finch start with full-frame redraws only, or include dirty region tracking from the beginning?
- Should panels render synchronously while background data loads asynchronously?
- How should frame timing diagnostics be exposed to users?
- Should themes be resolved before rendering or lazily during widget creation?
- How much overlay control should plugins receive?
