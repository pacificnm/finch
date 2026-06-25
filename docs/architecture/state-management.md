# State Management Specification

## Purpose

The State Manager defines how Finch stores, owns, updates, snapshots, and restores runtime state.

State management is central to Finch because nearly every subsystem depends on predictable state transitions: workspaces, panels, rendering, commands, configuration, notifications, plugins, tasks, and session restoration.

## Goals

- Maintain clear ownership of application state.
- Avoid hidden global mutable state.
- Support predictable and testable state transitions.
- Keep state changes observable through events.
- Support session restoration.
- Support plugin-owned state without allowing plugins to mutate Finch internals directly.
- Enable efficient redraws through meaningful state change events.
- Keep persistence simple for the MVP.

## Non-Goals for MVP

- Full immutable application state as the default model.
- Time-travel debugging.
- Incremental persistence.
- Cloud-synchronized state.
- Developer state inspector panel.
- Cross-device session sync.

These may be added later, but the MVP should use a simple, reliable model first.

## Core Principles

### One Owner Per State Domain

Every piece of state should have one authoritative owner.

Examples:

- Workspace state is owned by the Workspace Manager.
- Panel-local state is owned by the panel instance.
- Plugin state is owned by the plugin runtime or plugin manager.
- Theme state is owned by the Theme Engine.
- Command state is owned by the Command System.
- Notification state is owned by the Notification System.

Other systems request changes through commands, APIs, or events.

### Controlled Mutation

Finch will use controlled mutable state for the MVP. State may be updated by the owning subsystem, but not freely mutated by unrelated systems.

This avoids overengineering the first implementation while still preserving clear architectural boundaries.

### Event-Visible Changes

Meaningful state changes should emit events so other subsystems can react.

Example flow:

```text
Input Event
  -> Command System
  -> Workspace Manager
  -> Workspace State Updated
  -> WorkspaceChanged Event
  -> Rendering Pipeline Redraw
```

## High-Level Architecture

```text
+--------------------------------------------------+
|                  State Manager                   |
+--------------------------------------------------+
| App State | Session State | State Snapshots       |
| Registry  | Persistence   | Migration Hooks       |
+--------------------------------------------------+
        |                |                 |
        v                v                 v
 Workspace Manager   Plugin Manager     Event Bus
        |
        v
 Rendering Pipeline
```

## State Domains

### Application State

Application state describes the lifecycle of Finch itself.

Examples:

- Starting
- Ready
- Suspended
- Shutting down
- Error state
- Diagnostics state

### Workspace State

Workspace state includes:

- Active workspace
- Active tab
- Layout tree
- Focus state
- Dock state
- Session restoration metadata

The Workspace Manager owns this state.

### Panel State

Panel state is local to a panel instance.

Examples:

- Scroll offset
- Selected row
- Current path
- Active filter
- Cursor position
- View mode

Panels may serialize their own state through the Panel API.

### Plugin State

Plugin state belongs to a plugin and should be isolated from other plugins.

Plugin state may include:

- Plugin configuration
- Runtime state
- Cached data
- Persisted user state
- Plugin schema version

### Theme State

Theme state includes:

- Active theme name
- Color palette
- Text styles
- Border styles
- Status styles

### Notification State

Notification state includes:

- Active notifications
- Queued notifications
- Dismissed notifications
- Optional notification history

### Command State

Command state includes:

- Registered commands
- Command palette state
- Active command context
- Keybinding mappings

## Suggested Rust Structure

```rust
pub struct GlobalState {
    pub app: AppState,
    pub session: SessionState,
    pub workspaces: WorkspaceStateRef,
    pub theme: ThemeStateRef,
    pub notifications: NotificationStateRef,
    pub commands: CommandStateRef,
    pub plugins: PluginStateRegistry,
}
```

The final implementation may split these across crates, but the ownership model should remain explicit.

## State Ownership Rules

| State Domain | Owner |
| --- | --- |
| Application lifecycle | Finch Core |
| Workspace layout and focus | Workspace Manager |
| Panel-local state | Panel instance |
| Theme | Theme Engine |
| Commands and keybindings | Command System |
| Notifications | Notification System |
| Plugin runtime state | Plugin Manager |
| Plugin-local persisted state | Plugin |
| Configuration | Configuration System |

## State Transitions

State transitions should be explicit and testable.

A typical transition should follow this pattern:

1. Receive command, event, or API request.
2. Validate the request.
3. Ask the owning subsystem to apply the change.
4. Update state atomically where practical.
5. Publish a state change event.
6. Trigger redraw or downstream processing if needed.

## Session Restoration

Session restoration should be simple for the MVP.

At shutdown, Finch should persist the complete session.

Session data may include:

- Active workspace
- Open tabs
- Layout trees
- Focus state
- Panel states
- Theme selection
- Plugin state references

At startup, Finch should:

1. Load configuration.
2. Load session state.
3. Restore workspaces.
4. Restore panels.
5. Restore plugin state where available.
6. Publish a session restored event.
7. Render the first frame.

## Architecture Decisions

### Immutable Snapshots

Finch will not use immutable snapshots as the default internal representation for the MVP.

The MVP will use controlled mutable state owned by well-defined subsystems.

Immutable snapshots may be introduced later for:

- Debugging
- Diagnostics
- Undo and redo
- Render diffing
- Crash recovery
- State inspection

### Developer State Inspector

A developer-facing State Inspector is planned but is outside the MVP scope.

When implemented, it should expose runtime information for:

- Global application state
- Workspaces
- Panels
- Events
- Tasks
- Plugins
- Notifications
- Commands

The architecture should leave room for this feature, but the first implementation should not depend on it.

### Plugin State Versioning

Plugins that persist state must include a schema version.

When a plugin is updated, Finch should allow the plugin to migrate older state into the current format before restoring it.

A plugin state file should include:

- Plugin identifier
- Plugin version
- State schema version
- Persisted plugin state

### Session Persistence

The MVP will use full-session persistence.

Incremental persistence may be introduced later if profiling shows full-session saves become too slow or too large.

## Persistence Model

The initial persistence model should favor simplicity.

Recommended persisted files:

```text
~/.config/finch/session.toml
~/.config/finch/workspaces/*.toml
~/.config/finch/plugins/<plugin-id>/state.toml
```

Sensitive information should not be persisted unless explicitly permitted by the user and supported by a secure storage strategy.

## Events

The State Manager or owning subsystems may publish:

- `StateChanged`
- `SessionLoaded`
- `SessionSaved`
- `SessionRestoreFailed`
- `WorkspaceStateChanged`
- `PanelStateChanged`
- `PluginStateChanged`
- `ThemeStateChanged`
- `NotificationStateChanged`

The State Manager may subscribe to:

- Application lifecycle events
- Workspace events
- Panel lifecycle events
- Plugin events
- Configuration reload events
- Shutdown events

## Thread Safety

State should be safe to access in async contexts.

Guidelines:

- Avoid long-held locks.
- Avoid blocking IO while holding state locks.
- Prefer ownership boundaries over shared mutable access.
- Use async-aware synchronization where needed.
- Keep render-facing snapshots lightweight.

Possible Rust tools:

- `Arc`
- `RwLock`
- `tokio::sync`
- immutable clones for render-safe snapshots

## Error Handling

State failures should be contained and observable.

Examples:

- Session restore failure should fall back to a default workspace.
- Plugin state migration failure should disable or reset only that plugin state.
- Panel state restore failure should create a fresh panel instance.
- Persistence failure should notify the user and log diagnostics.

## Performance Considerations

- Avoid unnecessary cloning of large state objects.
- Keep state domains granular.
- Batch related state updates.
- Emit meaningful change events instead of noisy updates.
- Persist on shutdown first; add periodic or incremental persistence later.
- Keep render snapshots small.

## Testing Strategy

Recommended tests:

- State ownership boundaries
- Workspace state transitions
- Session save and restore
- Plugin state version handling
- Failed session restore fallback
- Panel state serialization
- Event publication after state changes
- Concurrent access behavior

## MVP Requirements

The first implementation should support:

- Global application state.
- Workspace state references.
- Panel-local state persistence hooks.
- Plugin state version metadata.
- Full-session save and restore.
- State change events.
- Safe fallback when restore fails.

## Future Enhancements

- Immutable state snapshots.
- Developer State Inspector panel.
- Time-travel debugging.
- Undo and redo support.
- Incremental persistence.
- Crash recovery snapshots.
- State diff visualization.
- Multi-session support.
- Cloud-synchronized session state.

## Open Questions

- What state should be considered safe to persist by default?
- Should plugin state be stored in TOML, JSON, or plugin-defined formats?
- Should session restoration be automatic or user-configurable?
- How often should Finch autosave state after the MVP?
