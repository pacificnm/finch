# Workspace Manager Specification

## Purpose

The Workspace Manager controls how users organize work inside Finch. It manages workspaces, tabs, split layouts, panel placement, focus state, layout persistence, and session restoration.

This subsystem is central to Finch because it defines the user's day-to-day interaction model.

## Goals

- Support named workspaces.
- Support multiple tabs within each workspace.
- Support horizontal and vertical split layouts.
- Persist layouts across sessions.
- Provide fast keyboard-first navigation.
- Keep panel placement separate from panel behavior.
- Allow plugins to contribute panels without directly modifying workspace internals.
- Support future layout features such as floating panels, layout templates, and project-aware workspaces.

## Non-Goals for MVP

- Graphical drag-and-drop layout editing.
- Overlapping window management.
- Cloud synchronization.
- Multi-machine workspace state.

The MVP should focus on predictable keyboard-driven layouts.

## Core Concepts

### Workspace

A workspace is a named environment containing tabs, layouts, panels, focus state, and metadata.

Example workspace names:

- `default`
- `dev`
- `ops`
- `kubernetes`
- `homelab`
- `incident-response`

### Tab

A tab is a top-level page inside a workspace. Each workspace may contain multiple tabs.

Example tabs:

- `Main`
- `Logs`
- `Git`
- `Cluster`
- `Monitoring`

### Split

A split divides available space horizontally or vertically. Splits may be nested.

### Panel

A panel is a renderable unit placed inside a layout container. The Workspace Manager tracks where panels live, but does not own panel behavior.

### Focus

Focus identifies which panel receives keyboard input. Focus is workspace-local and should be restored when switching tabs or workspaces.

## High-Level Architecture

```text
+--------------------------------------------------+
|                Workspace Manager                 |
+--------------------------------------------------+
| Workspace Registry | Layout Engine | Focus Graph  |
| Session Store      | Tab Manager    | Dock Manager |
+--------------------------------------------------+
        |                 |                  |
        v                 v                  v
  State Manager       Panel Manager       Event Bus
```

## Responsibilities

The Workspace Manager is responsible for:

- Creating, renaming, switching, and deleting workspaces.
- Creating, closing, moving, and renaming tabs.
- Managing split layouts.
- Assigning panels to layout containers.
- Tracking active workspace, active tab, and focused panel.
- Persisting workspace layouts.
- Restoring previous sessions.
- Emitting workspace lifecycle events.
- Validating layout trees.

The Workspace Manager is not responsible for:

- Rendering panel contents.
- Executing panel commands.
- Loading plugins.
- Managing global configuration.
- Running background tasks.

## Data Model

```rust
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub tabs: Vec<Tab>,
    pub active_tab: Option<TabId>,
    pub focus: FocusState,
    pub metadata: WorkspaceMetadata,
}

pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub root: LayoutNode,
    pub active_panel: Option<PanelId>,
}

pub enum LayoutNode {
    Panel(PanelPlacement),
    Split(SplitNode),
    Empty,
}

pub struct SplitNode {
    pub direction: SplitDirection,
    pub ratio: f32,
    pub first: Box<LayoutNode>,
    pub second: Box<LayoutNode>,
}

pub enum SplitDirection {
    Horizontal,
    Vertical,
}

pub struct PanelPlacement {
    pub panel_id: PanelId,
    pub preferred_size: Option<PanelSize>,
    pub min_size: Option<PanelSize>,
}
```

## Layout Tree

Layouts should be represented as trees. A tree model makes nested splits simple, serializable, and testable.

Example:

```text
Tab: Dev

Vertical Split
- Panel: File Browser
- Horizontal Split
  - Panel: Terminal
  - Panel: Git
```

Each layout node should be one of:

- Empty
- Panel
- Split

A split node should include:

- Direction
- Ratio
- First child
- Second child

## Layout Operations

Required operations:

- Open panel in current tab.
- Split focused panel horizontally.
- Split focused panel vertically.
- Close focused panel.
- Move focus left, right, up, or down.
- Move panel to another tab.
- Move panel to another workspace.
- Resize split.
- Balance splits.
- Save layout as a template.
- Restore layout from a template.

## Focus Management

Focus must be deterministic and keyboard-friendly.

Initial focus rules:

- New panels receive focus by default.
- Closing a focused panel moves focus to the nearest valid neighbor.
- Switching tabs restores the last focused panel for that tab.
- Switching workspaces restores the last focused tab and panel for that workspace.
- Directional focus movement should use resolved panel geometry.

## Docking Model

MVP docking should support fixed regions:

- Left dock
- Right dock
- Bottom dock
- Main area

```text
+--------------------------------------------------+
| Top Bar                                          |
+----------+-----------------------------+---------+
| Left     | Main Area                   | Right   |
| Dock     |                             | Dock    |
+----------+-----------------------------+---------+
| Bottom Dock                                      |
+--------------------------------------------------+
| Status Bar                                       |
+--------------------------------------------------+
```

## Persistence

Workspace layouts should be stored in human-readable files under:

`~/.config/finch/workspaces/`

Workspace files should include:

- Schema version
- Workspace name
- Workspace ID
- Tabs
- Layout tree
- Panel type identifiers
- Panel instance IDs
- Focus state
- Dock state

Example:

```toml
schema_version = 1
name = "dev"
active_tab = "main"

[[tabs]]
id = "main"
title = "Main"
```

## Serialization

Serialized layouts must include enough information to restore the user's visible workspace without embedding unnecessary runtime data.

Required serialized data:

- Workspace name
- Workspace ID
- Schema version
- Tabs
- Layout tree
- Panel type identifiers
- Panel instance IDs
- Focus state
- Dock state

Serialized layouts should avoid sensitive runtime data unless a panel explicitly opts in through its own persistence contract.

## Versioning

Workspace files should include a schema version so Finch can migrate older layouts when the format changes.

```toml
schema_version = 1
```

When the schema changes, Finch should provide migrations where practical.

## Events

The Workspace Manager should publish events such as:

- WorkspaceCreated
- WorkspaceDeleted
- WorkspaceRenamed
- WorkspaceSwitched
- TabCreated
- TabClosed
- TabSwitched
- LayoutChanged
- PanelFocused
- WorkspaceSaved
- WorkspaceRestored

The Workspace Manager should subscribe to:

- Command execution events
- Panel lifecycle events
- Configuration reload events
- Terminal resize events

## Plugin Integration

Plugins may request workspace actions through approved commands or SDK methods.

Examples:

- Open plugin panel in current workspace.
- Open plugin panel in a new tab.
- Request focus for a plugin panel.
- Save plugin-specific panel state.

Plugins should not mutate workspace state directly.

## Error Handling

Workspace operations should return structured errors.

Examples:

- Workspace not found
- Tab not found
- Panel not found
- Invalid layout tree
- Serialization failed
- Persistence failed
- Schema migration failed

A failed workspace operation should not corrupt the current layout.

## Performance Considerations

- Layout calculation should be fast and deterministic.
- Rendering should use cached layout geometry when possible.
- Repeated resize events should be coalesced.
- Workspace persistence should avoid blocking the UI thread.
- Large workspace files should be loaded asynchronously.

## MVP Requirements

The first implementation should support:

- One default workspace.
- Multiple tabs.
- Horizontal and vertical splits.
- Opening and closing panels.
- Keyboard focus movement.
- Basic layout persistence.
- Session restoration.
- Workspace events.

## Future Enhancements

- Multiple named workspaces.
- Layout templates.
- Floating panels.
- Workspace profiles.
- Project-aware workspaces.
- Remote SSH workspaces.
- Visual layout editor.
- Workspace import and export.

## Open Questions

- Should layouts be stored as TOML, JSON, or RON?
- Should workspaces be global or project-scoped by default?
- Should panel instance state live inside workspace files or separate panel state files?
- How much layout control should plugins receive?
- Should Finch support attachable sessions in the future?
