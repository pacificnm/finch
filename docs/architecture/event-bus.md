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
    pub id: EventId,
    pub kind: EventKind,
    pub source: EventSource,
    pub priority: EventPriority,
    pub timestamp: DateTime<Utc>,
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

```rust
pub trait EventSubscriber {
    fn id(&self) -> SubscriberId;
    fn subscriptions(&self) -> Vec<EventKind>;
    async fn handle_event(&mut self, event: Event) -> Result<()>;
}
```

Subscriptions may be exact or category-based.

Examples:

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

## Open Questions

- Should plugins receive raw event structs or a restricted SDK wrapper?
- Should event payloads use enums, trait objects, or serialized messages?
- Should the event bus support cross-process plugins in the first release?
