# Finch Architecture

## Overview

Finch is a modular, terminal-native workspace platform built in Rust using Ratatui for rendering. The architecture is designed around loosely coupled services that communicate through a central event bus.

```
+------------------------------------------------+
|                  Finch Core                    |
+------------------------------------------------+
| Command Palette | Workspace | Plugin Manager   |
| Notification    | Config    | Task Scheduler   |
+------------------------------------------------+
| Panel Manager | Event Bus | State Manager      |
+------------------------------------------------+
| Ratatui Rendering Engine                        |
+------------------------------------------------+
| Crossterm / Terminal                            |
+------------------------------------------------+
```

## Core Components

### Rendering Engine
- Ratatui-based rendering
- Incremental redraws
- Theme support
- Responsive layouts

### Event Bus
- Central publish/subscribe messaging
- Decouples components
- Supports plugins and internal services

### State Manager
- Maintains global application state
- Workspace persistence
- User preferences
- Session restoration

### Workspace Manager
- Multiple workspaces
- Dockable panels
- Saved layouts
- Tabs and splits

### Panel Manager
Every visible area is a panel.
Examples include:
- Terminal
- File Browser
- Git
- Kubernetes
- Logs
- Metrics
- Tasks
- AI Chat
- Notifications

### Command Palette
Provides keyboard-driven access to all actions, similar to VS Code's Command Palette.

### Plugin Manager
Loads first-party and third-party plugins with isolated lifecycles and a stable API.

### Configuration System
Uses TOML configuration files for application settings, keybindings, themes, plugins, and workspaces.

## Design Principles
- Modular by default
- Event-driven communication
- Non-blocking asynchronous operations
- Keyboard-first interaction
- Extensible through plugins
- Cross-platform support
- Minimal dependencies

## Future Architecture
Additional subsystems planned include:
- Integrated AI assistant
- Remote SSH workspaces
- Cloud provider integrations
- Kubernetes dashboard
- Git client
- Package manager
- Embedded scripting
- Extension marketplace