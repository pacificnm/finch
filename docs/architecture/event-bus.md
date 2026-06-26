# Event Bus Specification

## Purpose

The Finch event bus is the internal messaging backbone of the application. It allows core services, panels, plugins, background tasks, and UI components to communicate without tight coupling.

## Goals

- Decouple Finch subsystems.
- Support asynchronous, non-blocking workflows.
- Allow plugins to observe and publish approved events.
- Provide predictable ordering for high-priority events.
- Keep the core responsive under load.

## Event Model

An event is a typed message with metadata and an optional payload.

```rust
pub struct Event {
    pub id: EventId,           // atomic u64 counter, cheap ordering within a session
    pub kind: EventKind,
    pub source: EventSource,
    pub priority: EventPriority,
    pub timestamp: Instant,    // monotonic — not wall clock; see docs/specs/event-bus.md
    pub payload: EventPayload,
}
```

## Event Types

Initial event categories:

- `AppEvent` - startup, shutdown, suspend, resume
- `InputEvent` - keyboard, mouse, paste, terminal resize
- `CommandEvent` - command execution and command completion
- `PanelEvent` - panel open, close, focus, resize, render request
- `WorkspaceEvent` - workspace switch, layout save, layout load
- `PluginEvent` - plugin load, unload, error, message
- `ConfigEvent` - config reload, theme change, keybinding change
- `NotificationEvent` - user-facing notification lifecycle
- `TaskEvent` - task started, progress, completed, failed
- `SystemEvent` - OS signal, filesystem watch, background service update

## Priorities

Events use priority levels to keep the UI responsive.

| Priority | Purpose |
| --- | --- |
| Critical | Shutdown, crash handling, terminal restore |
| High | Input, focus change, command dispatch |
| Normal | Panel updates, plugin messages, notifications |
| Low | Telemetry, background refresh, cache updates |
| Idle | Cleanup, indexing, optional background work |

## Subscriptions

Subscribers register interest in event kinds.

Subscribers receive events through a `tokio::sync::mpsc` channel (see `docs/specs/event-bus.md` for the full subscription API). The bus delivers events by cloning into each subscriber's channel. Subscribers filter by `EventFilter` — see the spec for the filter grammar.

```rust
// Subscription — returns a receiver for the filtered event stream
let rx: mpsc::Receiver<Event> = bus.subscribe(
    SubscriberId::new(),
    EventFilter::Kind(EventKind::Input),
);

// Subscriber loop (typically inside a Tokio task)
while let Some(event) = rx.recv().await {
    // handle event
}
```

Subscriptions may be exact or category-based. Examples:

- Subscribe to all `InputEvent` events.
- Subscribe only to `CommandEvent::Execute`.
- Subscribe to plugin-scoped events for a specific plugin.

## Async Messaging

The event bus should use async channels internally.

Initial implementation recommendation:

- Tokio runtime
- Bounded channels for backpressure
- Priority queues for dispatch ordering
- Dedicated dispatcher task
- Non-blocking handler execution

## Backpressure

When queues fill, Finch should:

1. Always preserve critical and high-priority events.
2. Coalesce repeated render events.
3. Drop idle events before low-priority events.
4. Emit diagnostics when sustained pressure occurs.

## Event Coalescing

Certain events should be merged to reduce unnecessary work.

Examples:

- Multiple resize events become the latest resize event.
- Multiple render requests for the same panel become one render request.
- Repeated status refresh events may be collapsed.

## Plugin Access

Plugins may only publish and subscribe to events allowed by their capability manifest.

Example plugin manifest permission:

```toml
[capabilities.events]
publish = ["notification", "task"]
subscribe = ["workspace", "command"]
```

## Error Handling

A failing subscriber must not crash the event bus.

- Handler errors are captured.
- Plugin errors are routed to the plugin manager.
- Core service errors are routed to diagnostics.
- Repeated failures may disable a subscriber.

## Resolved Decisions

These questions were open during initial design and have since been resolved by ADRs.

**Plugins receive a restricted SDK wrapper.**
Plugins run in-process as WASM modules (ADR-0008). The WASM host does not pass raw event structs across the ABI boundary. Instead it serializes a restricted subset of event data using MessagePack (ADR-0005) and delivers it through the plugin SDK. Plugins cannot observe or emit arbitrary internal events — only what the SDK explicitly exposes.

**Event payloads use a hybrid typed enum.**
`EventPayload` is a Rust enum with concrete variants for all known event kinds plus a `Plugin { payload: Vec<u8> }` escape hatch for plugin-originated data (ADR-0007). Core event matching is zero-cost; plugins pass opaque MessagePack blobs through the escape hatch without polluting the core event types.

**Cross-process plugins are not in the first release.**
All Phase 1 plugins execute in-process (ADR-0008). Cross-process plugin support is deferred until a separate IPC transport design is completed.
