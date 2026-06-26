# Finch

**A lightweight, keyboard-first terminal workspace platform.**

Finch is a modular terminal environment built in Rust with Ratatui. It brings workspaces, panels, a command palette, plugins, and background services together into a single, fast, keyboard-driven interface — without leaving the terminal.

> Small enough to fly anywhere. Powerful enough to run your day.

**Project mantra:** Finch is a terminal workspace platform, not just a terminal application.

---

## Table of Contents

- [Vision](#vision)
- [Task List](TASKS.md)
- [Project Roadmap](#project-roadmap)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [Project Status](#project-status)

---

## Vision

Finch is designed for developers, DevOps engineers, SREs, platform engineers, network engineers, security professionals, and homelab users who want a cohesive terminal workspace instead of a collection of disconnected tools.

See the full vision: [docs/VISION.md](docs/VISION.md)

---

## Project Roadmap

### Phase 0 — Architecture

Finish all core subsystem specifications before writing significant code. Documentation is the source of truth.

| Subsystem | Status |
|---|---|
| Vision | Done |
| Core Principles | Done |
| Overall Architecture | Done |
| Event Bus | Done |
| State Management | Done |
| Workspace Manager | Done |
| Panel API | Done |
| Rendering Pipeline | Done |
| Plugin SDK | In Progress |
| Command System | In Progress |
| Configuration System | In Progress |
| Theme Engine | In Progress |
| Notification System | In Progress |
| Scheduler | In Progress |
| Task System | In Progress |
| AI Integration | In Progress |
| Security & Permissions | In Progress |

### Phase 1 — Foundation

Bootstrap the Rust workspace with clean crate boundaries:

```
finch/
├── finch-core        # core runtime, event bus, state manager
├── finch-ui          # UI primitives and layout engine
├── finch-events      # event bus implementation
├── finch-panels      # panel framework and built-in panels
├── finch-workspace   # workspace, tabs, splits, docking
├── finch-config      # configuration loading and validation
├── finch-plugins     # plugin host, WASM sandbox, SDK
├── finch-renderer    # Ratatui rendering pipeline
├── finch-cli         # binary entry point
└── finch-app         # top-level application assembly
```

### Phase 2 — MVP

First usable version:

- Full-screen Ratatui UI with status bar
- Command palette
- Multiple panels with docking
- Workspace manager
- Configuration and theme support
- Plugin loading
- Notifications

### Phase 3 — Ecosystem

First-party panels: Terminal, File Browser, Git, Process Monitor, Logs, Docker, Kubernetes, SSH, Tasks, Notes, AI Chat, RSS, Calendar, System Metrics, Network Monitor.

### Phase 4 — Platform

Plugin registry, `finch install <plugin>` package manager, first-party marketplace.

---

## Documentation

### Foundation

| Document | Description |
|---|---|
| [Vision](docs/VISION.md) | Mission, target audience, and long-term goals |
| [Architecture](docs/ARCHITECTURE.md) | High-level system design and component overview |
| [Core Principles](docs/CORE_PRINCIPLES.md) | Non-negotiable design rules and decision filter |

### Architecture

| Document | Subsystem |
|---|---|
| [Event Bus](docs/architecture/event-bus.md) | Internal publish/subscribe messaging |
| [State Management](docs/architecture/state-management.md) | State ownership and thread safety |
| [Workspace Manager](docs/architecture/workspace-manager.md) | Workspaces, tabs, splits, focus |
| [Panel API](docs/architecture/panel-api.md) | Renderable panel contract |
| [Rendering Pipeline](docs/architecture/rendering-pipeline.md) | Frame composition and redraw scheduling |
| [Plugin SDK](docs/architecture/plugin-sdk.md) | Plugin host, WASM sandbox, developer SDK |
| [Command System](docs/architecture/command-system.md) | Command palette and keybinding dispatch |
| [Configuration System](docs/architecture/configuration-system.md) | TOML config loading, validation, hot-reload |
| [Theme Engine](docs/architecture/theme-engine.md) | Color schemes, styles, and theming API |
| [Notification System](docs/architecture/notification-system.md) | In-app and OS notifications |
| [Scheduler](docs/architecture/scheduler.md) | Background task scheduling and timers |
| [Task System](docs/architecture/task-system.md) | Long-running task tracking and progress |
| [AI Integration](docs/architecture/ai-integration.md) | AI assistant panel and inference layer |
| [Security & Permissions](docs/architecture/security-permissions.md) | Capability model and plugin sandboxing |

### Specifications

| Document | Description |
|---|---|
| [State Management Spec](docs/specs/state-management.md) | State model, actions, reducers, milestones |
| [Event Bus Spec](docs/specs/event-bus.md) | Event catalog, subscription model, delivery guarantees |
| [Panel API Spec](docs/specs/panel-api.md) | Panel trait, lifecycle, PanelAction, PanelContext |
| [Workspace Persistence Spec](docs/specs/workspace-persistence.md) | Workspace file schema, validation, migration |

### Design Records

| Directory | Description |
|---|---|
| [ADRs](docs/decisions/README.md) | Architecture Decision Records |
| [RFCs](docs/rfcs/README.md) | Design proposals for major features |

### Panel Reference

| Document | Description |
|---|---|
| [Panel Index](docs/panels/README.md) | All first-party panels by phase |
| [Terminal](docs/architecture/terminal-panel.md) | PTY terminal emulator (Phase 1) |
| [File Browser](docs/panels/file-browser.md) | Filesystem navigator (Phase 1) |
| [Git](docs/panels/git.md) | Git status, staging, commit (Phase 1) |
| [Log Viewer](docs/panels/log-viewer.md) | Tail and search log files (Phase 1) |
| [AI Chat](docs/panels/ai-chat.md) | AI assistant with context injection (Phase 1) |

### User Guide

| Document | Description |
|---|---|
| [User Guide](docs/user-guide/README.md) | Installation, navigation, workspaces, panels, plugins |
| [Installation](docs/user-guide/installation.md) | Binary install, build from source, system requirements |
| [Keybinding Reference](docs/user-guide/keybindings.md) | All default keybindings by context |
| [Configuration Reference](docs/user-guide/configuration.md) | All `config.toml` keys, types, and defaults |

### Developer Guides

| Document | Description |
|---|---|
| [Plugin Developer Guide](docs/plugins/README.md) | Build and publish Finch plugins |
| [Contributing Guide](docs/developer/contributing.md) | PR workflow, commit conventions |
| [Coding Standards](docs/developer/coding-standards.md) | Rust style and conventions |
| [Development Setup](docs/developer/development-setup.md) | Build environment setup |
| [Testing Strategy](docs/developer/testing-strategy.md) | Unit, integration, snapshot, WASM tests, CI matrix |
| [CI/CD Pipeline](docs/developer/ci-cd.md) | Automated checks, release process, artifact publishing |
| [API Reference](docs/api/README.md) | Plugin API and internal APIs |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the short version, or the full [Contributing Guide](docs/developer/contributing.md).

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) and [Security Policy](SECURITY.md).

---

## Project Status

Finch is in **Phase 0: Architecture**. No source code exists yet.

Documentation is the source of truth — code will implement what the docs describe. Current work is completing the subsystem architecture docs listed in the Phase 0 table above.
