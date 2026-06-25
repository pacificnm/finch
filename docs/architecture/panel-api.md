# Panel API Specification

## Purpose

The Panel API defines the contract for every visible unit inside Finch. Panels are the building blocks of the user interface. Terminals, file browsers, Git views, logs, metrics, task lists, plugin views, and AI chat interfaces should all be represented as panels.

The Panel API sits between the Workspace Manager, Rendering Pipeline, Event Bus, Command System, and Plugin SDK.

## Goals

- Provide a stable interface for first-party and third-party panels.
- Keep panel behavior separate from workspace layout.
- Support keyboard-first interaction.
- Support focus, rendering, resizing, lifecycle hooks, and state persistence.
- Allow panels to publish and consume approved events.
- Make panels testable outside the full Finch runtime.
- Support both built-in panels and plugin-provided panels.

## Non-Goals for MVP

- Arbitrary graphical widgets outside terminal constraints.
- Direct panel-to-panel mutation.
- Plugin panels with unrestricted host access.
- Complex animation frameworks.

## Core Concepts

### Panel

A panel is a stateful, renderable component placed inside a workspace layout container.

Examples:

- Terminal panel
- File browser panel
- Git panel
- Logs panel
- Metrics panel
- Notifications panel
- AI chat panel
- Plugin-provided panel

### Panel Type

A panel type describes the kind of panel. Multiple panel instances may share the same type.

Example:

- `terminal`
- `file_browser`
- `git`
- `kubernetes`
- `plugin.example.dashboard`

### Panel Instance

A panel instance is a specific running copy of a panel type. Each instance has its own ID, state, focus status, and lifecycle.

### Panel Context

The panel context gives a panel controlled access to Finch services such as events, commands, configuration, theme data, and persistence.

## Responsibilities

Panels are responsible for:

- Rendering their own content into an assigned terminal area.
- Handling input when focused.
- Responding to lifecycle hooks.
- Managing panel-local state.
- Requesting commands or events through approved APIs.
- Saving and restoring panel-local state when supported.

Panels are not responsible for:

- Owning workspace layout.
- Directly changing other panels.
- Managing global application state.
- Rendering outside their assigned area.
- Blocking the UI thread with long-running work.

## High-Level Architecture

```text
+--------------------------------------------------+
|                    Panel API                     |
+--------------------------------------------------+
| Lifecycle | Render | Input | Focus | Persistence |
+--------------------------------------------------+
        |               |               |
        v               v               v
 Workspace Manager  Rendering Pipeline  Event Bus
        |
        v
 Plugin SDK
```

## Suggested Rust Trait

```rust
pub trait Panel {
    fn id(&self) -> PanelId;
    fn panel_type(&self) -> PanelType;
    fn title(&self) -> String;

    fn lifecycle(&mut self, event: PanelLifecycleEvent, ctx: &mut PanelContext) -> Result<()>;
    fn handle_input(&mut self, input: InputEvent, ctx: &mut PanelContext) -> Result<PanelAction>;
    fn update(&mut self, event: Event, ctx: &mut PanelContext) -> Result<PanelAction>;
    fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &PanelRenderContext) -> Result<()>;

    fn serialize_state(&self) -> Result<Option<PanelState>>;
    fn restore_state(&mut self, state: PanelState) -> Result<()>;
}
```

This is an initial direction, not a frozen API. The final trait should be adjusted once the rendering pipeline and plugin model are finalized.

## Panel Lifecycle

Panels should support a predictable lifecycle.

```text
Registered -> Created -> Mounted -> Focused -> Updated -> Blurred -> Unmounted -> Destroyed
```

Lifecycle events:

- `Created`
- `Mounted`
- `Focused`
- `Blurred`
- `Resized`
- `Suspended`
- `Resumed`
- `Unmounted`
- `Destroyed`

## Rendering Contract

A panel receives a rectangular area from the Rendering Pipeline. The panel must render only inside that area.

Rules:

- Panels must not draw outside their assigned rectangle.
- Panels should respect the active theme.
- Panels should support small terminal sizes gracefully.
- Panels should avoid expensive rendering work during every frame.
- Panels should request redraws through the event bus instead of forcing global redraws directly.

## Input Handling

Only the focused panel receives normal keyboard input.

Input categories:

- Character input
- Key combinations
- Navigation keys
- Mouse events when enabled
- Paste events
- Terminal resize notifications

Panels may return actions such as:

- No action
- Request redraw
- Request focus change
- Execute command
- Emit event
- Open panel
- Close panel
- Show notification

## Focus Management

The Workspace Manager owns focus state. Panels may request focus changes, but they should not directly assign focus.

Focus rules:

- Focused panels receive input.
- Blurred panels may continue background updates if allowed.
- A panel should receive lifecycle events when focus changes.
- Focus changes should publish events for interested services.

## State Persistence

Panels may optionally serialize local state.

Examples of panel state:

- Current directory
- Scroll offset
- Selected item
- Open file path
- Active filter
- Panel-specific settings

Panels should not persist secrets unless the user explicitly configures that behavior.

## Panel Actions

Panel methods should return structured actions instead of directly mutating global systems.

Example actions:

- `None`
- `Redraw`
- `Command(CommandId)`
- `Event(Event)`
- `OpenPanel(PanelType)`
- `CloseSelf`
- `Notification(Notification)`
- `RequestFocus(PanelId)`

This keeps panels isolated and makes behavior easier to test.

## Events

Panels may publish events such as:

- `PanelCreated`
- `PanelMounted`
- `PanelFocused`
- `PanelBlurred`
- `PanelResized`
- `PanelStateChanged`
- `PanelRequestedRedraw`
- `PanelClosed`
- `PanelError`

Panels may subscribe to approved events through the Panel Context.

## Plugin Panels

Plugin-provided panels should use the same conceptual API as first-party panels, but access should be restricted by plugin capabilities.

Plugin panel capabilities may include:

- Subscribe to selected event categories
- Publish selected event categories
- Read plugin configuration
- Store plugin state
- Execute approved commands
- Request network access if granted
- Request filesystem access if granted

Plugins should not receive unrestricted access to Finch internals.

## Error Handling

Panel errors should be contained.

Rules:

- A panel error should not crash Finch.
- Repeated panel errors may disable the panel instance.
- Plugin panel errors should be reported to the Plugin Manager.
- Core panel errors should be reported to diagnostics.
- User-facing errors should appear as notifications when appropriate.

## Performance Considerations

- Panel rendering should be bounded and predictable.
- Long-running work should happen in background tasks.
- Panels should avoid blocking input handling.
- Panels should support redraw invalidation instead of continuous redraws.
- Large data views should support pagination, virtual scrolling, or incremental loading.

## Testing Strategy

Panels should be testable with mocked contexts.

Recommended tests:

- Lifecycle transitions
- Input handling
- Render behavior under small sizes
- State serialization and restoration
- Error handling
- Event publication
- Command action generation

## MVP Requirements

The first implementation should support:

- A stable `Panel` trait or equivalent abstraction.
- Panel IDs and panel types.
- Lifecycle events.
- Rendering into assigned Ratatui areas.
- Focus-aware input handling.
- Structured panel actions.
- Basic state persistence.
- First-party terminal or placeholder panel.

## Future Enhancements

- Plugin panel ABI or SDK wrapper.
- Panel permissions model.
- Floating panel support.
- Panel previews.
- Panel templates.
- Panel-local command palettes.
- Panel-specific keybinding overrides.
- Advanced mouse support.

## Open Questions

- Should the Panel API use async traits or keep rendering/input synchronous?
- Should plugin panels run in-process, out-of-process, or both?
- Should panel state be stored inside workspace files or separate panel state files?
- How should Finch handle panels that require background workers?
- Should panel rendering be allowed to cache intermediate views?
