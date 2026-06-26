# Finch

**A lightweight, keyboard-first terminal workspace platform.**

Finch is a modular terminal environment built in Rust with Ratatui. It brings workspaces, panels, a command palette, plugins, and background services together into a single, fast, keyboard-driven interface — without leaving the terminal.

> Small enough to fly anywhere. Powerful enough to run your day.

---

## Table of Contents

- [Vision](#vision)
- [Documentation](#documentation)
  - [Architecture](#architecture)
  - [Specifications](#specifications)
- [Project Status](#project-status)

---

## Vision

Finch is designed for developers, DevOps engineers, SREs, platform engineers, network engineers, security professionals, and homelab users who want a cohesive terminal workspace instead of a collection of disconnected tools.

See the full vision: [docs/VISION.md](docs/VISION.md)

---

## Documentation

### Core Docs

| Document | Description |
|---|---|
| [Vision](docs/VISION.md) | Mission, target audience, and long-term goals |
| [Architecture](docs/ARCHITECTURE.md) | High-level system design and component overview |
| [Core Principles](docs/CORE_PRINCIPLES.md) | Non-negotiable design rules and decision filter |

### Architecture

Detailed specifications for each core subsystem:

| Document | Description |
|---|---|
| [Event Bus](docs/architecture/event-bus.md) | Internal publish/subscribe messaging backbone |
| [Workspace Manager](docs/architecture/workspace-manager.md) | Workspaces, tabs, splits, focus, and layout persistence |
| [Panel API](docs/architecture/panel-api.md) | Contract for all renderable panel units |
| [Rendering Pipeline](docs/architecture/rendering-pipeline.md) | Frame composition, layout resolution, and redraw scheduling |
| [State Management](docs/architecture/state-management.md) | State ownership, transitions, persistence, and thread safety |

### Specifications

In-depth design specifications:

| Document | Description |
|---|---|
| [State Management Spec](docs/specs/state-management.md) | Detailed state model, actions, effects, reducers, and milestones |

---

## Project Status

Finch is in early planning and design. No source code exists yet.

Current focus is establishing a solid architectural foundation before implementation begins. Documentation is the source of truth — code will implement what the docs describe.
