# Finch Core Principles

## Project Mantra

**Finch is a terminal workspace platform, not just a terminal application.**

This mantra defines the project. Finch should not become a collection of unrelated terminal widgets. It should provide a cohesive, extensible environment where panels, commands, plugins, workspaces, and services behave as parts of one platform.

## Documentation as Source of Truth

Finch documentation is part of the design, not an afterthought.

Architecture documents define how Finch should work. Code should implement the documented architecture. When the design changes, the documentation should change with it.

## Non-Negotiable Principles

### 1. Keyboard First

Every core workflow must be fully usable from the keyboard. Mouse support may exist, but the mouse must never be required.

### 2. Terminal Native

Finch should feel like it belongs in the terminal. It should embrace terminal constraints instead of fighting them.

### 3. Lightweight by Default

Finch must start quickly, use modest memory, and avoid unnecessary background work.

### 4. Modular by Design

Core features should be implemented as clear subsystems with well-defined boundaries.

### 5. Event-Driven Communication

Core services, panels, and plugins should communicate through the event bus instead of direct coupling.

### 6. Panels Are Isolated Units

Panels should not directly control other panels. Coordination happens through commands, state, and events.

### 7. Plugins Use Explicit Capabilities

Plugins must declare what they need. Access to events, files, shell commands, network resources, and system integrations should be capability-based.

### 8. Configuration Is Human-Readable

Configuration should use text-based files that are easy to read, edit, version, and share.

### 9. Responsive Under Load

The UI must remain responsive even when background tasks, plugins, or integrations are busy.

### 10. Sensible Defaults, Deep Customization

Finch should work well out of the box while allowing advanced users to deeply customize layouts, keybindings, themes, panels, and plugins.

### 11. Cross-Platform Where Practical

Finch should support Linux, macOS, and Windows where feasible, while still respecting platform differences.

### 12. Open Architecture

Finch should be understandable, extensible, and contributor-friendly. Public APIs and internal design decisions should be documented.

## Design Rules

- Prefer explicit interfaces over implicit global behavior.
- Prefer composition over inheritance-like patterns.
- Prefer small, testable crates over one large crate.
- Prefer async, non-blocking workflows for IO-heavy work.
- Prefer documented exceptions over hidden special cases.
- Prefer stable internal APIs before adding feature complexity.

## Decision Filter

Before adding a major feature, ask:

1. Does it fit the terminal workspace platform model?
2. Can it be used from the keyboard?
3. Does it preserve UI responsiveness?
4. Does it integrate through documented subsystems?
5. Can it be configured or disabled?
6. Does it belong in core, or should it be a plugin?

If the answer is unclear, document the decision in an ADR before implementation.
