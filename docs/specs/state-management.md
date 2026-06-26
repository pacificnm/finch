# Finch State Management Specification

## Status

v0.2 — Open decisions resolved. See linked ADRs for rationale.

## Purpose

This document defines the initial state management model for Finch, a lightweight, keyboard-first terminal workspace environment built in Rust with Ratatui.

Finch is not just a terminal UI with screens. It is a desktop-like terminal workspace made of panels, workspaces, commands, tasks, notifications, plugins, and background activity. State management needs to keep those pieces predictable without making the app feel heavy or over-engineered.

The state model should answer these questions:

- Where does application state live?
- How do panels update state?
- How do background tasks report results?
- How do plugins interact with state safely?
- How does Finch redraw only what changed?
- How do we persist workspace/session state between launches?

## Goals

- Keep the core application state centralized and inspectable.
- Keep panel-local state isolated from global app state.
- Use explicit events and commands instead of hidden mutation.
- Support background tasks without blocking rendering.
- Support plugins without letting plugins corrupt core state.
- Make redraw behavior predictable and low-flicker.
- Allow workspace/session persistence later without redesigning the core.

## Non-Goals

- Finch will not use a web-style Redux clone directly.
- Panels should not receive unrestricted mutable access to the full app state.
- Plugins should not own the global event loop.
- Persistence does not need to store every runtime detail in v0.1.
- Distributed or multi-user state is out of scope.

## State Layers

Finch state is split into layers:

1. **App State**: global runtime state owned by the core.
2. **Workspace State**: tabs, splits, panel layout, active workspace, and focus.
3. **Panel State**: local state owned by each panel implementation.
4. **Task State**: async jobs, process status, command execution, and results.
5. **Plugin State**: plugin metadata, registered commands, registered panels, and plugin-owned runtime data.
6. **Persistent State**: saved configuration and session/workspace snapshots.

This separation keeps the core stable while allowing panels and plugins to evolve independently.

## Top-Level Shape

The core should maintain a single `AppState` value owned by the application runtime.

```rust
pub struct AppState {
    pub config: ConfigState,
    pub ui: UiState,
    pub workspaces: WorkspaceStore,
    pub panels: PanelStore,
    pub commands: CommandStore,
    pub tasks: TaskStore,
    pub notifications: NotificationStore,
    pub plugins: PluginStore,
    pub session: SessionState,
}
```

The exact field names can change, but the ownership rule should not: core state lives in one predictable place and is changed through actions handled by the core.

## App State

App-level state includes global runtime data that is not owned by any single panel.

Examples:

- Current theme
- Keybinding profile
- Active workspace ID
- Global command palette state
- Notification drawer open/closed
- Focus history
- Registered plugins
- Running tasks
- Global status bar data
- Current mode, such as normal, command, search, or input capture

App state should be serializable where practical, but runtime-only fields may be skipped.

## Workspace State

A workspace is a named environment containing panel layout and focus state.

```rust
pub struct WorkspaceState {
    pub id: WorkspaceId,
    pub name: String,
    pub tabs: Vec<TabState>,
    pub active_tab: TabId,
    pub focus: FocusState,
}
```

Workspace state should track:

- Workspace identity
- Tabs
- Split layout tree
- Panel IDs in each layout node
- Active tab
- Focused panel
- Recently focused panels
- Optional pinned panels
- Optional workspace-specific command context

Workspace state should not store large panel internals. It should store layout and references to panels.

## Panel State

Panels own their own internal state. The core should not know the details of every panel.

Examples:

- File browser cursor position
- Git status selected file
- Terminal scrollback handle
- Log viewer filter text
- GitHub issue list selection
- Plugin dashboard widget state

Panels should expose behavior through a trait and communicate with the core using events and actions.

```rust
pub trait Panel: Send + 'static {
    fn id(&self) -> PanelId;
    fn panel_type(&self) -> &'static str;
    fn title(&self) -> String;
    fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &PanelRenderContext);
    fn handle_input(&mut self, input: InputEvent, ctx: &mut PanelContext) -> PanelAction;
    fn on_event(&mut self, event: &Event, ctx: &mut PanelContext) -> PanelAction { PanelAction::None }
    fn on_mounted(&mut self, ctx: &mut PanelContext) {}
    fn on_unmounted(&mut self, ctx: &mut PanelContext) {}
    fn on_focused(&mut self, ctx: &mut PanelContext) {}
    fn on_blurred(&mut self, ctx: &mut PanelContext) {}
    fn serialize_state(&self) -> Option<toml::Value> { None }
    fn restore_state(&mut self, state: toml::Value) {}
}
```

The full trait definition, `PanelAction` variants, and lifecycle state machine are in `docs/specs/panel-api.md` (ADR-0009). Panels return `PanelAction` rather than raw `AppAction` values so the host can validate and translate requests before applying them to `AppState`. State is serialized as `toml::Value` because panel state files are TOML (ADR-0010) — no conversion step needed.

## Actions

All meaningful state changes should flow through explicit actions.

```rust
pub enum AppAction {
    Quit,
    Redraw,
    OpenCommandPalette,
    CloseCommandPalette,
    SwitchWorkspace(WorkspaceId),
    CreateWorkspace { name: String },
    CloseWorkspace(WorkspaceId),
    FocusPanel(PanelId),
    OpenPanel(OpenPanelRequest),
    ClosePanel(PanelId),
    SplitPanel { direction: SplitDirection, panel: OpenPanelRequest },
    MoveFocus(FocusDirection),
    StartTask(TaskRequest),
    CancelTask(TaskId),
    TaskCompleted(TaskResult),
    Notify(Notification),
    Plugin(PluginAction),
}
```

Actions can be emitted by:

- Keyboard bindings
- Command palette commands
- Panels
- Background tasks
- Plugins
- System events

The app runtime receives actions and applies them to `AppState` through reducers/handlers.

## Reducers / Handlers

Finch should use reducer-style handlers for core state updates.

```rust
pub fn reduce(state: &mut AppState, action: AppAction) -> Vec<Effect> {
    match action {
        AppAction::OpenCommandPalette => {
            state.ui.command_palette.open = true;
            vec![Effect::RequestRedraw]
        }
        AppAction::FocusPanel(panel_id) => {
            state.workspaces.focus_panel(panel_id);
            vec![Effect::RequestRedraw]
        }
        AppAction::StartTask(request) => {
            let task_id = state.tasks.register(request.clone());
            vec![Effect::SpawnTask { task_id, request }]
        }
        _ => vec![],
    }
}
```

Handlers should be boring and explicit. A handler may update state and return side effects, but it should not perform long-running work inline.

## Effects

Effects are requests for the runtime to do something outside pure state mutation.

```rust
pub enum Effect {
    RequestRedraw,
    PersistSession,
    SpawnTask { task_id: TaskId, request: TaskRequest },
    CancelTask(TaskId),
    OpenExternalUrl(String),
    WriteClipboard(String),
    PluginCall(PluginCall),
}
```

Effects are executed by the runtime after the state update completes. This keeps state mutation predictable and makes it easier to test handlers.

## Event Loop Flow

The Finch runtime should follow this general loop:

1. Read terminal input, timer ticks, task messages, and plugin messages.
2. Convert input into one or more `AppAction` values.
3. Apply actions to `AppState` using reducers/handlers.
4. Execute returned effects.
5. Mark dirty regions or request a full redraw.
6. Render the current workspace and active overlays.

The render path should be fast and should not block on IO.

## Background Task State

Tasks represent async work managed by Finch.

Examples:

- Running shell commands
- Polling Git status
- Watching files
- Loading GitHub issues
- Tail-following logs
- Running plugin jobs
- Refreshing system metrics

```rust
pub struct TaskState {
    pub id: TaskId,
    pub label: String,
    pub status: TaskStatus,
    pub owner: TaskOwner,
    pub started_at: std::time::SystemTime,
    pub last_update_at: std::time::SystemTime,
}

pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

Task results should return to the app as actions, not mutate state directly.

```rust
AppAction::TaskCompleted(TaskResult)
```

## Notifications State

Notifications are global app state.

```rust
pub struct Notification {
    pub id: NotificationId,
    pub level: NotificationLevel,
    pub title: String,
    pub message: Option<String>,
    pub source: NotificationSource,
    pub read: bool,
}
```

Notifications can be emitted by panels, tasks, plugins, and core services. The notification drawer reads from `NotificationStore`.

## Focus State

Focus must be explicit because Finch is keyboard-first.

```rust
pub struct FocusState {
    pub focused_panel: Option<PanelId>,
    pub previous_panel: Option<PanelId>,
    pub mode: InputMode,
}

pub enum InputMode {
    Normal,
    CommandPalette,
    Search,
    TextInput,
    PanelCapture,
}
```

Rules:

- Only one panel has keyboard focus at a time.
- Overlays can temporarily capture focus.
- Panels can request focus changes but the host decides.
- Focus history should allow toggling back to the previous panel.
- Global shortcuts are disabled or limited during text input and panel capture.

## Dirty / Redraw State

**ADR-0012 decision: full-frame redraws for the entire MVP phase.**

Ratatui already diffs each render against the previous buffer at the terminal cell level, so full-frame redraws do not cause visible flicker or extra terminal writes — only changed cells are written to stdout. Panel-level dirty tracking adds complexity with no meaningful user benefit in Phase 1.

```rust
pub enum RedrawRequest {
    None,
    Frame,
}
```

The state model uses a single `RedrawRequest` flag. Any event that might affect visible output sets it to `Frame`. The render pass runs and Ratatui handles the diff.

Region-level dirty tracking (per-panel, per-region) may be revisited in a later phase if profiling shows the full-frame approach is a bottleneck. The `DirtyState` enum below is documented for reference but is not implemented in Phase 1.

```rust
// Reserved for future use (post-MVP)
pub enum DirtyState {
    Clean,
    Full,
    Panels(Vec<PanelId>),
    StatusBar,
    Overlay,
}
```

## Plugin State

Plugins can register panels, commands, keybindings, and background tasks. Plugin state should be scoped and controlled.

```rust
pub struct PluginState {
    pub id: PluginId,
    pub name: String,
    pub enabled: bool,
    pub registered_panels: Vec<PanelKind>,
    pub registered_commands: Vec<CommandId>,
}
```

Plugin rules:

- Plugins cannot receive `&mut AppState`.
- Plugins interact through commands, actions, and scoped APIs.
- Plugin tasks report results through the same task/event channel as core tasks.
- Plugin panel state is owned by the plugin panel instance.
- Plugin persistent data should be namespaced by plugin ID.

## Persistence

Finch should eventually persist selected state between launches.

Persisted state may include:

- Config
- Theme
- Keybindings
- Workspaces
- Layout trees
- Open panel kinds and panel restore data
- Last active workspace
- Plugin enabled/disabled state

Persisted state should not include:

- Raw terminal handles
- Running process handles
- Secrets
- Expired task results
- Large scrollback buffers by default
- Unbounded logs

Example snapshot:

```rust
pub struct SessionSnapshot {
    pub version: u32,
    pub active_workspace: WorkspaceId,
    pub workspaces: Vec<WorkspaceSnapshot>,
    pub plugins: Vec<PluginSnapshot>,
}
```

Persistence must be versioned so Finch can migrate session files later.

## Configuration State

Config is user-authored and should be loaded at startup.

Examples:

- Theme
- Keybindings
- Default workspace
- Enabled plugins
- Panel defaults
- Command aliases
- Status bar modules

Config state should be treated as input. Runtime state may derive from it but should not overwrite it unless the user explicitly saves settings.

## Command Palette State

The command palette is a global overlay with its own UI state.

```rust
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub selected_index: usize,
    pub results: Vec<CommandSearchResult>,
}
```

Command execution emits `AppAction` values. Commands should be able to target the focused panel, active workspace, or global app.

## Testing Strategy

State management should be testable without a terminal.

Tests should cover:

- Reducer behavior for each core action
- Focus movement
- Workspace switching
- Panel open/close
- Task lifecycle
- Notification creation/read state
- Session snapshot serialization
- Plugin action boundaries
- Dirty state updates

Reducers should avoid direct terminal IO so unit tests can apply actions and assert state changes.

## Initial Milestones

### Milestone 1: Core State Skeleton

- Add `AppState`
- Add workspace store
- Add panel store
- Add focus state
- Add action enum
- Add basic reducer function

### Milestone 2: Event Loop Integration

- Convert keyboard events to actions
- Route panel events through host
- Add effect execution
- Add redraw requests

### Milestone 3: Task Store

- Add task registry
- Add async task result channel
- Add task lifecycle actions
- Add notifications from failed tasks

### Milestone 4: Session Snapshot

- Define serializable snapshot structs
- Save workspace layout
- Restore open panels that support snapshots
- Version the session file

### Milestone 5: Plugin Boundary

- Add plugin state registry
- Register plugin commands and panels
- Add scoped plugin actions
- Prevent plugins from mutating `AppState` directly

## Resolved Decisions

These questions were open during initial design. All are now closed by ADRs or architecture docs.

**Single `AppAction` enum with nested domain variants.**
A single top-level `AppAction` enum with nested variant groups (e.g., `AppAction::Plugin(PluginAction)`) keeps the reducer match exhaustive and the action surface visible in one place. Domain modules define their own sub-enums. This avoids a proliferation of separate `reduce_workspace`, `reduce_panels`, etc. dispatch functions while still keeping domain logic in separate handler modules.

**Panel state is stored in separate per-panel files, not as inline snapshots.**
`serialize_state()` returns `Option<toml::Value>`. The host serializes it to `~/.local/share/finch/panel-state/<type>-<id>.toml` on exit and reloads it by calling `restore_state()` before `on_mounted`. Panels that cannot restore safely return `None`. (ADR-0010)

**Dirty tracking is not implemented in Phase 1 — full-frame redraws only.**
Ratatui's buffer diffing makes full-frame redraws inexpensive. Region-level dirty tracking is deferred to a post-MVP phase. (ADR-0012)

**Terminal scrollback is not restored across sessions.**
Scrollback buffers (up to 10,000 lines per panel, held in memory) are large and terminal-state-sensitive. They are not included in the session snapshot. The terminal panel starts fresh after restart. The user's shell history is preserved by the shell itself.

**Task state is visible in the status bar by default.**
The status bar's right zone shows a spinner and count for active background tasks. Task results that complete with errors emit a notification. Users can open the notification drawer (`Ctrl+Shift+N`) to see all task history. (see `docs/architecture/status-bar.md`)

**Plugins run in-process as WASM modules in Phase 1.**
Out-of-process plugins are not supported in Phase 1. All plugin execution runs inside the Finch process via Wasmtime with a capability sandbox. Cross-process support is a future consideration requiring a separate IPC transport design. (ADR-0008)

## Decision

Finch will use centralized core state with explicit actions and effects. Panels and plugins will not directly mutate global state. Panel-local state remains owned by panel instances. Background work reports back through actions. This gives Finch a predictable architecture that fits a keyboard-first terminal workspace while leaving room for plugins, persistence, and richer panel behavior later.
