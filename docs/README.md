# Finch Documentation

Finch is a lightweight, keyboard-first terminal workspace platform written in Rust and built on Ratatui.

**Project mantra:** Finch is a terminal workspace platform, not just a terminal application.

Documentation is the source of truth. Architecture and specifications are written before code; implementation follows the docs.

## Foundation

| Document | Description |
|---|---|
| [Vision](VISION.md) | Mission, target audience, and long-term goals |
| [Architecture Overview](ARCHITECTURE.md) | High-level system design |
| [Core Principles](CORE_PRINCIPLES.md) | Non-negotiable design rules and decision filter |

## Architecture

| Document | Subsystem |
|---|---|
| [Event Bus](architecture/event-bus.md) | Central pub/sub messaging |
| [State Management](architecture/state-management.md) | State ownership and thread safety |
| [Workspace Manager](architecture/workspace-manager.md) | Workspaces, tabs, splits |
| [Panel API](architecture/panel-api.md) | Renderable panel contract |
| [Rendering Pipeline](architecture/rendering-pipeline.md) | Frame composition and scheduling |
| [Plugin SDK](architecture/plugin-sdk.md) | Plugin host, WASM sandbox, developer SDK |
| [Command System](architecture/command-system.md) | Command palette and keybinding dispatch |
| [Configuration System](architecture/configuration-system.md) | TOML config, validation, hot-reload |
| [Theme Engine](architecture/theme-engine.md) | Color schemes and theming API |
| [Notification System](architecture/notification-system.md) | In-app and OS notifications |
| [Scheduler](architecture/scheduler.md) | Recurring and deferred background tasks |
| [Task System](architecture/task-system.md) | Long-running operation tracking |
| [AI Integration](architecture/ai-integration.md) | AI chat panel and provider abstraction |
| [Security & Permissions](architecture/security-permissions.md) | Capability model and sandboxing |

## Specifications

| Document | Description |
|---|---|
| [State Management Spec](specs/state-management.md) | State model, actions, reducers |

## Design Records

| Directory | Description |
|---|---|
| [ADRs](decisions/README.md) | Architecture Decision Records |
| [RFCs](rfcs/README.md) | Design proposals for major features |

## Developer Guides

| Document | Description |
|---|---|
| [Plugin Developer Guide](plugins/README.md) | Build and publish Finch plugins |
| [Contributing](developer/contributing.md) | PR workflow and decision process |
| [Coding Standards](developer/coding-standards.md) | Rust style and conventions |
| [Development Setup](developer/development-setup.md) | Build environment |
| [API Reference](api/README.md) | Plugin API and internal APIs |

## Releases

| Document | Description |
|---|---|
| [Release Process](releases/README.md) | Versioning and release steps |
| [Changelog](../CHANGELOG.md) | Release history |
