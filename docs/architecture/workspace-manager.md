# Workspace Manager Specification

## Purpose

The Workspace Manager controls how users organize work inside Finch. It manages workspaces, tabs, split layouts, panel placement, focus state, layout persistence, and session restoration.

## Goals

- Support named workspaces.
- Support multiple tabs within each workspace.
- Support horizontal and vertical split layouts.
- Persist layouts across sessions.
- Provide fast keyboard-first navigation.
- Keep panel placement separate from panel behavior.
- Allow plugins to contribute panels without directly modifying workspace internals.

## Core Concepts

### Workspace

A workspace is a named environment containing tabs, layouts, panels, focus state, and metadata.

### Tab

A tab is a top-level page inside a workspace. Each workspace may contain multiple tabs.

### Split

A split divides available space horizontally or vertically. Splits may be nested.

### Panel

A panel is a renderable unit placed inside a layout container. The Workspace Manager tracks where panels live, but does not own panel behavior.

### Focus

Focus identifies which panel receives keyboard input. Focus is workspace-local and should be restored when switching tabs or workspaces.

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

## Layout Model

Layouts should be represented as trees. A tree model makes nested splits simple, serializable, and testable.

Each layout node should be one of:

- Empty
- Panel
- Split

A split node should include:

- Direction
- Ratio
- First child
- Second child

## Required Operations

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

Plugins may request workspace actions through approved commands or SDK methods. Plugins should not mutate workspace state directly.

## Error Handling

Workspace operations should return structured errors. A failed workspace operation should not corrupt the current layout.

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
- Visual layout editor.
- Workspace import and export.

## Open Questions

- Should layouts be stored as TOML, JSON, or RON?
- Should workspaces be global or project-scoped by default?
- Should panel instance state live inside workspace files or separate panel state files?
- How much layout control should plugins receive?
